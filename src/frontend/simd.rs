use wide::{CmpEq, u8x32};

const CHUNK: usize = 32;

#[inline]
fn in_range(v: u8x32, lo: u8, hi: u8) -> u8x32 {
    let lo_v = u8x32::splat(lo);
    let hi_v = u8x32::splat(hi);
    v.max(lo_v).min(hi_v).cmp_eq(v)
}

fn ident_mask(v: u8x32) -> u32 {
    let dig = in_range(v, b'0', b'9');
    let up = in_range(v, b'A', b'Z');
    let lo = in_range(v, b'a', b'z');
    let und = v.cmp_eq(u8x32::splat(b'_'));
    (dig | up | lo | und).move_mask() as u32
}

fn ws_mask(v: u8x32) -> u32 {
    let sp = v.cmp_eq(u8x32::splat(b' '));
    let tab = v.cmp_eq(u8x32::splat(b'\t'));
    let lf = v.cmp_eq(u8x32::splat(b'\n'));
    let cr = v.cmp_eq(u8x32::splat(b'\r'));
    let nul = v.cmp_eq(u8x32::splat(0));
    let vt = v.cmp_eq(u8x32::splat(0x0b));
    let ff = v.cmp_eq(u8x32::splat(0x0c));
    (sp | tab | lf | cr | nul | vt | ff).move_mask() as u32
}

fn newline_mask(v: u8x32) -> u32 {
    v.cmp_eq(u8x32::splat(b'\n')).move_mask() as u32
}

fn bump_loc_scalar(row: &mut usize, col: &mut usize, bytes: &[u8]) {
    for &b in bytes {
        if b == b'\n' {
            *row += 1;
            *col = 0;
        } else {
            *col += 1;
        }
    }
}

fn bump_loc_chunk(row: &mut usize, col: &mut usize, arr: &[u8; CHUNK]) {
    let lf = newline_mask(u8x32::from(*arr));
    if lf == 0 {
        *col += CHUNK;
        return;
    }
    if lf == u32::MAX {
        *row += CHUNK;
        *col = 0;
        return;
    }
    *row += lf.count_ones() as usize;
    let last_nl = 31 - lf.leading_zeros();
    *col = CHUNK - 1 - last_nl as usize;
}

pub fn bump_loc(row: &mut usize, col: &mut usize, bytes: &[u8]) {
    let mut off = 0;
    while off + CHUNK <= bytes.len() {
        let arr: [u8; CHUNK] = bytes[off..off + CHUNK].try_into().unwrap();
        bump_loc_chunk(row, col, &arr);
        off += CHUNK;
    }
    if off < bytes.len() {
        bump_loc_scalar(row, col, &bytes[off..]);
    }
}

pub unsafe fn scan_ident(ptr: *const u8, max: usize) -> usize {
    let mut off = 0;
    while off + CHUNK <= max {
        let arr = *ptr.add(off).cast::<[u8; CHUNK]>();
        let m = ident_mask(u8x32::from(arr));
        if m == u32::MAX {
            off += CHUNK;
        } else {
            return off + m.trailing_ones() as usize;
        }
    }
    while off < max {
        let b = *ptr.add(off);
        if !(b.is_ascii_alphanumeric() || b == b'_') {
            break;
        }
        off += 1;
    }
    off
}

pub unsafe fn scan_whitespace(ptr: *const u8, max: usize) -> usize {
    let mut off = 0;
    while off + CHUNK <= max {
        let arr = *ptr.add(off).cast::<[u8; CHUNK]>();
        let m = ws_mask(u8x32::from(arr));
        if m == u32::MAX {
            off += CHUNK;
        } else {
            return off + m.trailing_ones() as usize;
        }
    }
    while off < max {
        let b = *ptr.add(off);
        if !matches!(b, 0 | b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
            break;
        }
        off += 1;
    }
    off
}

