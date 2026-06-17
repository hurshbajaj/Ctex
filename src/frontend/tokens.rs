#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
pub enum DirectiveTyp {
    Use,
    From,
    Import,
    TypCast,
    Defer
}

#[derive(Clone, Debug)]
pub enum BinOp {
    Plus, Minus, Mult, Div, Mod,
    Geq, Leq, Ge, Le, Eq, Neq, 
    And, Or,
}

#[derive(Clone, Debug)]
pub enum TokenTyp {
    KwLet, KwIf, KwNul, KwBlank, KwWhile, KwMatch, KwMut, KwAsg, KwTrait, KwMod,
    Identifier(usize),
    Register(usize),
    Integer(u128),
    Float(f64),
    String(String),
    Ptr,
    Colon, Comma, Semicolon,
    ParenOpen, ParenClose,
    BracketOpen, BracketClose,
    CurlyOpen, CurlyClose,
    Andp, RArrow, FatRArrow, RArrowSquig, Squig, 
    BinOp(BinOp),
    Dot,
    Directive(DirectiveTyp),
    MetaString(String),
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
pub struct Token {
    pub typ: TokenTyp,
    pub loc: (usize, (usize, usize))
}
