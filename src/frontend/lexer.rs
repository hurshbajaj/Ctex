use crate::frontend::keywords::{lookup_directive, lookup_keyword};
use crate::frontend::simd;
use crate::frontend::tokens::{Token, TokenTyp};
use memmap2::{Mmap, MmapOptions};
use std::collections::HashMap;
use std::fs::File;

pub struct Lexer {
    pub mmap: [Mmap; 2],
    chunk_off: [usize; 2],
    pub file: File,
    pub file_at: usize,
    pub buf_n: usize,
    pub mmap_active: u8,
    pub i: *const u8,
    pub row: usize,
    pub col: usize,
    pub tokStream: Vec<Token>,
    pub idents: HashMap<String, TokenTyp>,
    pub idents_n: usize,
    pub dispatch_table: [DT_Handler; 256],
    file_len: usize,
    pos: usize,
    linear: bool,
}

type DT_Handler = fn(&mut Lexer);

impl Lexer {
    fn chunk_slot_for_pos(&self, pos: usize) -> Option<usize> {
        for slot in 0..2 {
            let off = self.chunk_off[slot];
            let len = self.mmap[slot].len();
            if len > 0 && pos >= off && pos < off + len {
                return Some(slot);
            }
        }
        None
    }

    unsafe fn sync_cursor(&mut self) {
        if self.linear {
            self.i = self.mmap[0].as_ptr().add(self.pos);
            self.mmap_active = 0;
            return;
        }
        if self.at_eof() {
            return;
        }
        while self.chunk_slot_for_pos(self.pos).is_none() && self.file_at < self.file_len {
            self.load_next_chunk();
        }
        if self.at_eof() {
            return;
        }
        let slot = self
            .chunk_slot_for_pos(self.pos)
            .expect("[Lexer] cursor position is not mapped");
        self.mmap_active = slot as u8;
        self.i = self
            .mmap[slot]
            .as_ptr()
            .add(self.pos - self.chunk_off[slot]);
    }

    unsafe fn load_next_chunk(&mut self) {
        let end0 = self.chunk_off[0] + self.mmap[0].len();
        let end1 = self.chunk_off[1] + self.mmap[1].len();
        let slot = if end0 <= end1 { 0 } else { 1 };
        let remap_len = (self.file_len - self.file_at).min(self.buf_n);
        if remap_len == 0 {
            return;
        }
        self.mmap[slot] = MmapOptions::new()
            .offset(self.file_at as u64)
            .len(remap_len)
            .map(&self.file)
            .unwrap();
        self.chunk_off[slot] = self.file_at;
        self.file_at += remap_len;
    }

    unsafe fn ensure_mapped(&mut self) {
        if self.linear {
            return;
        }
        self.sync_cursor();
    }

    fn bytes_ahead(&self) -> usize {
        self.file_len.saturating_sub(self.pos)
    }

