extern crate wcwidth;

const N: Option<u8> = None;
const S0: Option<u8> = Some(0);
const S1: Option<u8> = Some(1);
const S2: Option<u8> = Some(2);

const STRING_TESTS: &[(&str, &[Option<u8>], Option<usize>)] = &[
    ("コンニチハ, セカイ!", &[S2, S2, S2, S2, S2, S1, S1, S2, S2, S2, S1], Some(19)),
    ("abc\x00def", &[S1, S1, S1, S0, S1, S1, S1], Some(6)),
    ("\x1b[0m", &[N, S1, S1, S1], None),
    ("--\u{05bf}--", &[S1, S1, S0, S1, S1], Some(4)),
    ("cafe\u{0301}", &[S1, S1, S1, S1, S0], Some(4)),
    ("\u{0410}\u{0488}", &[S1, S0], Some(1)),
    ("\u{1B13}\u{1B28}\u{1B2E}\u{1B44}", &[S1, S1, S1, S1], Some(4)),
];

#[test]
fn strings() {
    for &(s, each, full) in STRING_TESTS {
        for (i, c) in s.chars().map(|c|(c, wcwidth::char_width(c))).enumerate() {
            if c.1 != each[i] {
                panic!("{:?}, {:?}: w ({:?}) != each[{}] ({:?})", s, c.0, c.1, i, each[i]);
            }
        }
        let w = wcwidth::str_width(s);
        if w != full { panic!("{:?}: w ({:?}) != full ({:?})", s, w, full); }
    }
}
