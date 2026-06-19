use crate::frontend::tokens::{BinOp, Flg, Keyword, StaticTyp, Token, TokenTyp};
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
    ComplexType,
}

#[derive(Debug)]
pub enum CompilerDirective {
    TypeCast,
    Defer,
}

#[derive(Debug)]
pub enum DirectiveFlgOpts {
    Collapse,
    StaticType(StaticTyp),
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
        flags: Vec<DirectiveFlgOpts>,
        block: Vec<Stmt<'a>>,
    },
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
    Expr(Box<Expr<'a>>),
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
    TypeExpr(StaticTyp),
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
        flag_payload: Vec<Box<Expr<'a>>>,
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
                let stmt = Box::new(Stmt::ExprStmt(
                    self.parse_expr(0).unwrap_or_else(|()| panic!("Explicit")),
                ));
                if !self.ignore_semi_c {
                    self.expect(TokenTyp::Semicolon, || panic!("Explicit"));
                } else {
                    self.ignore_semi_c = false;
                }
                stmt
            }
        }
    }
    fn parse_expr(&mut self, min_bp: u8) -> Result<Box<Expr<'a>>, ()> {
        let mut lhs = match self.tokstream.peek() {
            Some(Token {
                typ: TokenTyp::Keyword(Keyword::Let),
                ..
            }) => {
                self.tokstream.next();
                let target = self.parse_expr(0).unwrap_or_else(|_| panic!("Explicit"));
                let mut flags = vec![];
                let mut flags_pl = vec![];
                loop {
                    match self.tokstream.peek() {
                        Some(Token {
                            typ: TokenTyp::FlagBegin,
                            ..
                        }) => {
                            self.tokstream.next();
                            match self.tokstream.peek() {
                                Some(Token {
                                    typ: TokenTyp::Flag(Flg::Single(single)),
                                    ..
                                }) => {
                                    flags.push(Flg::Single(single.clone()));
                                    self.tokstream.next();
                                    self.expect(TokenTyp::FlagEnd, || panic!("Explicit"));
                                }
                                Some(Token {
                                    typ: TokenTyp::Flag(flg),
                                    ..
                                }) => {
                                    flags.push(flg.clone());
                                    self.tokstream.next();
                                    self.expect(TokenTyp::FlagColon, || panic!("Explicit"));
                                    flags_pl.push(
                                        self.parse_expr(0).unwrap_or_else(|_| panic!("Explicit")),
                                    );
                                    self.expect(TokenTyp::FlagEnd, || panic!("Explicit"));
                                }
                                _ => {
                                    flags.push(Flg::Type);
                                    flags_pl.push(
                                        self.parse_expr(0).unwrap_or_else(|_| panic!("Explicit")),
                                    );
                                    self.expect(TokenTyp::FlagEnd, || panic!("Explicit"));
                                }
                            }
                        }
                        _ => break,
                    }
                }
                let value = self.parse_expr(0).unwrap_or_else(|_| panic!("Explicit"));
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
                            break;
                        }
                        _ => {
                            els.push(self.parse_expr(22).unwrap_or_else(|_| panic!("Explicit")));
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
                let cond = self.parse_expr(0).unwrap_or_else(|_| panic!("Explicit"));
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
                let lhs = self.parse_expr(0).unwrap_or_else(|_| panic!("Explicit"));
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
                let rhs = self.parse_expr(21).unwrap_or_else(|_| panic!("Explicit"));
                Box::new(Expr::Unary {
                    op: unary_typ,
                    target: rhs,
                })
            }

            _ => self
                .parse_prim_expr()
                .unwrap_or_else(|_| panic!("Explicit")),
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
                                        args.push(
                                            self.parse_expr(22)
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
                        index: self.parse_expr(22).unwrap_or_else(|_| panic!("Explicit")),
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
                        tail: self.parse_expr(21).unwrap_or_else(|_| panic!("Explicit")),
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

                    let rhs = self.parse_expr(r_bp).unwrap_or_else(|_| panic!("Explicit"));

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
            _ => Err(()),
        }
    }
}