    fn bump_byte(&mut self, b: u8) {
        if b == b'\n' {
            self.row += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
    }

    fn push_at(&mut self, typ: TokenTyp, col_start: usize) {
        self.tokStream.push(Token {
            typ,
            loc: (self.row, (col_start, self.col)),
        });
    }

    unsafe fn read_slice(&self, len: usize) -> &str {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.i, len))
    }

    unsafe fn contiguous_ahead(&self) -> usize {
        if self.at_eof() {
            return 0;
        }
        if self.linear {
            return self.file_len - self.pos;
        }
        let Some(slot) = self.chunk_slot_for_pos(self.pos) else {
            return 0;
        };
        self.chunk_off[slot] + self.mmap[slot].len() - self.pos
    }

    unsafe fn advance_by(&mut self, adv_by: usize) {
        if adv_by == 0 {
            return;
        }
        if self.linear {
            let slice = std::slice::from_raw_parts(self.i, adv_by);
            simd::bump_loc(&mut self.row, &mut self.col, slice);
            self.pos += adv_by;
            self.i = self.i.add(adv_by);
            return;
        }
        let mut remaining = adv_by;
        while remaining > 0 {
            self.ensure_mapped();
            let step = self.contiguous_ahead().min(remaining);
            if step == 0 {
                let b = self.advance(1);
                self.bump_byte(b);
                remaining -= 1;
                continue;
            }
            let slice = std::slice::from_raw_parts(self.i, step);
            simd::bump_loc(&mut self.row, &mut self.col, slice);
            self.pos += step;
            remaining -= step;
            self.sync_cursor();
        }
    }

    unsafe fn advance_n(&mut self, n: usize) {
        self.advance_by(n);
    }

    unsafe fn scan_ident_word(&mut self) -> String {
        let mut word = String::new();
        loop {
            let max = self.bytes_ahead();
            if max == 0 {
                break;
            }
            let cap = self.contiguous_ahead();
            let n = simd::scan_ident(self.i, cap.min(max));
            if n == 0 {
                break;
            }
            word.push_str(self.read_slice(n));
            self.advance_n(n);
            if n < cap.min(max) {
                break;
            }
            if cap >= max {
                break;
            }
            let b = self.peek(0);
            if !(b.is_ascii_alphanumeric() || b == b'_') {
                break;
            }
        }
        word
    }

    fn intern_with(&mut self, word: String, make: impl FnOnce(usize) -> TokenTyp) -> TokenTyp {
        if let Some(t) = self.idents.get(&word) {
            return t.clone();
        }
        self.idents_n += 1;
        let t = make(self.idents_n);
        self.idents.insert(word, t.clone());
        t
    }

    unsafe fn advance_bytes_one_col(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        let first = self.peek(0);
        if self.linear {
            self.pos += n;
            self.i = self.i.add(n);
        } else {
            self.pos += n;
            self.sync_cursor();
        }
        if first == b'\n' {
            self.row += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
    }

    unsafe fn eat_utf8_scalar(&mut self) -> String {
        let ahead = self.bytes_ahead().min(4);
        if ahead == 0 {
            return String::new();
        }
        let mut buf = [0u8; 4];
        for j in 0..ahead {
            buf[j] = self.peek(j);
        }
        match std::str::from_utf8(&buf[..ahead]) {
            Ok(s) => {
                let n = s.chars().next().map(|c| c.len_utf8()).unwrap_or(1);
                let ch: String = s.chars().take(1).collect();
                self.advance_bytes_one_col(n);
                ch
            }
            Err(e) => {
                let n = e.valid_up_to().max(1).min(ahead);
                let ch = String::from_utf8_lossy(&buf[..n]).into_owned();
                self.advance_bytes_one_col(n);
                ch
            }
        }
    }

    unsafe fn scan_whitespace_run(&mut self) -> usize {
        let mut total = 0usize;
        loop {
            let max = self.bytes_ahead();
            if max == 0 {
                break;
            }
            let cap = self.contiguous_ahead();
            let n = simd::scan_whitespace(self.i, cap.min(max));
            if n == 0 {
                break;
            }
            self.advance_n(n);
            total += n;
            if n < cap.min(max) {
                break;
            }
            if cap >= max {
                break;
            }
            let b = self.peek(0);
            if !matches!(b, 0 | b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
                break;
            }
        }
        total
    }
}

pub fn DT_unknown(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Unknown, col_start);
}

pub fn DT_unicode(lexer: &mut Lexer) {
    unsafe {
        let col_start = lexer.col;
        let row = lexer.row;
        let word = lexer.eat_utf8_scalar();
        if word.is_empty() {
            return;
        }
        let typ = lexer.intern_with(word, TokenTyp::InvalidIdent);
        lexer.tokStream.push(Token {
            typ,
            loc: (row, (col_start, col_start + 1)),
        });
    }
}

pub fn DT_nl(lexer: &mut Lexer) {
    unsafe {
        lexer.advance_n(1);
    }
}

pub fn DT_whitespace(lexer: &mut Lexer) {
    unsafe {
        let n = lexer.scan_whitespace_run();
        if n == 0 {
            lexer.advance_n(1);
        }
    }
}

