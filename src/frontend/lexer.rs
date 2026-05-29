use crate::frontend::tokens::{DirectiveTyp, StaticTyp, Token, TokenTyp};
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::collections::HashMap;

use paste::paste;

macro_rules! static_type {
    ($kind:ident $bits:literal) => {
        paste! {
            (
                stringify!([<$kind $bits>]).to_string(),
                TokenTyp::StaticTyp(
                    StaticTyp::[<$kind:upper $bits>]
                )
            )
        }
    };
}

pub struct Lexer {
    pub mmap: [Mmap;2],
    pub file: File,
    pub file_at: usize,
    pub buf_n: usize,
    pub mmap_active: u8,
    pub i: *const u8,
    pub loc: (usize, usize),
    pub tokStream: Vec<Token>,
    pub idents: HashMap<String, TokenTyp>,
    pub idents_n: usize,
    pub dispatch_table: [DT_Handler; 256],
    file_len: usize,
    pos: usize,
}

type DT_Handler = fn(&mut Lexer);

pub fn DT_default(lexer: &mut Lexer) { unsafe{
    panic!(
        "[Lexer] [{}:{}] Unrecognized character {:?}",
        lexer.loc.0,
        lexer.loc.1,
        *lexer.i as char
    );
}}
pub fn DT_nl(lexer:&mut Lexer) {unsafe{
    lexer.loc.1 += 1;
    lexer.loc.0 = 0;
    lexer.advance(1);
}}  
pub fn DT_whitespace(lexer:&mut Lexer) {unsafe {
    lexer.loc.0 += 1;
    lexer.advance(1);
}}
pub fn DT_numeric(lexer: &mut Lexer) {
    unsafe{
        let mut num = String::new();
        let mut is_float = false;
        while matches!(lexer.peek(0), b'0'..=b'9' | b',' | b'.') {
            let tmp = lexer.advance(1);
            if tmp != b',' {
                if tmp == b'.' {is_float = !is_float; if is_float == false{panic!("[Lexer] [{}:{}] Multiple decimal points within a float not allowed!", lexer.loc.0, lexer.loc.1)}}
                num.push(tmp as char); 
            }
        }
        if matches!(num.chars().next().unwrap_or(' '), ',' | '.') || matches!(num.chars().next_back().unwrap_or(' '), ',' | '.'){
            panic!("[Lexer] [{}:{}] Leading/trailing commas/decimals not allowed!", lexer.loc.0, lexer.loc.1)
        }
        lexer.tokStream.push(Token{typ: if is_float {TokenTyp::Float(num.parse().unwrap())} else {TokenTyp::Integer(num.parse().unwrap())},loc: lexer.loc});
        lexer.loc.0 += num.len();
    }
}

pub fn DT_identifier(lexer: &mut Lexer) { unsafe{
     let mut identifier = String::new();
     identifier.push(lexer.advance(1) as char);
     while matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
         identifier.push(lexer.advance(1) as char);        
     }

     if let Some(value) = lexer.idents.get(&identifier) {
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: value.clone(),
         });
     } else {
         lexer.idents_n += 1;
         lexer.idents.insert(identifier.clone(), TokenTyp::Identifier(lexer.idents_n));
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: TokenTyp::Identifier(lexer.idents_n),
         });
     }
     lexer.loc.0 += identifier.len();
}}

pub fn DT_register(lexer: &mut Lexer) { unsafe{
     let mut register = String::new();
     register.push(lexer.advance(1) as char);
     while matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
         register.push(lexer.advance(1) as char);        
     }

     if let Some(value) = lexer.idents.get(&register) {
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: value.clone(),
         });
     } else {
         lexer.idents_n += 1;
         lexer.idents.insert(register.clone(), TokenTyp::Register(lexer.idents_n));
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: TokenTyp::Register(lexer.idents_n),
         });
     }
     lexer.loc.0 += register.len();
}}

pub fn DT_ptr(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Ptr});
    lexer.loc.0 += 1;
}

pub fn DT_andp(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Andp});
    lexer.loc.0 += 1;
}

