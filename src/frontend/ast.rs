#![allow(dead_code)]
#![allow(unused_must_use)]
#![allow(bad_style)]

use crate::frontend::ast::Stmt::Use;
use crate::frontend::tokens::TokenTyp::{Directive, Identifier, ParenOpen, Wild};
use crate::frontend::tokens::{BinOp, DirectiveTyp, Flg, Keyword, Token, TokenTyp};
use std::marker::PhantomData;

pub struct TokStream {
    tokens: Vec<Option<Token>>,
    cursor: usize,
}

impl TokStream {
    pub fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).and_then(|tok| tok.as_ref())
    }

    pub fn next(&mut self) -> Option<Token> {
        if self.cursor < self.tokens.len() {
            let tok = self.tokens[self.cursor].take();
            self.cursor += 1;
            tok
        } else {
            None
        }
    }
}

pub struct Parser<'a> {
    tokstream: TokStream,
    _marker: PhantomData<&'a ()>,
    ignore_semi_c: bool,

    flag_repr_partition: usize,
    flag_repr_cap: usize,
    typ_repr_partition: u8,
    parsing_flag: bool,
    typ_repr_partition_2: u8,
}

#[derive(Debug)]
pub enum Stmt<'a> {
    Asg {
        target: usize,
        value: Box<Expr<'a>>,
    },
    Flag {
        flag: Flg,
        payload: Option<Box<Expr<'a>>>,
    },
    ExprStmt(Box<Expr<'a>>),

    // All "Compiler Directives"
    Use(Vec<Box<str>>),
    Import(Vec<Box<str>>),
    From {
        path: Box<str>,
        pattern: Box<str>,
    },
    CompilerDirective {
        directive: CompilerDirective,
        flags: Vec<DirectiveFlgOpts>,
        block: Vec<Stmt<'a>>,
    },

    Program(Vec<Box<Stmt<'a>>>),
    _Marker(PhantomData<&'a ()>),
}

#[derive(Debug)]
pub enum Expr<'a> {
    Keyword(Keyword),
    TypeExpr(TypExpr<'a>),
    String(Box<str>),
    Integer(u128),
    Float(f64),
    Var(usize),
    Reg(usize),
    Unary {
        op: UnaryOp,
        target: Box<Expr<'a>>,
    },
    Bin {
        op: BinOp,
        lhs: Box<Expr<'a>>,
        rhs: Box<Expr<'a>>,
    },
    Call {
        callee: Box<Expr<'a>>,
        args: Vec<Box<Expr<'a>>>,
    },
    Index {
        base: Box<Expr<'a>>,
        index: Box<Expr<'a>>,
    },
    Chain {
        head: Box<Expr<'a>>,
        tail: Box<Expr<'a>>,
    },
    Field {
        parent: Box<Expr<'a>>,
        field: Box<Expr<'a>>,
    },
    Array(Vec<Box<Expr<'a>>>),
    Decl {
        identifier: Box<PatternExpr<'a>>,
        value: Box<Expr<'a>>,
        flags: Vec<Flg>,
        flag_payload: Vec<Box<Expr<'a>>>,
    },
    If {
        cond: Box<Expr<'a>>,
        then_block: Vec<Box<Stmt<'a>>>,
        else_block: Option<Box<Stmt<'a>>>,
    },
    Match {
        expr: Box<Expr<'a>>,
        arms: Vec<MatchArm<'a>>,
    },
    Scope(Vec<Box<Stmt<'a>>>),

    StaticType_payload_params(Vec<Box<Expr<'a>>>),
    Struct(Vec<Field<'a>>),
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Copy)]
pub enum StaticTyp {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Usize,
    Isize,

    Str,
    Enum,
    Scope,
    Obj,
    Struct,
    ENTRY,
    INIT,
    Bool,

    // payload
    // single
    Func, // -> 22
    Trait,
    Vector,

    //variable-length
    Tuple, // -> 25

    // special
    Array,
    // User Defined: post-ast logic
}

#[derive(Debug)]
pub struct TypExpr<'a> {
    pub typ: StaticTyp,
    pub payload: Option<Box<Expr<'a>>>,
}

#[derive(Debug)]
pub enum UnaryOp {
    Minus,
    Plus,
    Bang,
    Deref,
    Ptr,
    ComplexType,
}

// Block Payloads
#[derive(Debug)]
pub enum CompilerDirective {
    // Use,
    // From,
    // Import,
    TypeCast,
    Defer,
}

#[derive(Debug)]
pub enum DirectiveFlgOpts {
    Collapse,
    StaticType(StaticTyp),
}

#[derive(Debug)]
pub enum PatternExpr<'a> {
    Wildcard,
    Tuple(Vec<Box<PatternExpr<'a>>>),
    EnumVariant {
        variant: Box<Expr<'a>>,
        pattern: Box<PatternExpr<'a>>,
    },
    Expr(Box<Expr<'a>>),
}

#[derive(Debug)]
pub struct MatchArm<'a> {
    pattern: Box<PatternExpr<'a>>,
    guard: Option<Box<Expr<'a>>>,
    body: Box<Expr<'a>>,
}

#[derive(Debug)]
pub struct Field<'a> {
    field: usize,
    value: Box<Expr<'a>>,
}

impl<'a> From<TokenTyp> for UnaryOp {
    fn from(value: TokenTyp) -> Self {
        match value {
            TokenTyp::BinOp(BinOp::Minus) => UnaryOp::Minus,
            TokenTyp::BinOp(BinOp::Plus) => UnaryOp::Plus,
            TokenTyp::Andp => UnaryOp::Deref,
            TokenTyp::BinOp(BinOp::Mult) => UnaryOp::Ptr,
            _ => unreachable!(),
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Option<Token>>) -> Parser<'a> {
        Parser {
            tokstream: TokStream { tokens, cursor: 0 },
            _marker: PhantomData,
            ignore_semi_c: false,
            flag_repr_partition: 1,   // No arg flag
            flag_repr_cap: 3,         // Arg Flag
            typ_repr_partition: 22,   // for types with single arg payload
            typ_repr_partition_2: 25, // for types with payloads of variable length
            parsing_flag: false,
        }
    }
    pub fn from_ast(&mut self) -> Stmt<'a> {
        let mut ast = vec![];
        while let Some(_) = self.tokstream.peek() {
            ast.push(self.parse_stmt().unwrap());
        }
        Stmt::Program(ast)
    }
    fn expect(
        &mut self,
        expected: TokenTyp,
        panic_handler: fn() -> Result<(), ()>,
    ) -> Result<(), ()> {
        match self.tokstream.peek() {
            Some(tok) if tok.typ == expected => {
                self.tokstream.next();
                Ok(())
            }
            _ => panic_handler(),
        }
    }
    fn parse_stmt(&mut self) -> Result<Box<Stmt<'a>>, ()> {
        match self.tokstream.peek() {
            Some(Token {
                typ: TokenTyp::Identifier(x),
                ..
            }) => {
                let x = x.to_owned();
                let ident = self.parse_prim_expr().unwrap();
                match self.tokstream.peek() {
                    Some(Token {
                        typ: TokenTyp::RArrow,
                        ..
                    }) => {
                        self.tokstream.next();
                        let value = self
                            .parse_expr(0, None, false)
                            .unwrap_or_else(|_| panic!("Explicit"));
                        self.expect(TokenTyp::Semicolon, || panic!(""));
                        Ok(Box::new(Stmt::Asg { target: x, value }))
                    }
                    Some(Token {
                        typ: TokenTyp::RArrowSquig,
                        ..
                    }) => {
                        self.tokstream.next();
                        let value = Box::new(Expr::Unary {
                            op: UnaryOp::Ptr,
                            target: self
                                .parse_expr(0, None, false)
                                .unwrap_or_else(|_| panic!("Explicit")),
                        });
                        self.expect(TokenTyp::Semicolon, || panic!(""));
                        Ok(Box::new(Stmt::Asg { target: x, value }))
                    }
                    _ => Ok(Box::new(Stmt::ExprStmt(ident))),
                }
            }
            Some(Token {
                typ: TokenTyp::BinOp(BinOp::Lt),
                ..
            }) => {
                self.parsing_flag = true;
                self.tokstream.next();
                match self.tokstream.peek() {
                    Some(Token {
                        typ: TokenTyp::Identifier(n),
                        ..
                    }) if n <= &self.flag_repr_partition => {
                        let flag = unsafe { std::mem::transmute::<u8, Flg>(n.to_owned() as u8) };
                        self.tokstream.next();
                        self.expect(TokenTyp::BinOp(BinOp::Gt), || panic!("Explicit"));
                        self.parsing_flag = false;
                        Ok(Box::new(Stmt::Flag {
                            flag,
                            payload: None,
                        }))
                    }
                    Some(Token {
                        typ: TokenTyp::Identifier(n),
                        ..
                    }) if n <= &self.flag_repr_cap => {
                        let flag = unsafe { std::mem::transmute::<u8, Flg>(n.to_owned() as u8) };
                        self.tokstream.next();
                        self.expect(TokenTyp::Colon, || panic!("Explicit"));
                        let payload = Some(
                            self.parse_expr(0, None, false)
                                .unwrap_or_else(|_| panic!("Explicit")),
                        );

                        self.expect(TokenTyp::BinOp(BinOp::Gt), || panic!("Explicit"));

                        self.parsing_flag = false;
                        Ok(Box::new(Stmt::Flag {
                            flag,
                            payload: payload,
                        }))
                    }

                    _ => {
                        let payload = Some(
                            self.parse_expr(0, None, false)
                                .unwrap_or_else(|_| panic!("Explicit")),
                        );
                        self.expect(TokenTyp::BinOp(BinOp::Gt), || panic!("Explicit"));

                        self.parsing_flag = false;
                        Ok(Box::new(Stmt::Flag {
                            flag: Flg::Type,
                            payload,
                        }))
                    }
                }
            }
            Some(Token {
                typ: TokenTyp::Directive(DirectiveTyp::Use),
                ..
            }) => {
                self.tokstream.next();
                let mut out = vec![];
                loop {
                    match self
                        .tokstream
                        .next()
                        .unwrap_or(Token {
                            typ: TokenTyp::MetaString("".into()),
                            loc: (0, (0, 0)),
                        })
                        .typ
                    {
                        TokenTyp::MetaString(x) => out.push(x),
                        _ => break,
                    }
                }
                self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                Ok(Box::new(Use(out)))
            }
            Some(Token {
                typ: TokenTyp::Directive(DirectiveTyp::Import),
                ..
            }) => {
                self.tokstream.next();
                let mut out = vec![];
                loop {
                    match self
                        .tokstream
                        .next()
                        .unwrap_or(Token {
                            typ: TokenTyp::String("".into()),
                            loc: (0, (0, 0)),
                        })
                        .typ
                    {
                        TokenTyp::String(x) => out.push(x),
                        _ => break,
                    }
                }
                self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                Ok(Box::new(Stmt::Import(out)))
            }
            _ => {
                let stmt = Box::new(Stmt::ExprStmt(
                    self.parse_expr(0, None, false)
                        .unwrap_or_else(|()| panic!("Explicit")),
                ));
                if !self.ignore_semi_c {
                    self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                } else {
                    self.ignore_semi_c = false;
                }
                Ok(stmt)
            }
        }
    }
    fn parse_expr(
        &mut self,
        min_bp: u8,
        lhs_: Option<Box<Expr<'a>>>,
        ignore_postfix_braces: bool,
    ) -> Result<Box<Expr<'a>>, ()> {
        let mut lhs = if lhs_.is_none() {
            match self.tokstream.peek() {
                Some(Token {
                    typ: TokenTyp::Keyword(Keyword::Match),
                    ..
                }) => {
                    self.tokstream.next();
                    let target = self.parse_expr(0, None, false)?;
                    let mut v = vec![];
                    self.expect(TokenTyp::CurlyOpen, || panic!("Explicit"));
                    loop {
                        match self.tokstream.peek().unwrap_or_else(|| panic!("Explicit")) {
                            Token {
                                typ: TokenTyp::CurlyClose,
                                ..
                            } => {
                                self.tokstream.next();
                                break;
                            }
                            _ => {
                                let pattern = self.parse_pattern_expr()?;
                                let guard = if let Some(x) = self.tokstream.peek() {
                                    if x.typ == TokenTyp::Keyword(Keyword::If) {
                                        self.tokstream.next();
                                        Some(self.parse_expr(0, None, false)?)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                self.expect(TokenTyp::FatRArrow, || panic!("Explicit"));
                                let body = self.parse_expr(0, None, false)?;
                                self.expect(TokenTyp::Comma, || panic!("Explicit"));
                                v.push(MatchArm {
                                    body,
                                    guard,
                                    pattern,
                                });
                            }
                        }
                    }
                    self.ignore_semi_c = true;
                    Box::new(Expr::Match {
                        expr: target,
                        arms: v,
                    })
                }
                Some(Token {
                    typ: TokenTyp::Keyword(Keyword::Let),
                    ..
                }) => {
                    self.tokstream.next();
                    self.parsing_flag = true;
                    let target = self
                        .parse_pattern_expr()
                        .unwrap_or_else(|_| panic!("Explicit"));
                    let mut flags = vec![];
                    let mut flags_pl = vec![];
                    loop {
                        match self.tokstream.peek() {
                            Some(Token {
                                typ: TokenTyp::BinOp(BinOp::Lt),
                                ..
                            }) => {
                                self.tokstream.next();
                                match self.tokstream.peek() {
                                    Some(Token {
                                        typ: TokenTyp::Identifier(n),
                                        ..
                                    }) if n <= &self.flag_repr_partition => {
                                        dbg!(n);
                                        dbg!(self.flag_repr_partition);
                                        flags.push(unsafe {
                                            std::mem::transmute::<u8, Flg>(n.to_owned() as u8)
                                        });
                                        self.tokstream.next();
                                        self.expect(TokenTyp::BinOp(BinOp::Gt), || {
                                            panic!("Explicit")
                                        });
                                    }
                                    Some(Token {
                                        typ: TokenTyp::Identifier(n),
                                        ..
                                    }) if n <= &self.flag_repr_cap => {
                                        flags.push(unsafe {
                                            std::mem::transmute::<u8, Flg>(n.to_owned() as u8)
                                        });
                                        self.tokstream.next();
                                        self.expect(TokenTyp::Colon, || panic!("Explicit"));
                                        flags_pl.push(
                                            self.parse_expr(0, None, false)
                                                .unwrap_or_else(|_| panic!("Explicit")),
                                        );

                                        self.expect(TokenTyp::BinOp(BinOp::Gt), || {
                                            panic!("Explicit")
                                        });
                                    }

                                    _ => {
                                        flags.push(Flg::Type);
                                        flags_pl.push(
                                            self.parse_expr(0, None, false)
                                                .unwrap_or_else(|_| panic!("Explicit")),
                                        );
                                        self.expect(TokenTyp::BinOp(BinOp::Gt), || {
                                            panic!("Explicit")
                                        });
                                    }
                                }
                            }

                            _ => {
                                break;
                            }
                        }
                    }

                    self.parsing_flag = false;
                    let value = self
                        .parse_expr(0, None, false)
                        .unwrap_or_else(|_| panic!("Explicit"));
                    Box::new(Expr::Decl {
                        identifier: target,
                        value,
                        flags,
                        flag_payload: flags_pl,
                    })
                }
                Some(Token {
                    typ: TokenTyp::BracketOpen,
                    ..
                }) => {
                    self.tokstream.next();
                    let mut els = vec![];
                    loop {
                        match self.tokstream.peek() {
                            Some(Token {
                                typ: TokenTyp::BracketClose,
                                ..
                            }) => {
                                self.tokstream.next();
                                break;
                            }
                            _ => {
                                els.push(
                                    self.parse_expr(0, None, false)
                                        .unwrap_or_else(|_| panic!("Explicit")),
                                );
                                match self.tokstream.peek() {
                                    Some(
                                        Token {
                                            typ: TokenTyp::Semicolon,
                                            ..
                                        },
                                        ..,
                                    ) => {
                                        self.tokstream.next();
                                    }
                                    Some(
                                        Token {
                                            typ: TokenTyp::BracketClose,
                                            ..
                                        },
                                        ..,
                                    ) => {
                                        self.tokstream.next();
                                        break;
                                    }
                                    _ => panic!("Explicit"),
                                }
                            }
                        }
                    }
                    Box::new(Expr::Array(els))
                }
                Some(Token {
                    typ: TokenTyp::CurlyOpen,
                    ..
                }) => match self.peek_3() {
                    Some(Token {
                        typ: TokenTyp::Colon,
                        ..
                    }) => {
                        self.tokstream.next();
                        let mut fields = Vec::new();

                        while !matches!(
                            self.tokstream.peek(),
                            Some(Token {
                                typ: TokenTyp::CurlyClose,
                                ..
                            })
                        ) {
                            fields.push(Field {
                                field: {
                                    match self
                                        .tokstream
                                        .next()
                                        .unwrap_or_else(|| panic!("Explicit"))
                                    {
                                        Token {
                                            typ: Identifier(n), ..
                                        } => n,
                                        _ => panic!("Explicit"),
                                    }
                                },
                                value: {
                                    self.expect(TokenTyp::Colon, || panic!("Explicit"));
                                    self.parse_expr(0, None, false)?
                                },
                            });
                            self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                        }

                        self.tokstream.next();

                        Box::new(Expr::Struct(fields))
                    }
                    _ => Box::new(Expr::Scope(
                        self.parse_block().unwrap_or_else(|_| panic!("Explicit")),
                    )),
                },
                Some(Token {
                    typ: TokenTyp::Keyword(Keyword::If),
                    ..
                }) => {
                    self.tokstream.next();
                    let cond = self
                        .parse_expr(0, None, false)
                        .unwrap_or_else(|_| panic!("Explicit"));
                    let then_block = self.parse_block().unwrap_or_else(|_| panic!("Explicit"));

                    let else_block = if matches!(
                        self.tokstream.peek(),
                        Some(Token {
                            typ: TokenTyp::Keyword(Keyword::Else),
                            ..
                        })
                    ) {
                        self.tokstream.next();
                        Some(self.parse_stmt().unwrap_or_else(|_| panic!("Explicit")))
                    } else {
                        None
                    };

                    self.ignore_semi_c = true;
                    Box::new(Expr::If {
                        cond,
                        then_block,
                        else_block,
                    })
                }
                Some(Token {
                    typ: TokenTyp::ParenOpen,
                    ..
                }) => {
                    self.tokstream.next();
                    let lhs = self
                        .parse_expr(0, None, false)
                        .unwrap_or_else(|_| panic!("Explicit"));
                    self.expect(TokenTyp::ParenClose, || panic!("Explicit"));
                    lhs
                }

                Some(Token {
                    typ: TokenTyp::BinOp(BinOp::Plus),
                    ..
                })
                | Some(Token {
                    typ: TokenTyp::BinOp(BinOp::Minus),
                    ..
                })
                | Some(Token {
                    typ: TokenTyp::Andp,
                    ..
                })
                | Some(Token {
                    typ: TokenTyp::BinOp(BinOp::Mult), //Ptr
                    ..
                }) => {
                    let unary_typ = self.tokstream.next().unwrap().typ.into();
                    let rhs = self
                        .parse_expr(21, None, false)
                        .unwrap_or_else(|_| panic!("Explicit"));
                    Box::new(Expr::Unary {
                        op: unary_typ,
                        target: rhs,
                    })
                }

                _ => self
                    .parse_prim_expr()
                    .unwrap_or_else(|_| panic!("Explicit")),
            }
        } else {
            lhs_.unwrap()
        };
        loop {
            match self.tokstream.peek() {
                Some(Token {
                    typ: TokenTyp::Bang,
                    ..
                }) => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Unary {
                        op: UnaryOp::Bang,
                        target: lhs,
                    });
                }

                Some(Token {
                    typ: TokenTyp::BinOp(BinOp::Mult),
                    ..
                }) => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Unary {
                        op: UnaryOp::ComplexType,
                        target: lhs,
                    });
                }

                Some(Token {
                    typ: TokenTyp::ParenOpen,
                    ..
                }) if !ignore_postfix_braces => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Call {
                        callee: lhs,
                        args: {
                            let mut args = vec![];
                            loop {
                                match self.tokstream.peek() {
                                    Some(Token {
                                        typ: TokenTyp::ParenClose,
                                        ..
                                    }) => {
                                        break;
                                    }
                                    _ => {
                                        args.push(
                                            self.parse_expr(22, None, false)
                                                .unwrap_or_else(|_| panic!("Explicit")),
                                        );
                                        match self.tokstream.peek() {
                                            Some(
                                                Token {
                                                    typ: TokenTyp::Semicolon,
                                                    ..
                                                },
                                                ..,
                                            ) => {
                                                self.tokstream.next();
                                            }
                                            Some(
                                                Token {
                                                    typ: TokenTyp::ParenClose,
                                                    ..
                                                },
                                                ..,
                                            ) => {
                                                break;
                                            }
                                            _ => panic!("Explicit"),
                                        }
                                    }
                                }
                            }
                            args
                        },
                    });
                    self.expect(TokenTyp::ParenClose, || panic!("Explicit"));
                }

                Some(Token {
                    typ: TokenTyp::BracketOpen,
                    ..
                }) => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Index {
                        base: lhs,
                        index: self
                            .parse_expr(22, None, false)
                            .unwrap_or_else(|_| panic!("Explicit")),
                    });
                    self.expect(TokenTyp::BracketClose, || panic!("Explicit"));
                }

                Some(Token {
                    typ: TokenTyp::AccessColon,
                    ..
                }) => {
                    if 21 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Chain {
                        head: lhs,
                        tail: self
                            .parse_expr(21, None, false)
                            .unwrap_or_else(|_| panic!("Explicit")),
                    });
                }

                Some(Token {
                    typ: TokenTyp::BinOp(op),
                    ..
                }) if !self.parsing_flag || (op != &BinOp::Gt && op != &BinOp::Lt) => {
                    let op = op.clone();

                    let (l_bp, r_bp) = self.infix_bp(&op);
                    if l_bp < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    let rhs = self
                        .parse_expr(r_bp, None, false)
                        .unwrap_or_else(|_| panic!("Explicit"));

                    lhs = Box::new(Expr::Bin { op, lhs, rhs });
                }

                _ => break,
            }
        }

        Ok(lhs)
    }
    fn parse_block(&mut self) -> Result<Vec<Box<Stmt<'a>>>, ()> {
        self.expect(TokenTyp::CurlyOpen, || Err(()))?;

        let mut stmts = Vec::new();

        while !matches!(
            self.tokstream.peek(),
            Some(Token {
                typ: TokenTyp::CurlyClose,
                ..
            })
        ) {
            stmts.push(self.parse_stmt()?);
        }

        self.tokstream.next();

        Ok(stmts)
    }
    fn infix_bp(&self, op: &BinOp) -> (u8, u8) {
        match op {
            BinOp::Plus | BinOp::Minus => (1, 2),
            BinOp::Mult | BinOp::Div | BinOp::Mod => (3, 4),

            BinOp::Or => (3, 4),

            BinOp::And => (5, 6),

            BinOp::Eq | BinOp::Neq => (13, 14),

            BinOp::Leq | BinOp::Geq => (15, 16),
            BinOp::Lt | BinOp::Gt if !self.parsing_flag => (15, 16),

            BinOp::Index => (100, 101),
            _ => panic!("Explicit"),
        }
    }
    fn parse_prim_expr(&mut self) -> Result<Box<Expr<'a>>, ()> {
        match self.tokstream.peek() {
            Some(&Token {
                typ: TokenTyp::Register(x),
                ..
            }) => {
                self.tokstream.next();
                Ok(Box::new(Expr::Reg(x.to_owned())))
            }

            Some(&Token {
                typ: TokenTyp::String(ref x),
                ..
            }) => {
                let x = x.clone();
                self.tokstream.next();
                Ok(Box::new(Expr::String(x.clone())))
            }

            Some(&Token {
                typ: TokenTyp::Identifier(x),
                ..
            }) => {
                self.tokstream.next();
                Ok(Box::new(Expr::Var(x.to_owned())))
            }

            Some(&Token {
                typ: TokenTyp::Integer(x),
                ..
            }) => {
                self.tokstream.next();
                Ok(Box::new(Expr::Integer(x.to_owned())))
            }

            Some(&Token {
                typ: TokenTyp::Float(x),
                ..
            }) => {
                self.tokstream.next();
                Ok(Box::new(Expr::Float(x.to_owned())))
            }
            Some(&Token {
                typ: TokenTyp::Keyword(x),
                ..
            }) => {
                self.tokstream.next();
                Ok(Box::new(Expr::Keyword(x)))
            }

            Some(&Token {
                // array
                typ: TokenTyp::StaticTyp(n),
                ..
            }) if n == StaticTyp::Array => {
                dbg!(n as u8);
                let token = self.tokstream.next().unwrap();

                if let TokenTyp::StaticTyp(x) = token.typ {
                    Ok(Box::new(Expr::TypeExpr(TypExpr {
                        typ: x,
                        payload: Some({
                            self.expect(TokenTyp::ParenOpen, || panic!("Explicit"));
                            let ret1 = self.parse_expr(0, None, false)?;
                            self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                            let ret2 = self.parse_expr(0, None, false)?;
                            self.expect(TokenTyp::ParenClose, || panic!("Explicit"));
                            Box::new(Expr::StaticType_payload_params(vec![ret1, ret2]))
                        }),
                    })))
                } else {
                    unreachable!()
                }
            }
            Some(&Token {
                // variable length type
                typ: TokenTyp::StaticTyp(n),
                ..
            }) if n as u8 >= self.typ_repr_partition_2 => {
                dbg!(n as u8);
                let token = self.tokstream.next().unwrap();

                if let TokenTyp::StaticTyp(x) = token.typ {
                    Ok(Box::new(Expr::TypeExpr(TypExpr {
                        typ: x,
                        payload: Some({
                            let mut ret = vec![];
                            self.expect(TokenTyp::ParenOpen, || panic!("Explicit"));
                            loop {
                                ret.push(self.parse_expr(0, None, false)?);
                                match self.tokstream.peek() {
                                    Some(Token {
                                        typ: TokenTyp::Semicolon,
                                        ..
                                    }) => {
                                        self.tokstream.next();
                                    }
                                    _ => break,
                                }
                            }
                            self.expect(TokenTyp::ParenClose, || panic!("Explicit"));
                            Box::new(Expr::StaticType_payload_params(ret))
                        }),
                    })))
                } else {
                    unreachable!()
                }
            }
            Some(&Token {
                // single parameter type
                typ: TokenTyp::StaticTyp(n),
                ..
            }) if n as u8 >= self.typ_repr_partition => {
                dbg!(n as u8);
                let token = self.tokstream.next().unwrap();

                if let TokenTyp::StaticTyp(x) = token.typ {
                    Ok(Box::new(Expr::TypeExpr(TypExpr {
                        typ: x,
                        payload: Some({
                            self.expect(TokenTyp::ParenOpen, || panic!("Explicit"));
                            let ret = self.parse_expr(0, None, false)?;
                            self.expect(TokenTyp::ParenClose, || panic!("Explicit"));
                            ret
                        }),
                    })))
                } else {
                    unreachable!()
                }
            }
            Some(&Token {
                // no payload type
                typ: TokenTyp::StaticTyp(n),
                ..
            }) => {
                dbg!(n);
                dbg!(n as u8);
                let token = self.tokstream.next().unwrap();

                if let TokenTyp::StaticTyp(x) = token.typ {
                    Ok(Box::new(Expr::TypeExpr(TypExpr {
                        typ: x,
                        payload: None,
                    })))
                } else {
                    unreachable!()
                }
            }
            x => Err(()),
        }
    }
    fn parse_pattern_expr(&mut self) -> Result<Box<PatternExpr<'a>>, ()> {
        match self.tokstream.peek() {
            Some(Token { typ: Wild, .. }) => {
                self.tokstream.next();
                Ok(Box::new(PatternExpr::Wildcard))
            }
            Some(Token { typ: ParenOpen, .. }) => {
                self.tokstream.next();
                let mut v = vec![];
                v.push(
                    self.parse_pattern_expr()
                        .unwrap_or_else(|_| panic!("Explicit")),
                );
                loop {
                    match self.tokstream.peek() {
                        Some(Token {
                            typ: TokenTyp::Comma,
                            ..
                        }) => {
                            self.tokstream.next();
                            v.push(
                                self.parse_pattern_expr()
                                    .unwrap_or_else(|_| panic!("Explicit")),
                            );
                        }
                        Some(Token {
                            typ: TokenTyp::ParenClose,
                            ..
                        }) => {
                            self.tokstream.next();
                            break;
                        }
                        _ => panic!("Explicit"),
                    }
                }
                Ok(Box::new(PatternExpr::Tuple(v)))
            }
            Some(Token {
                typ: Identifier(_), ..
            }) => {
                let lhs = self.parse_expr(0, None, true)?;
                match self.tokstream.peek() {
                    Some(Token { typ: ParenOpen, .. }) => {
                        self.tokstream.next();
                        let pm = self.parse_pattern_expr()?;
                        self.expect(TokenTyp::ParenClose, || panic!("Explicit"));
                        Ok(Box::new(PatternExpr::EnumVariant {
                            variant: lhs,
                            pattern: pm,
                        }))
                    }
                    _ => Ok(Box::new(PatternExpr::Expr(lhs))),
                }
            }
            None => Err(()),
            _ => Ok(Box::new(PatternExpr::Expr(
                self.parse_expr(0, None, false)?,
            ))),
        }
    }
    fn peek_3(&mut self) -> Option<&Token> {
        self.tokstream
            .tokens
            .get(self.tokstream.cursor + 2)
            .and_then(|tok| tok.as_ref())
    }
}