pub fn DT_numeric(lexer: &mut Lexer) {
    unsafe {
        let col_start = lexer.col;
        let mut num = String::new();
        let mut is_float = false;
        loop {
            let b = lexer.peek(0);
            if b.is_ascii_digit() {
                let b = lexer.advance(1);
                lexer.bump_byte(b);
                num.push(b as char);
            } else if b == b',' || b == b'.' {
                if lexer.peek(1).is_ascii_digit() {
                    if b == b'.' {
                        is_float = true;
                        let b = lexer.advance(1);
                        lexer.bump_byte(b);
                        num.push('.');
                    } else {
                        let b = lexer.advance(1);
                        lexer.bump_byte(b);
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if num.is_empty() {
            return;
        }
        let typ = if is_float {
            match num.parse::<f64>() {
                Ok(v) => TokenTyp::Float(v),
                Err(_) => TokenTyp::Unknown,
            }
        } else {
            match num.parse::<usize>() {
                Ok(v) => TokenTyp::Integer(v),
                Err(_) => TokenTyp::Unknown,
            }
        };
        lexer.push_at(typ, col_start);
    }
}

pub fn DT_identifier(lexer: &mut Lexer) {
    unsafe {
        let col_start = lexer.col;
        let word = lexer.scan_ident_word();
        let typ = if let Some(kw) = lookup_keyword(&word) {
            kw
        } else if let Some(t) = lexer.idents.get(&word) {
            t.clone()
        } else {
            lexer.idents_n += 1;
            let t = TokenTyp::Identifier(lexer.idents_n);
            lexer.idents.insert(word, t.clone());
            t
        };
        lexer.push_at(typ, col_start);
    }
}

pub fn DT_register(lexer: &mut Lexer) {
    unsafe {
        let col_start = lexer.col;
        let b = lexer.advance(1);
        lexer.bump_byte(b);
        let tail = lexer.scan_ident_word();
        let word = format!("%{tail}");
        let typ = if let Some(t) = lexer.idents.get(&word) {
            t.clone()
        } else {
            lexer.idents_n += 1;
            let t = TokenTyp::Register(lexer.idents_n);
            lexer.idents.insert(word, t.clone());
            t
        };
        lexer.push_at(typ, col_start);
    }
}

pub fn DT_ptr(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Ptr, col_start);
}

pub fn DT_andp(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Andp, col_start);
}

pub fn DT_dot(lexer: &mut Lexer) {
    let col_start = lexer.col;
    if lexer.peek(1) != b'.' {
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Dot, col_start);
    } else {
        unsafe {
            lexer.advance_n(2);
        }
        lexer.push_at(TokenTyp::KwBlank, col_start);
    }
}

pub fn DT_let(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::KwLet, col_start);
}

pub fn DT_colon(lexer: &mut Lexer) {
    let col_start = lexer.col;
    if lexer.peek(1) == b':' {
        unsafe {
            lexer.advance_n(2);
        }
        lexer.push_at(TokenTyp::AccessColon, col_start);
    } else {
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Colon, col_start);
    }
}

pub fn DT_semi_colon(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Semicolon, col_start);
}

pub fn DT_comma(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Comma, col_start);
}

pub fn DT_curly_open(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::CurlyOpen, col_start);
}

pub fn DT_curly_close(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::CurlyClose, col_start);
}

pub fn DT_paren_open(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::ParenOpen, col_start);
}

pub fn DT_paren_close(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::ParenClose, col_start);
}

pub fn DT_bracket_open(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::BracketOpen, col_start);
}

pub fn DT_bracket_close(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::BracketClose, col_start);
}

pub fn DT_wild(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Wild, col_start);
}

pub fn DT_question(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Question, col_start);
}

pub fn DT_plus(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Plus, col_start);
}

pub fn DT_mult(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Mult, col_start);
}

