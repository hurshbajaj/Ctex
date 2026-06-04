use crate::frontend::tokens::{DirectiveTyp, StaticTyp, TokenTyp};

pub fn lookup_keyword(word: &str) -> Option<TokenTyp> {
    match word {
        "if" => Some(TokenTyp::KwIf),
        "while" => Some(TokenTyp::KwWhile),
        "match" => Some(TokenTyp::KwMatch),
        "mutable" => Some(TokenTyp::KwMut),
        "asg" => Some(TokenTyp::KwAsg),
        "$" => Some(TokenTyp::KwLet),
        "nul" => Some(TokenTyp::KwNul),
        "trait" => Some(TokenTyp::KwTrait),
        "mod" => Some(TokenTyp::KwMod),
        ".." => Some(TokenTyp::KwBlank),
        "string" => Some(TokenTyp::StaticTyp(StaticTyp::Str)),
        "u8" => Some(TokenTyp::StaticTyp(StaticTyp::U8)),
        "u16" => Some(TokenTyp::StaticTyp(StaticTyp::U16)),
        "u32" => Some(TokenTyp::StaticTyp(StaticTyp::U32)),
        "u64" => Some(TokenTyp::StaticTyp(StaticTyp::U64)),
        "u128" => Some(TokenTyp::StaticTyp(StaticTyp::U128)),
        "i8" => Some(TokenTyp::StaticTyp(StaticTyp::I8)),
        "i16" => Some(TokenTyp::StaticTyp(StaticTyp::I16)),
        "i32" => Some(TokenTyp::StaticTyp(StaticTyp::I32)),
        "i64" => Some(TokenTyp::StaticTyp(StaticTyp::I64)),
        "i128" => Some(TokenTyp::StaticTyp(StaticTyp::I128)),
        "array" => Some(TokenTyp::StaticTyp(StaticTyp::Array)),
        "tuple" => Some(TokenTyp::StaticTyp(StaticTyp::Tuple)),
        "enum" => Some(TokenTyp::StaticTyp(StaticTyp::Enum)),
        "scope" => Some(TokenTyp::StaticTyp(StaticTyp::Scope)),
        "object" => Some(TokenTyp::StaticTyp(StaticTyp::Obj)),
        "struct" => Some(TokenTyp::StaticTyp(StaticTyp::Struct)),
        "ENTRY" => Some(TokenTyp::StaticTyp(StaticTyp::ENTRY)),
        "INIT" => Some(TokenTyp::StaticTyp(StaticTyp::INIT)),
        "bool" => Some(TokenTyp::StaticTyp(StaticTyp::Bool)),
        "vec" => Some(TokenTyp::StaticTyp(StaticTyp::Vector)),
        "func" => Some(TokenTyp::StaticTyp(StaticTyp::Func)),
        "usize" => Some(TokenTyp::StaticTyp(StaticTyp::Usize)),
        "isize" => Some(TokenTyp::StaticTyp(StaticTyp::Isize)),
        _ => None,
    }
}

pub fn lookup_directive(word: &str) -> Option<TokenTyp> {
    match word {
        "@use" => Some(TokenTyp::Directive(DirectiveTyp::Use)),
        "@from" => Some(TokenTyp::Directive(DirectiveTyp::From)),
        "@import" => Some(TokenTyp::Directive(DirectiveTyp::Import)),
        "@defer" => Some(TokenTyp::Directive(DirectiveTyp::Defer)),
        "@type_cast" => Some(TokenTyp::Directive(DirectiveTyp::TypCast)),
        _ => None,
    }
}

