use crate::frontend::tokens::{Token, TokenTyp, BinOp};
use std::iter::Peekable;
use std::vec::IntoIter;
use std::marker::PhantomData;

pub struct Parser <'a> {
    tokstream: Peekable<IntoIter<Token<'a>>>,
    _marker: PhantomData<&'a ()>
}

#[derive(Debug)]
pub enum UnaryOp {
    Minus,
    Plus,
    Bang,
    Deref,
    Ptr
}

#[derive(Debug)]
pub enum CompilerDirective {
    TypeCast,
    Defer
}

#[derive(Debug)]
pub enum DirectiveFlgOpts <'a> {
    Collapse,
    StaticType(TypExpr<'a>)
}

#[derive(Debug)]
pub enum FlgOpts {
    Mut,
}

#[derive(Debug)]
pub enum Stmt<'a>{
    Asg{target: Box<Expr<'a>>, value: Box<Expr<'a>>},
    ExprStmt(Box<Expr<'a>>),
    Scope(Vec<Box<Stmt<'a>>>),
    Use(Vec<&'a str>),
    From{
        path: &'a str,
        pattern: &'a str
    },
    CompilerDirective{
        directive: CompilerDirective,
        flags: Vec<DirectiveFlgOpts<'a>>,
        block: Vec<Stmt<'a>>
    }
}

#[derive(Debug)]
pub enum TypExpr<'a>{
    Str,
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
    F32, F64,
    Tuple(Vec<Box<TypExpr<'a>>>),
    Enum,
    Scope,
    Obj(&'a str),
    Array{typ: Box<TypExpr<'a>>, size: usize},
    Struct,
    ENTRY,
    INIT,
    Bool,
    Vector,
    Func(Box<TypExpr<'a>>),
    Usize,
    Isize,

    Def(String),
    Path(Vec<&'a str>),
    Inferred
}

#[derive(Debug)]
pub enum PatternExpr <'a> {
    Wildcard,
    Tuple(Vec<Box<PatternExpr<'a>>>),
    List(Vec<Expr<'a>>),
    EnumVariant{variant: Expr<'a>, pattern: Box<PatternExpr<'a>>}
}

#[derive(Debug)]
pub struct MatchArm <'a> {
    pattern: PatternExpr<'a>,
    guard: Option<Expr<'a>>,
    body: Vec<Stmt<'a>>
}

#[derive(Debug)]
pub enum Expr <'a> {
    String(&'a str),
    Integer(u128),
    Float(f64),
    Var(usize),
    Reg(usize),
    Unary{op: UnaryOp, target: Box<Expr<'a>>},
    Bin{op: BinOp, lhs: Box<Expr<'a>>, rhs: Box<Expr<'a>>},
    Call{callee: Box<Expr<'a>>, args: Vec<Box<Expr<'a>>>},
    Index {
        base: Box<Expr<'a>>,
        index: Box<Expr<'a>>,
    },
    Chain{
        head: Box<Expr<'a>>,
        tail: Box<Expr<'a>>
    },
    Array(Vec<Expr<'a>>),
    Decl{identifier: Box<PatternExpr<'a>>, value: Box<Expr<'a>>, flags: Vec<FlgOpts>, typ: TypExpr<'a>},
    If{cond: Box<Expr<'a>>, then_block: Vec<Stmt<'a>>, else_block: Option<Vec<Stmt<'a>>>},
    Match{expr: Box<Expr<'a>>, arms: Vec<MatchArm<'a>>},

    Block(Vec<Box<Stmt<'a>>>),
}

impl<'a> From<TokenTyp<'a>> for UnaryOp {
    fn from(value: TokenTyp) -> Self {
        match value {
            TokenTyp::BinOp(BinOp::Minus) => UnaryOp::Minus,
            TokenTyp::BinOp(BinOp::Plus) => UnaryOp::Plus,
            TokenTyp::Andp => UnaryOp::Deref,
            TokenTyp::BinOp(BinOp::Mult) => UnaryOp::Ptr,
            _ => unreachable!()
        }
    }
}

impl<'a> Parser<'a> {
    pub fn new(tokstream: Peekable<IntoIter<Token<'a>>>) -> Parser<'a> {
        Parser {tokstream, _marker: PhantomData}
    }
    pub fn from_ast(&mut self) -> Stmt<'a> {
        let mut ast = vec![];
        while let Some(_) = self.tokstream.peek() {
            ast.push( self.parse_stmt() );
        }
        Stmt::Scope(ast)
    }
    fn expect(&mut self, expected: TokenTyp, panic_handler: fn()) {
        match self.tokstream.peek() {
            Some(tok) if tok.typ == expected => {
                self.tokstream.next();
            }
            _ => panic_handler(),
        }
    }
    fn parse_stmt(&mut self) -> Box<Stmt<'a>> {
        match self.tokstream.peek() {
            _ => {
                let stmt = Box::new(Stmt::ExprStmt(self.parse_expr())); 
                self.expect(TokenTyp::Semicolon, ||{panic!("Explicit")});
                stmt
            }
        }
    }
    fn parse_expr(&mut self)-> Box<Expr<'a>> {
        match self.tokstream.peek().unwrap() {
            _ => self.parse_bin_op(0)
        }
    }
    fn parse_bin_op(&mut self, min_bp: u8) -> Box<Expr<'a>> {
        let mut lhs = match self.tokstream.peek() {
            Some(Token {typ: TokenTyp::ParenOpen, ..}) => {
                self.tokstream.next();
                let lhs = self.parse_bin_op(0);
                self.expect(TokenTyp::ParenClose, ||{panic!("Explicit")});
                lhs
            },

            Some(Token{typ: TokenTyp::BinOp(BinOp::Plus), ..})
            | Some(Token{typ: TokenTyp::BinOp(BinOp::Minus), ..})
            | Some(Token{typ: TokenTyp::Andp, ..})
            | Some(Token{typ: TokenTyp::BinOp(BinOp::Mult), ..}) => {
                let unary_typ = self.tokstream.next().unwrap().typ.into();
                let rhs = self.parse_bin_op(21);
                Box::new(Expr::Unary { op: unary_typ, target: rhs })
            },

            _ => {
                self.parse_prim_expr()
            }
        };
        loop {
            match self.tokstream.peek() {
                Some(Token { typ: TokenTyp::Bang, .. }) => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Unary {
                        op: UnaryOp::Bang,
                        target: lhs,
                    });
                },
                Some(Token { typ: TokenTyp::ParenOpen, .. }) => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Call { callee: lhs, args: {
                        let mut args = vec![];
                        loop{
                           match self.tokstream.peek() {
                                Some(Token{typ: TokenTyp::ParenClose, ..}) => {break;}
                                _ => {
                                    args.push(self.parse_bin_op(22)); 
                                    match self.tokstream.peek() {
                                        Some(Token{typ:TokenTyp::Semicolon, ..}, ..) => {
                                            self.tokstream.next();
                                        },
                                        Some(Token{typ:TokenTyp::ParenClose, ..}, ..) => {
                                            break;
                                        },
                                        _ => panic!("Explicit")
                                    }
                                }
                           }
                        }
                        args
                    }});
                    self.expect(TokenTyp::ParenClose, ||{panic!("Explicit")});
                },

                Some(Token { typ: TokenTyp::BracketOpen, .. }) => {
                    if 22 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Index { base: lhs, index: self.parse_bin_op(22) });
                    self.expect(TokenTyp::BracketClose, ||{panic!("Explicit")});
                },
                Some(Token { typ: TokenTyp::AccessColon, .. }) => {
                    if 21 < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    lhs = Box::new(Expr::Chain { head: lhs, tail: self.parse_bin_op(21) });
                },

                Some(Token { typ: TokenTyp::BinOp(op), .. }) => {
                    let op = op.clone();

                    let (l_bp, r_bp) = self.infix_bp(&op);
                    if l_bp < min_bp {
                        break;
                    }

                    self.tokstream.next();

                    let rhs = self.parse_bin_op(r_bp);

                    lhs = Box::new(Expr::Bin {
                        op,
                        lhs,
                        rhs,
                    });
                },

                _ => break,
            }
        }

        lhs
    }   
    fn infix_bp(&self, op: &BinOp) -> (u8, u8) {
        match op {
            BinOp::Plus
                | BinOp::Minus => (1,2),
            BinOp::Mult
                | BinOp::Div
                | BinOp::Mod => (3, 4),

            BinOp::Or => (3, 4),

            BinOp::And => (5, 6),

            BinOp::Eq | BinOp::Neq => (13, 14),

            BinOp::Lt
                | BinOp::Leq
                | BinOp::Gt
                | BinOp::Geq => (15, 16),

            BinOp::Index => (100, 101),
                _ => panic!("Explicit"),
        }
    }
    fn parse_prim_expr(&mut self) -> Box<Expr<'a>> {
        match self.tokstream.peek() {
            Some(&Token {typ: TokenTyp::Register(x), .. }) => {
                self.tokstream.next(); 
                Box::new(Expr::Reg(x.to_owned()))
            },

            Some(&Token {typ: TokenTyp::String(x), .. }) => {
                self.tokstream.next(); 
                Box::new(Expr::String(x))
            },

            Some(&Token {typ: TokenTyp::Identifier(x), .. }) => {
                self.tokstream.next(); 
                Box::new(Expr::Var(x.to_owned()))
            },

            Some(&Token {typ: TokenTyp::Integer(x), .. }) => {
                self.tokstream.next(); 
                Box::new(Expr::Integer(x.to_owned()))
            },

            Some(&Token {typ: TokenTyp::Float(x), .. }) => {
                self.tokstream.next(); 
                Box::new(Expr::Float(x.to_owned()))
            },
            _ => panic!("Implicit")
        }
    }
}



