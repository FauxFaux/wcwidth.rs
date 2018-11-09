extern crate reqwest;

use std::cmp;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;
use std::u32;

mod trie;

const EAW_URL: &str = "http://www.unicode.org/Public/UNIDATA/EastAsianWidth.txt";
const UCD_URL: &str = "http://www.unicode.org/Public/UNIDATA/extracted/DerivedGeneralCategory.txt";
const EAW_PROPS: &[&str] = &["W", "F"];
const ZERO_CATS: &[&str] = &["Me", "Mn"];

fn strip_line<'a>(s: &'a str) -> Option<&'a str> {
    let s = s[..s.find('#').unwrap_or(s.len())].trim();
    return if s.len() != 0 { Some(s) } else { None };
}

fn split2<'a>(s: &'a str, sep: &str) -> Option<(&'a str, &'a str)> {
    let mut s = s.splitn(2, sep);
    s.next().and_then(|one| s.next().map(|two| (one, two)))
}

fn parse_range(s: &str) -> Option<(u32, u32)> {
    let (start, end) = split2(s, "..").unwrap_or((s, s));
    Some((u32::from_str_radix(start, 16).unwrap(),
          u32::from_str_radix(end,   16).unwrap()))
}

// can handle eaw and ucd
fn parse_ucd(l: &str, filter: &[&str]) -> Option<(u32, u32)> {
    let l = match strip_line(&l) { Some(l) => l, None => return None };
    let (addrs, details) = split2(l, ";").unwrap();
    let (addrs, details) = (addrs.trim(), details.trim());
    if !filter.iter().any(|p| details.contains(p)) { return None; }
    parse_range(addrs)
}

fn add_range(v: &mut Vec<(u32, u32)>, (start, end): (u32, u32)) {
    if let Some(last) = v.last_mut() {
        if last.1 >= start - 1 { last.1 = cmp::max(last.1, end); return }
    }
    v.push((start, end));
}

fn fetch(url: &str) -> reqwest::Response {
    let res = reqwest::get(url).unwrap();
    assert_eq!(res.status(), 200);
    res
}

fn make_table(url: &str, filter: &[&str]) -> Vec<(u32, u32)> {
    let mut raw: Vec<(u32, u32)> = BufReader::new(fetch(url))
        .lines().filter_map(|l| parse_ucd(&l.unwrap(), filter)).collect();
    raw.sort_by_key(|&(k, _)| k);
    let mut tab = Vec::new();
    for r in raw { add_range(&mut tab, r); }
    tab
}

fn encode(v: i8) -> u64 {
	match v { -1 => 3, 0 => 2, 1 => 0, 2 => 1, _ => panic!("invalid value") }
}

pub fn decode(v: u64) -> i8 {
	match v { 3 => -1, 2 => 0, 0 => 1, 1 => 2, _ => panic!("invalid value") }
}

const overrides: &[(u32, u32, i8)] = &[
	// Control codes
	(0x0000, 0x001F, -1),
	(0x007F, 0x009F, -1),
	// Misc zero width from Cf
	(0x0000, 0x0000, 0),
	(0x200B, 0x200F, 0),
	(0x2028, 0x2029, 0),
	(0x202A, 0x202E, 0),
	(0x2060, 0x2063, 0),
	(0x034F, 0x034F, 0),
];

fn main() {
    let eaw = make_table(EAW_URL, EAW_PROPS);
    let zeros = make_table(UCD_URL, ZERO_CATS);
    use trie::Trie;
    use std::char;
    let mut t = Trie::new();
    for (start, end) in eaw {
        for i in start..end+1 { t.insert(char::from_u32(i).unwrap(), encode(2)); }
    }
    for (start, end) in zeros {
        for i in start..end+1 { t.insert(char::from_u32(i).unwrap(), encode(0)); }
    }
    for &(start, end, v) in overrides {
        for i in start..end+1 { t.insert(char::from_u32(i).unwrap(), encode(v)); }
    }
    let mut f = BufWriter::new(io::stdout());
    writeln!(f, "// generated file\n");
    t.write_width_tables("WIDTH", &mut f);
}