pub fn DT_dot(lexer: &mut Lexer) {
    if lexer.peek(1) != b'.' {
        lexer.advance(1);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Dot});
        lexer.loc.0 += 1;
    } else {
        lexer.advance(2);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::KwBlank});
        lexer.loc.0 += 2;
    }
}

pub fn DT_let(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::KwLet});
    lexer.loc.0 += 1;
}

pub fn DT_colon(lexer: &mut Lexer) {
    if lexer.peek(1) == b':' {
        lexer.advance(2);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::AccessColon});
        lexer.loc.0 += 2;
    } else {
        lexer.advance(1);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Colon});
        lexer.loc.0 += 1;
    }
}

pub fn DT_semi_colon(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Semicolon});
    lexer.loc.0 += 1;
}

pub fn DT_comma(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token { loc: lexer.loc, typ: TokenTyp::Comma });
    lexer.loc.0 += 1;
}

pub fn DT_curly_open(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::CurlyOpen});
    lexer.loc.0 += 1;
}

pub fn DT_curly_close(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::CurlyClose});
    lexer.loc.0 += 1;
}

pub fn DT_paren_open(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::ParenOpen});
    lexer.loc.0 += 1;
}

pub fn DT_paren_close(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::ParenClose});
    lexer.loc.0 += 1;
}

pub fn DT_bracket_open(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::BracketOpen});
    lexer.loc.0 += 1;
}

pub fn DT_bracket_close(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::BracketClose});
    lexer.loc.0 += 1;
}

pub fn DT_wild(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Wild});
    lexer.loc.0 += 1;
}

pub fn DT_question(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token { loc: lexer.loc, typ: TokenTyp::Question });
    lexer.loc.0 += 1;
}

pub fn DT_plus (lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Plus});
    lexer.loc.0 += 1;
}

pub fn DT_mult(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Mult});
    lexer.loc.0 += 1;
}

pub fn DT_div(lexer: &mut Lexer) {
    lexer.advance(1);
    if lexer.peek(0) == b'/' {
        while lexer.peek(0) != b'\n' {
            lexer.advance(1);
        } lexer.advance(1); lexer.loc.0 = 0; lexer.loc.1 += 1;
    } else if lexer.peek(0) == b'*' {
        lexer.advance(1);
        let mut ending = false;
        let mut cols_tba = 1;
        loop {
            if lexer.at_eof() {panic!("[Lexer] [{}:{}] Unclosed multiline comment!", lexer.loc.0, lexer.loc.1)}
            match lexer.peek(0) {
                b'*' => {
                    ending = true;
                    lexer.advance(1);
                    cols_tba += 1;
                }

                b'/' if ending => {
                    lexer.advance(1);
                    lexer.loc.0 += cols_tba + 1;
                    break;
                }

                b'\n' if ending => {
                    ending = false;
                    lexer.advance(1);
                    cols_tba = 0;
                    lexer.loc.1 += 1;
                }

                _ => {
                    ending = false;
                    lexer.advance(1);
                    cols_tba += 1;
                }
            }
        } 
    } else {
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Div});
        lexer.loc.0 += 1;
    }
}

pub fn DT_minus(lexer: &mut Lexer) {
    if lexer.peek(1) != b'>' {
        lexer.advance(1);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Minus});
        lexer.loc.0 += 1;
    }else{
        lexer.advance(2);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::RArrow});
        lexer.loc.0 += 2;
    }
}

pub fn DT_squig(lexer: &mut Lexer) {
    if lexer.peek(1) == b'>' {
        lexer.advance(2);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::RArrowSquig});
        lexer.loc.0 += 2;
    }else if lexer.peek(1) == b'%' {
        lexer.advance(1);
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Squig});
        lexer.loc.0 += 1;
    } else {
        let mut string = String::new();
        string.push(lexer.advance(1) as char);
        while matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
            string.push(lexer.advance(1) as char);        
        }

        if let Some(value) = lexer.idents.get(&string) {
            lexer.tokStream.push(Token {
                loc: lexer.loc,
                typ: value.clone(),
            });
        } else {
            lexer.idents_n += 1;
            lexer.idents.insert(string.clone(), TokenTyp::Identifier(lexer.idents_n));
            lexer.tokStream.push(Token {
                loc: lexer.loc,
                typ: TokenTyp::MetaString(string.clone()),
            });
        }

        lexer.loc.0 += string.len();
    }
}

