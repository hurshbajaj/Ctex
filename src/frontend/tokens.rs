#[derive(Clone)]
pub enum StaticTyp {
    Str(String),
    Uint(usize), Int(usize), 
    Def(String)
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
    Andp, RArrow, RArrowSquig, Squig,
    Plus, Minus, Mult, Div, Mod,
    Dot,

    FlgType(StaticTyp), FlgAsg, FlgTrait(Vec<StaticTyp>), FlgMut, 
}

pub struct Token {
    pub typ: TokenTyp,
    pub loc: (usize, usize)
}
