//! Fast implementations of wcwidth and wcswidth.
//!
//! See https://www.cl.cam.ac.uk/~mgk25/ucs/wcwidth.c for further information.
//! This package is based on codegen from https://github.com/golang/text/ and
//! the python implementation at https://github.com/jquast/wcwidth.

/// Implements wcwidth as defined in the Single UNIX Specification.
///
/// Returns `Some(0)` for zero-width characters and characters that have no
/// effect on the terminal (like `'\0'`), `None` for control codes, and `Some(1)`
/// or `Some(2)` for printable characters.
pub fn char_width(c: char) -> Option<u8> {
    let mut b = [0u8; 4];
    let (v, _) = lookup_bytes(c.encode_utf8(&mut b).as_bytes());
    if v != -1 { Some(v as u8) } else { None }
}

/// Implements wcswidth.
///
/// Returns `None` if `s` contains any non-printable characters.
pub fn str_width(s: &str) -> Option<usize> {
    let s = s.as_bytes();
    let mut w = 0;
    let mut i = 0;
    while i < s.len() {
        let (v, z) = lookup_bytes(&s[i..]);
        if v == -1 { return None; }
        w += v as usize;
        i += z;
    }
    Some(w)
}

mod tables;

macro_rules! i { ($a:ident[$n:expr, $b:expr]) => ($a[(($n as usize) << 6) + ($b as usize)]) }

#[inline(always)]
fn lookup_bytes(s: &[u8]) -> (i8, usize) {
    use tables::{WIDTH_VALUES, WIDTH_INDEX};
    let c = s[0];
    if c < 0x80 { return (WIDTH_VALUES[c as usize], 1); } // ascii
    let i = WIDTH_INDEX[c as usize];
    if c < 0xE0 { return (i!(WIDTH_VALUES[i, s[1]]), 2); } // 2-byte utf-8
    let i = i!(WIDTH_INDEX[i, s[1]]);
    if c < 0xF0 { return (i!(WIDTH_VALUES[i, s[2]]), 3); } // 3-byte utf-8
    let i = i!(WIDTH_INDEX[i, s[2]]);
    if c < 0xF8 { return (i!(WIDTH_VALUES[i, s[3]]), 4); } // 4-byte utf-8
    panic!("invalid utf-8");
}