pub fn DT_directive(lexer: &mut Lexer) {
     let mut register = String::new();
     register.push(lexer.advance(1) as char);        
     while matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
            register.push(lexer.advance(1) as char);        
     }

     let value = lexer.idents.get(&register).expect("[Lexer] Unrecognized Compiler Directive!"); 

     lexer.tokStream.push(Token {
         loc: lexer.loc,
         typ: value.clone(),
     });

     lexer.loc.0 += register.len();
}

pub fn DT_leq(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Leq});
    lexer.loc.0 += 1;
}   

pub fn DT_geq(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Geq});
    lexer.loc.0 += 1;
}   

pub fn DT_le(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Le});
    lexer.loc.0 += 1;
}   

pub fn DT_ge(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Ge});
    lexer.loc.0 += 1;
} 

pub fn DT_eq(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Eq});
    lexer.loc.0 += 1;
}   

pub fn DT_neq(lexer: &mut Lexer) {
    if lexer.peek(1) != b'=' {panic!("[Lexer] [{}:{}] Unrecognized (!) token!", lexer.loc.0, lexer.loc.1)}
    lexer.advance(2);
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Neq});
    lexer.loc.0 += 2;
}

pub fn DT_str(lexer: &mut Lexer) {
    unsafe {
        lexer.advance(1);
        let mut s = String::new();
        while lexer.peek(0) != b'"' && !lexer.at_eof() {
            s.push(lexer.advance(1) as char);
        }
        if lexer.peek(0) != b'"' {
            panic!("[Lexer] [{}:{}] Unterminated string literal", lexer.loc.0, lexer.loc.1);
        }
        lexer.advance(1);
        lexer.tokStream.push(Token {
            loc: lexer.loc,
            typ: TokenTyp::String(s.clone()),
        });
        lexer.loc.0 += s.len() + 2;
    }
}

