use crate::frontend::tokens::{BinOp, Flg, Keyword, Token, TokenTyp};
use std::iter::Peekable;
use std::marker::PhantomData;
use std::vec::IntoIter;

pub struct Parser<'a> {
    tokstream: Peekable<IntoIter<Token>>,
    _marker: PhantomData<&'a ()>,
    ignore_semi_c: bool,
}

#[derive(Debug)]
pub enum UnaryOp {
    Minus,
    Plus,
    Bang,
    Deref,
    Ptr,
}

#[derive(Debug)]
pub enum CompilerDirective {
    TypeCast,
    Defer,
}

#[derive(Debug)]
pub enum DirectiveFlgOpts<'a> {
    Collapse,
    StaticType(TypExpr<'a>),
}

#[derive(Debug)]
pub enum Stmt<'a> {
    Asg {
        target: Box<Expr<'a>>,
        value: Box<Expr<'a>>,
    },
    ExprStmt(Box<Expr<'a>>),
    Scope(Vec<Box<Stmt<'a>>>),
    Use(Vec<&'a str>),
    From {
        path: &'a str,
        pattern: &'a str,
    },
    CompilerDirective {
        directive: CompilerDirective,
        flags: Vec<DirectiveFlgOpts<'a>>,
        block: Vec<Stmt<'a>>,
    },
}

#[derive(Debug)]
pub enum TypExpr<'a> {
    Str,
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
    Tuple(Vec<Box<TypExpr<'a>>>),
    Enum,
    Scope,
    Obj(&'a str),
    Array { typ: Box<TypExpr<'a>>, size: usize },
    Struct,
    ENTRY,
    INIT,
    Bool,
    Vector,
    Func(Box<TypExpr<'a>>),
    Trait(Box<TypExpr<'a>>),
    Usize,
    Isize,

    Def(String),
    Path(Vec<&'a str>),
    Inferred,
}

#[derive(Debug)]
pub enum PatternExpr<'a> {
    Wildcard,
    Tuple(Vec<Box<PatternExpr<'a>>>),
    List(Vec<Expr<'a>>),
    EnumVariant {
        variant: Expr<'a>,
        pattern: Box<PatternExpr<'a>>,
    },
}

#[derive(Debug)]
pub struct MatchArm<'a> {
    pattern: PatternExpr<'a>,
    guard: Option<Expr<'a>>,
    body: Vec<Stmt<'a>>,
}

#[derive(Debug)]
pub enum Expr<'a> {
    Keyword(Keyword),
    String(String),
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
        typ: TypExpr<'a>,
    },
    If {
        cond: Box<Expr<'a>>,
        then_block: Vec<Box<Stmt<'a>>>,
        else_block: Option<Vec<Box<Stmt<'a>>>>,
    },
    Match {
        expr: Box<Expr<'a>>,
        arms: Vec<MatchArm<'a>>,
    },

    Scope(Vec<Box<Stmt<'a>>>),
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
    pub fn new(tokstream: Peekable<IntoIter<Token>>) -> Parser<'a> {
        Parser {
            tokstream,
            _marker: PhantomData,
            ignore_semi_c: false,
        }
    }
    pub fn from_ast(&mut self) -> Stmt<'a> {
        let mut ast = vec![];
        while let Some(_) = self.tokstream.peek() {
            ast.push(self.parse_stmt());
        }
        Stmt::Scope(ast)
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
    fn parse_stmt(&mut self) -> Box<Stmt<'a>> {
        match self.tokstream.peek() {
            _ => {
                let stmt = Box::new(Stmt::ExprStmt(self.parse_expr(0)));
                if !self.ignore_semi_c {
                    self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                } else {
                    self.ignore_semi_c = false;
                }
                stmt
            }
        }
    }
    fn parse_expr(&mut self, min_bp: u8) -> Box<Expr<'a>> {
        let mut lhs = match self.tokstream.peek() {
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
                            break;
                        }
                        _ => {
                            els.push(self.parse_expr(22));
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
            }) => Box::new(Expr::Scope(
                self.parse_block().unwrap_or_else(|_| panic!("Explicit")),
            )),
            Some(Token {
                typ: TokenTyp::Keyword(Keyword::If),
                ..
            }) => {
                self.tokstream.next();
                let cond = self.parse_expr(0);
                let then_block = self.parse_block().unwrap_or_else(|_| panic!("Explicit"));

                let else_block = if matches!(
                    self.tokstream.peek(),
                    Some(Token {
                        typ: TokenTyp::Keyword(Keyword::Else),
                        ..
                    })
                ) {
                    self.tokstream.next();
                    Some(self.parse_block().unwrap_or_else(|_| panic!("Explicit")))
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
                let lhs = self.parse_expr(0);
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
                typ: TokenTyp::BinOp(BinOp::Mult),
                ..
            }) => {
                let unary_typ = self.tokstream.next().unwrap().typ.into();
                let rhs = self.parse_expr(21);
                Box::new(Expr::Unary {
                    op: unary_typ,
                    target: rhs,
                })
            }

            _ => self.parse_prim_expr(),
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
                    typ: TokenTyp::ParenOpen,
                    ..
                }) => {
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
                                        args.push(self.parse_expr(22));
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
                        index: self.parse_expr(22),
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
                        tail: self.parse_expr(21),
                    });
                }

                Some(Token {
                    typ: TokenTyp::BinOp(op),
                    ..
                }) => {
                    let op = op.clone();

                    let (l_bp, r_bp) = self.infix_bp(&op);
                    if l_bp < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    let rhs = self.parse_expr(r_bp);

                    lhs = Box::new(Expr::Bin { op, lhs, rhs });
                }

                _ => break,
            }
        }

        lhs
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
            stmts.push(self.parse_stmt());
        }

        self.expect(TokenTyp::CurlyClose, || Err(()))?;

        Ok(stmts)
    }
    fn infix_bp(&self, op: &BinOp) -> (u8, u8) {
        match op {
            BinOp::Plus | BinOp::Minus => (1, 2),
            BinOp::Mult | BinOp::Div | BinOp::Mod => (3, 4),

            BinOp::Or => (3, 4),

            BinOp::And => (5, 6),

            BinOp::Eq | BinOp::Neq => (13, 14),

            BinOp::Lt | BinOp::Leq | BinOp::Gt | BinOp::Geq => (15, 16),

            BinOp::Index => (100, 101),
            _ => panic!("Explicit"),
        }
    }
    fn parse_prim_expr(&mut self) -> Box<Expr<'a>> {
        match self.tokstream.peek() {
            Some(&Token {
                typ: TokenTyp::Register(x),
                ..
            }) => {
                self.tokstream.next();
                Box::new(Expr::Reg(x.to_owned()))
            }

            Some(&Token {
                typ: TokenTyp::String(ref x),
                ..
            }) => {
                let x = x.clone();
                self.tokstream.next();
                Box::new(Expr::String(x.clone()))
            }

            Some(&Token {
                typ: TokenTyp::Identifier(x),
                ..
            }) => {
                self.tokstream.next();
                Box::new(Expr::Var(x.to_owned()))
            }

            Some(&Token {
                typ: TokenTyp::Integer(x),
                ..
            }) => {
                self.tokstream.next();
                Box::new(Expr::Integer(x.to_owned()))
            }

            Some(&Token {
                typ: TokenTyp::Float(x),
                ..
            }) => {
                self.tokstream.next();
                Box::new(Expr::Float(x.to_owned()))
            }
            Some(&Token {
                typ: TokenTyp::Keyword(x),
                ..
            }) => {
                self.tokstream.next();
                Box::new(Expr::Keyword(x))
            }
            _ => panic!("Implicit"),
        }
    }
}