pub fn DT_div(lexer: &mut Lexer) {
    if lexer.peek(1) == b'/' {
        unsafe {
            lexer.advance_n(2);
            while !lexer.at_eof() && lexer.peek(0) != b'\n' {
                let b = lexer.advance(1);
                lexer.bump_byte(b);
            }
        }
    } else if lexer.peek(1) == b'*' {
        let col_start = lexer.col;
        unsafe {
            lexer.advance_n(2);
            loop {
                if lexer.at_eof() {
                    lexer.push_at(TokenTyp::UnterminatedMultilineComment, col_start);
                    return;
                }
                if lexer.peek(0) == b'*' && lexer.peek(1) == b'/' {
                    lexer.advance_n(2);
                    break;
                }
                let b = lexer.advance(1);
                lexer.bump_byte(b);
            }
        }
    } else {
        let col_start = lexer.col;
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Div, col_start);
    }
}

pub fn DT_minus(lexer: &mut Lexer) {
    let col_start = lexer.col;
    if lexer.peek(1) != b'>' {
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Minus, col_start);
    } else {
        unsafe {
            lexer.advance_n(2);
        }
        lexer.push_at(TokenTyp::RArrow, col_start);
    }
}

pub fn DT_squig(lexer: &mut Lexer) {
    if lexer.peek(1) == b'>' {
        let col_start = lexer.col;
        unsafe {
            lexer.advance_n(2);
        }
        lexer.push_at(TokenTyp::RArrowSquig, col_start);
    } else if lexer.peek(1) == b'%' {
        let col_start = lexer.col;
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Squig, col_start);
    } else {
        unsafe {
            let col_start = lexer.col;
            let b = lexer.advance(1);
            lexer.bump_byte(b);
            let word = lexer.scan_ident_word();
            let typ = if let Some(t) = lexer.idents.get(&word) {
                t.clone()
            } else {
                lexer.idents_n += 1;
                let t = TokenTyp::MetaString(word.clone());
                lexer.idents.insert(word, t.clone());
                t
            };
            lexer.push_at(typ, col_start);
        }
    }
}

pub fn DT_directive(lexer: &mut Lexer) {
    unsafe {
        let col_start = lexer.col;
        let b = lexer.advance(1);
        lexer.bump_byte(b);
        let tail = lexer.scan_ident_word();
        let word = format!("@{tail}");
        let typ = match lookup_directive(&word) {
            Some(t) => t,
            None => lexer.intern_with(word, TokenTyp::UnrecognizedCompilerDirective),
        };
        lexer.push_at(typ, col_start);
    }
}

pub fn DT_leq(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Leq, col_start);
}

pub fn DT_geq(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Geq, col_start);
}

pub fn DT_le(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Le, col_start);
}

pub fn DT_ge(lexer: &mut Lexer) {
    let col_start = lexer.col;
    unsafe {
        lexer.advance_n(1);
    }
    lexer.push_at(TokenTyp::Ge, col_start);
}

pub fn DT_eq(lexer: &mut Lexer) {
    let col_start = lexer.col;
    if lexer.peek(1) == b'>' {
        unsafe {
            lexer.advance_n(2);
        }
        lexer.push_at(TokenTyp::FatRArrow, col_start);
    } else {
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Eq, col_start);
    }
}

pub fn DT_bang(lexer: &mut Lexer) {
    let col_start = lexer.col;
    if lexer.peek(1) == b'=' {
        unsafe {
            lexer.advance_n(2);
        }
        lexer.push_at(TokenTyp::Neq, col_start);
    } else {
        unsafe {
            lexer.advance_n(1);
        }
        lexer.push_at(TokenTyp::Bang, col_start);
    }
}

pub fn DT_str(lexer: &mut Lexer) {
    unsafe {
        let col_start = lexer.col;
        let b = lexer.advance(1);
        lexer.bump_byte(b);
        let mut s = String::new();
        while lexer.peek(0) != b'"' && !lexer.at_eof() {
            let c = lexer.advance(1);
            lexer.bump_byte(c);
            s.push(c as char);
        }
        if lexer.peek(0) != b'"' {
            lexer.push_at(TokenTyp::UnterminatedString, col_start);
            return;
        }
        let b = lexer.advance(1);
        lexer.bump_byte(b);
        lexer.push_at(TokenTyp::String(s), col_start);
    }
}

