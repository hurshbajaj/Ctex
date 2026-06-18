#[derive(Clone, Debug, PartialEq)]
pub enum StaticTyp {
    Str,
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
    F32, F64,
    Array,
    Tuple,
    Enum,
    Scope,
    Obj,
    Struct,
    ENTRY,
    INIT,
    Bool,
    Vector,
    Func,
    Usize,
    Isize,
    Def(String)
}
#[derive(Clone, Debug, PartialEq)]
pub enum DirectiveTyp {
    Use,
    From,
    Import,
    TypCast,
    Defer
}

#[derive(Clone, Debug, PartialEq)]
pub enum BinOp {
    Plus, Minus, Mult, Div, Mod,
    Geq, Leq, Gt, Lt, Eq, Neq, 
    And, Or,
    Index
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenTyp <'a> {
    KwLet, KwIf, KwNul, KwBlank, KwWhile, KwMatch, KwMut, KwAsg, KwTrait,
    Identifier(usize),
    Register(usize),
    Integer(u128),
    Float(f64),
    String(&'a str),
    Colon, Comma, Semicolon,
    ParenOpen, ParenClose,
    BracketOpen, BracketClose,
    CurlyOpen, CurlyClose,
    Andp, RArrow, FatRArrow, RArrowSquig, Squig, 
    BinOp(BinOp),
    Directive(DirectiveTyp),
    MetaString(&'a str),
    AccessColon,
    FlagBegin,
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
pub struct Token <'a> {
    pub typ: TokenTyp<'a>,
    pub loc: (usize, (usize, usize))
}
