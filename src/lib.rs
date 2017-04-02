#![no_std]

pub mod tables;

pub fn char_width(c: char) -> i8 {
    let mut b = [0u8; 4];
    let (v, _) = lookup_bytes(c.encode_utf8(&mut b).as_bytes());
    v
}

pub fn str_width(s: &str) -> isize {
    let s = s.as_bytes();
	let mut w = 0;
	let mut i = 0;
	while i < s.len() {
		let (v, z) = lookup_bytes(s);
		if v == -1 { return -1; }
		w += v as isize;
		i += z;
	}
	return w;
}

macro_rules! i { ($a:ident[$n:expr, $b:expr]) => ($a[(($n as usize) << 6) + ($b as usize)]) }

#[inline(always)]
fn lookup_bytes(s: &[u8]) -> (i8, usize) {
    use tables::{WIDTH_VALUES, WIDTH_INDEX};
	let (i, c) = (0, s[0]);
	if c < 0x80 { return (i!(WIDTH_VALUES[i, s[0]]), 1); } // ascii
	let i = i!(WIDTH_INDEX[i, s[0]]);
	if c < 0xE0 { return (i!(WIDTH_VALUES[i, s[1]]), 2); } // 2-byte utf-8
	let i = i!(WIDTH_INDEX[i, s[1]]);
	if c < 0xF0 { return (i!(WIDTH_VALUES[i, s[2]]), 3); } // 3-byte utf-8
	let i = i!(WIDTH_INDEX[i, s[2]]);
	if c < 0xF8 { return (i!(WIDTH_VALUES[i, s[3]]), 4); } // 4-byte utf-8
    panic!("invalid utf-8");
}