impl Lexer {
    pub fn new(file: String) -> Self {
        let buf_n = 4096;
        let file = File::open(file.as_str()).expect("[Lexer] File not found!");
        let file_len = file.metadata().unwrap().len() as usize;
        let first_len = file_len.min(buf_n);
        let mmap = unsafe {
            MmapOptions::new().offset(0).len(first_len).map(&file).unwrap()
        };

        let mmap_bg = unsafe {
            if file_len > buf_n {
                let second_len = (file_len - buf_n).min(buf_n);
                MmapOptions::new().offset(buf_n as u64).len(second_len).map(&file).unwrap()
            } else {
                MmapOptions::new().offset(0).len(first_len.max(1)).map(&file).unwrap()
            }
        };
        let i = mmap.as_ptr();
        let file_at = first_len.min(file_len);
        return Lexer {
            file,
            mmap:[mmap,mmap_bg],
            buf_n,
            file_at,
            mmap_active: 0,
            i,
            loc: (0,0),
            tokStream: vec![],
            idents:HashMap::from([
                ("if".to_string(), TokenTyp::KwIf),
                ("$".to_string(), TokenTyp::KwLet),
                ("nul".to_string(), TokenTyp::KwNul),
                ("mod".to_string(), TokenTyp::Mod),
                ("@use".to_string(), TokenTyp::Directive(DirectiveTyp::Use)),
                ("@from".to_string(), TokenTyp::Directive(DirectiveTyp::From)),
                ("@import".to_string(), TokenTyp::Directive(DirectiveTyp::Import)),
                ("@defer".to_string(), TokenTyp::Directive(DirectiveTyp::Defer)),
                ("@type_cast".to_string(), TokenTyp::Directive(DirectiveTyp::TypCast)),
                ("..".to_string(), TokenTyp::KwBlank),
                ("string".to_string(), TokenTyp::StaticTyp(StaticTyp::Str)),
                static_type!(u 8), static_type!(u 16), static_type!(u 32), static_type!(u 64), static_type!(u 128),
                static_type!(i 8), static_type!(i 16), static_type!(i 32), static_type!(i 64), static_type!(i 128)
            ]),
            idents_n: 0,
            dispatch_table: {
                let mut dt = [DT_default as DT_Handler;256];

                for c in b'0'..=b'9' {
                    dt[c as usize] = DT_numeric;
                }

                for c in (b'a'..=b'z')
                    .chain(b'A'..=b'Z')
                        .chain(b'_'..=b'_')
                        {
                            dt[c as usize] = DT_identifier;
                        }

                for c in 9..=13 {
                    dt[c] = DT_whitespace;
                }

                dt[' '  as usize] = DT_whitespace;
                dt['\0' as usize] = DT_whitespace;
                dt['\n' as usize] = DT_nl;

                dt['*' as usize] = DT_ptr;
                dt['&' as usize] = DT_andp;
                dt['.' as usize] = DT_dot;
                dt['$' as usize] = DT_let;
                dt[':' as usize] = DT_colon;
                dt[';' as usize] = DT_semi_colon;
                dt[',' as usize] = DT_comma;

                dt['{' as usize] = DT_curly_open;
                dt['}' as usize] = DT_curly_close;

                dt['(' as usize] = DT_paren_open;
                dt[')' as usize] = DT_paren_close;

                dt['[' as usize] = DT_bracket_open;
                dt[']' as usize] = DT_bracket_close;

                dt['_' as usize] = DT_wild;
                dt['?' as usize] = DT_question;

                dt['+' as usize] = DT_plus;
                dt['-' as usize] = DT_minus;
                dt['/' as usize] = DT_div;
                dt['~' as usize] = DT_squig;

                dt['%' as usize] = DT_register;
                dt['@' as usize] = DT_directive;

                dt['<' as usize] = DT_le;
                dt['>' as usize] = DT_ge;
                dt['=' as usize] = DT_eq;
                dt['!' as usize] = DT_neq;
                dt['"' as usize] = DT_str;

                dt
            },
            file_len,
            pos: 0,
        }
    }
    fn at_eof(&self) -> bool {
        self.pos >= self.file_len
    }
    fn active_chunk_len(&self) -> usize {
        self.mmap[self.mmap_active as usize].len()
    }
    pub fn peek(&self, peek_by: usize) -> u8 {
        unsafe {
            if self.pos + peek_by >= self.file_len {
                return 0;
            }
            let chunk_end = self.mmap[self.mmap_active as usize]
                .as_ptr()
                .add(self.active_chunk_len());
            if self.i.add(peek_by) < chunk_end {
                *self.i.add(peek_by)
            } else {
                let offset = self
                    .i
                    .add(peek_by)
                    .offset_from(chunk_end);
                *self.mmap[((self.mmap_active + 1) % 2) as usize]
                    .as_ptr()
                    .add(offset as usize)
            }
        }
    }
    pub fn advance(&mut self, adv_by: usize) -> u8 {
        unsafe {
            let out = *self.i;
            self.pos += adv_by;
            let chunk_end = self.mmap[self.mmap_active as usize]
                .as_ptr()
                .add(self.active_chunk_len());
            if self.i.add(adv_by) < chunk_end {
                self.i = self.i.add(adv_by);
            } else if self.file_len > self.buf_n {
                let offset = self.i.add(adv_by).offset_from(chunk_end);
                self.i = self.mmap[((self.mmap_active + 1) % 2) as usize]
                    .as_ptr()
                    .add(offset as usize);
                let remap_len = (self.file_len - self.file_at).min(self.buf_n);
                self.mmap[self.mmap_active as usize] = MmapOptions::new()
                    .offset(self.file_at as u64)
                    .len(remap_len)
                    .map(&self.file)
                    .unwrap();
                self.file_at += remap_len;
                self.mmap_active = (self.mmap_active + 1) % 2;
            }
            out
        }
    }
    pub fn lex(&mut self) {
        while !self.at_eof() {
            unsafe {
                let handler = self.dispatch_table[*self.i as usize];
                handler(self);
            }
        }
    }
} 

