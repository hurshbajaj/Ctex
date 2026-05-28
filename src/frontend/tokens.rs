#[derive(Clone)]
pub enum StaticTyp {
    Str,
    U8, U16, U32, U64, U128,
    I8, I16, I32, I64, I128,
    F32, F64,
    Def(String)
}

#[derive(Clone)]
pub enum FlgTyp {
    Typ(StaticTyp),
    Trait(Vec<String>),
    Asg,
    Mut
}

#[derive(Clone)]
pub enum DirectiveTyp {
    Use,
    From,
    Import,
    TypCast,
    Defer
}

#[derive(Clone)]
pub enum TokenTyp {
    KwLet, KwIf, KwNul, KwBlank,
    Identifier(usize),
    Register(usize),
    Integer(usize),
    Float(f64),

    Ptr,
    Colon, Semicolon,
    ParenOpen, ParenClose,
    BracketOpen, BracketClose,
    CurlyOpen, CurlyClose,
    Andp, RArrow, RArrowSquig, Squig, //Squig -> Register Drop
    Plus, Minus, Mult, Div, Mod,
    Dot,

    Directive(DirectiveTyp),
    MetaString(String),
    AccessColon,
    Wild,
    Geq, Leq, Eq, Neq,

    Flg(FlgTyp)
}

pub struct Token {
    pub typ: TokenTyp,
    pub loc: (usize, usize)
}
