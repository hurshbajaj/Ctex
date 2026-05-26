use crate::frontend::tokens::{Token, TokenTyp};
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::collections::HashMap;

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
    pub dispatch_table: [DT_Handler; 256]
}

type DT_Handler = fn(&mut Lexer);

pub fn DT_default(lexer: &mut Lexer) { unsafe{
    if lexer.peek(0) == b'\n' {
        lexer.loc.1 += 1;
        lexer.loc.0 = 0;
    } else if lexer.peek(0).is_ascii_whitespace() {
        lexer.loc.0 += 1;
    } else {
        panic!("[Lexer] Unrecognized character \"{}\"", *lexer.i);
    }
    lexer.advance(1);
}}
pub fn DT_numeric(lexer: &mut Lexer) {
    unsafe{
        let mut num = String::new();
        let mut is_float = false;
        while ( lexer.peek(0) >= b'1' && lexer.peek(0) <= b'9' ) || lexer.peek(0) == b',' || lexer.peek(0) == b'.' {
            if lexer.peek(0) != b',' {
                if lexer.peek(0) == b'.' {is_float = !is_float; if is_float == false{panic!("[Lexer] Multiple decimal points within a float not allowed!")}}
                num.push(lexer.peek(0) as char); 
            }
            lexer.advance(1);
        }
        lexer.loc.0 += num.len();
        lexer.tokStream.push(Token{typ: if is_float {TokenTyp::Float(num.parse().unwrap())} else {TokenTyp::Integer(num.parse().unwrap())},loc: lexer.loc});
    }
}

pub fn DT_identifier(lexer: &mut Lexer) { unsafe{
     let mut identifier = String::new();
     if matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' ) {
         identifier.push(lexer.peek(0) as char);
        lexer.advance(1);
        while matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
            identifier.push(lexer.peek(0) as char);        
            lexer.advance(1);
        }
     }

     lexer.loc.0 += identifier.len();
     if let Some(value) = lexer.idents.get(&identifier) {
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: value.clone(),
         });
     } else {
         lexer.idents_n += 1;
         lexer.idents.insert(identifier, TokenTyp::Identifier(lexer.idents_n));
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: TokenTyp::Identifier(lexer.idents_n),
         });
     }
}}

pub fn DT_register(lexer: &mut Lexer) { unsafe{
     let mut register = String::new();
     if matches!(lexer.peek(0), b'%') {
        register.push(lexer.peek(0) as char);
        lexer.advance(1);
        while matches!(lexer.peek(0), b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9') {
            register.push(lexer.peek(0) as char);        
            lexer.advance(1);
        }
     }

     lexer.loc.0 += register.len();
     if let Some(value) = lexer.idents.get(&register) {
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: value.clone(),
         });
     } else {
         lexer.idents_n += 1;
         lexer.idents.insert(register, TokenTyp::Register(lexer.idents_n));
         lexer.tokStream.push(Token {
             loc: lexer.loc,
             typ: TokenTyp::Register(lexer.idents_n),
         });
     }
}}

pub fn DT_ptr(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.loc.0 += 1;
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Ptr});
}

pub fn DT_andp(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.loc.0 += 1;
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Andp});
}

pub fn DT_dot(lexer: &mut Lexer) {
    if lexer.peek(1) != b'.' {
        lexer.advance(1);
        lexer.loc.0 += 1;
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::Dot});
    } else {
        lexer.advance(2);
        lexer.loc.0 += 2;
        lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::KwBlank});
    }
}

pub fn DT_let(lexer: &mut Lexer) {
    lexer.advance(1);
    lexer.loc.0 += 1;
    lexer.tokStream.push(Token{loc: lexer.loc, typ: TokenTyp::KwLet});
}

impl Lexer {
    pub fn new(file: String) -> Self {
        let buf_n = 4096;
        let file = File::open(file.as_str()).expect("[Lexer] File not found!");
        let mmap = unsafe {
            MmapOptions::new().offset(0).len(buf_n).map(&file).unwrap()
        };
        let mmap_bg = unsafe {
            MmapOptions::new().offset(buf_n as u64).len(buf_n).map(&file).unwrap()
        };
        let i = mmap.as_ptr();
        return Lexer {
            file,
            mmap:[mmap,mmap_bg],
            buf_n,
            file_at: buf_n * 2,
            mmap_active: 0,
            i,
            loc: (0,0),
            tokStream: vec![],
            idents:HashMap::from([
                ("if".to_string(), TokenTyp::KwIf),
                ("$".to_string(), TokenTyp::KwLet),
                ("nul".to_string(), TokenTyp::KwNul),
                ("..".to_string(), TokenTyp::KwBlank),
            ]),
            idents_n: 0,
            dispatch_table: {
                let mut dt = [DT_default as DT_Handler;256];
                for c in b'1'..=b'9' {
                    dt[c as usize] = DT_numeric;
                }
                for c in (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'_'..=b'_') {
                    dt[c as usize] = DT_identifier;
                }
                dt['*' as usize] = DT_ptr;
                dt['.' as usize] = DT_dot;
                dt['$' as usize] = DT_let;
                dt['%' as usize] = DT_register;
                dt
            }
        }
    }
    pub fn peek(&self, peek_by:usize) -> u8 {unsafe{
        if self.i.add(peek_by) >= self.mmap[self.mmap_active as usize].as_ptr().add( self.buf_n ) {
            let offset = self.i.add(peek_by).offset_from(self.mmap[self.mmap_active as usize].as_ptr().add( self.buf_n ));
            *(self.mmap[((self.mmap_active + 1) % 2) as usize].as_ptr().add(offset as usize))
        }else{
            *(self.i.add(peek_by))
        }
    }}
    pub fn advance(&mut self, adv_by: usize){unsafe{
        if self.i.add(adv_by) < self.mmap[self.mmap_active as usize].as_ptr().add(self.buf_n){
            self.i = self.i.add(adv_by);
        } else {
            let offset = self.i.add(adv_by).offset_from(self.mmap[self.mmap_active as usize].as_ptr().add( self.buf_n ));
            self.i = self.mmap[((self.mmap_active + 1) % 2) as usize].as_ptr().add(offset as usize);
            self.mmap[self.mmap_active as usize] = MmapOptions::new().offset(self.file_at as u64).len((self.file.metadata().unwrap().len() as usize - self.file_at).min( self.buf_n )).map(&self.file).unwrap();
            self.file_at += (self.file.metadata().unwrap().len() as usize - self.file_at).min( self.buf_n );

            self.mmap_active = (self.mmap_active + 1) % 2;

        }
    }}
    pub fn lex(&self) {

    }
}