impl Lexer {
    pub fn new(file: String) -> Self {
        let buf_n = 4096;
        let file = File::open(file.as_str()).expect("[Lexer] File not found!");
        let file_len = file.metadata().unwrap().len() as usize;
        let linear = file_len <= buf_n;
        let (mmap, mmap_bg, chunk_off, file_at) = if linear {
            let mmap = unsafe {
                MmapOptions::new()
                    .len(file_len.max(1))
                    .map(&file)
                    .unwrap()
            };
            let pad = unsafe { MmapOptions::new().len(1).map(&file).unwrap() };
            (mmap, pad, [0, 0], file_len)
        } else {
            let first_len = file_len.min(buf_n);
            let mmap = unsafe { MmapOptions::new().offset(0).len(first_len).map(&file).unwrap() };
            let (mmap_bg, second_off) = if file_len > buf_n {
                let second_len = (file_len - buf_n).min(buf_n);
                let bg = unsafe {
                    MmapOptions::new()
                        .offset(buf_n as u64)
                        .len(second_len)
                        .map(&file)
                        .unwrap()
                };
                (bg, buf_n)
            } else {
                let bg = unsafe {
                    MmapOptions::new()
                        .offset(0)
                        .len(1)
                        .map(&file)
                        .unwrap()
                };
                (bg, 0)
            };
            let loaded = first_len + if file_len > buf_n { mmap_bg.len() } else { 0 };
            (mmap, mmap_bg, [0, second_off], loaded)
        };
        let i = mmap.as_ptr();
        Lexer {
            file,
            mmap: [mmap, mmap_bg],
            chunk_off,
            buf_n,
            file_at,
            mmap_active: 0,
            linear,
            i,
            row: 0,
            col: 0,
            tokStream: vec![],
            idents: HashMap::new(),
            idents_n: 0,
            dispatch_table: {
                let mut dt = [DT_unknown as DT_Handler; 256];
                for b in 128u8..=255 {
                    dt[b as usize] = DT_unicode;
                }
                for c in b'0'..=b'9' {
                    dt[c as usize] = DT_numeric;
                }
                for c in (b'a'..=b'z').chain(b'A'..=b'Z').chain(b'_'..=b'_') {
                    dt[c as usize] = DT_identifier;
                }
                for c in 9..=13 {
                    dt[c] = DT_whitespace;
                }
                dt[' ' as usize] = DT_whitespace;
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
                dt['!' as usize] = DT_bang;
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

    pub fn peek(&mut self, peek_by: usize) -> u8 {
        unsafe {
            if self.pos + peek_by >= self.file_len {
                return 0;
            }
            if !self.linear {
                self.ensure_mapped();
                let p = self.pos + peek_by;
                if self.chunk_slot_for_pos(p).is_none() {
                    self.load_next_chunk();
                    self.sync_cursor();
                }
            }
            let p = self.pos + peek_by;
            if self.linear {
                *self.i.add(peek_by)
            } else {
                let slot = self
                    .chunk_slot_for_pos(p)
                    .expect("[Lexer] peek position is not mapped");
                self.mmap[slot][p - self.chunk_off[slot]]
            }
        }
    }

    pub fn advance(&mut self, adv_by: usize) -> u8 {
        unsafe {
            if !self.linear {
                self.ensure_mapped();
            }
            let out = *self.i;
            self.pos += adv_by;
            if self.linear {
                self.i = self.i.add(adv_by);
            } else {
                self.sync_cursor();
            }
            out
        }
    }

    pub fn lex(&mut self) {
        while !self.at_eof() {
            unsafe {
                if !self.linear {
                    self.ensure_mapped();
                }
                let handler = self.dispatch_table[*self.i as usize];
                handler(self);
            }
        }
    }
}
