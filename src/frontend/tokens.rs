use crate::frontend::ast::StaticTyp;

#[derive(Clone, Debug, PartialEq)]
pub enum DirectiveTyp {
    Use,
    From,
    Import,
    TypCast,
    Defer,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Plus,
    Minus,
    Mult,
    Div,
    Mod,
    Geq,
    Leq,
    Gt,
    Lt,
    Eq,
    Neq,
    And,
    Or,
    Index,
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Keyword {
    Let,
    If,
    Else,
    True,
    False,
    Nul,
    Blank,
    While,
    Match,
    Mut,
    Asg,
    Trait,
}

#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum Flg {
    Asg,
    Mutable,
    Type,
    Trait,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenTyp {
    Keyword(Keyword),
    Identifier(usize),
    Register(usize),
    Integer(u128),
    Float(f64),
    String(Box<str>),
    Colon,
    Comma,
    Semicolon,
    ParenOpen,
    ParenClose,
    BracketOpen,
    BracketClose,
    CurlyOpen,
    CurlyClose,
    Andp,
    RArrow,
    FatRArrow,
    RArrowSquig,
    Squig,
    BinOp(BinOp),
    Directive(DirectiveTyp),
    MetaString(Box<str>),
    AccessColon,
    Wild,
    Bang,
    StaticTyp(StaticTyp),
    InvalidIdent(usize),
    Unknown,
    UnterminatedString,
    UnterminatedMultilineComment,
    UnrecognizedCompilerDirective(usize),
}
#[derive(Debug)]
pub struct Token {
    pub typ: TokenTyp,
    pub loc: (usize, (usize, usize)),
}
