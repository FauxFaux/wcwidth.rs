use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher as DHasher;
use std::hash::Hasher;

use std::io::prelude::*;

const BLOCK_SIZE: usize = 64;

// Subtract two blocks to offset 0x80, the first continuation byte.
const BLOCK_OFFSET: usize = 2;

// Subtract three blocks to offset 0xC0, the first non-ASCII starter.
const ROOT_BLOCK_OFFSET: usize = 3;

// Builder builds a set of tries for associating values with runes. The set of
// tries can share common index and value blocks.
#[derive(Clone)]
struct Builder {
    value_type: &'static str,
    index_type: &'static str,

    trie: Trie,

    index_blocks: Vec<Rc<RefCell<Trie>>>,
    value_blocks: Vec<Vec<u64>>,

    index_block_idx: HashMap<u64, usize>,
    value_block_idx: HashMap<u64, usize>,
}

// Trie is a node of the intermediate trie structure.
#[derive(Clone, Debug)]
pub struct Trie {
    children: Option<Vec<Option<Rc<RefCell<Trie>>>>>,
    values: Vec<u64>,
    index: usize,
}

impl Trie {
    pub fn new() -> Trie {
        Trie {
            children: Some(vec![None; BLOCK_SIZE]),
            values: vec![0; 0x80], // utf8.RuneSelf
            index: 0,
        }
    }

    fn empty() -> Trie {
        Trie { children: None, values: vec![], index: 0 }
    }

    // Insert associates value with the given rune. Insert will panic if a non-zero
    // value is passed for an invalid rune.
    pub fn insert(&mut self, r: char, value: u64) {
        if value == 0 { return; } 

        let s = r.to_string();
        let mut s = s.as_bytes();
        if s.len() == 1 { // ascii
            self.values[s[0] as usize] = value;
            return;
        }

        let root = Rc::new(RefCell::new(self.clone()));
        let mut n = root.clone();
        while s.len() > 1 {
            if n.borrow().children.is_none() {
                n.borrow_mut().children = Some(vec![None; BLOCK_SIZE]);
            }
            let p = (s[0] as usize) % BLOCK_SIZE;
            let mut c = n.borrow().children.as_ref().unwrap()[p].clone();
            if c.is_none() {
                c = Some(Rc::new(RefCell::new(Trie::empty())));
                n.borrow_mut().children.as_mut().unwrap()[p] = c.clone();
            }
            n = c.unwrap();
            if s.len() > 2 && n.borrow().values.len() != 0 {
                panic!("triegen: insert({}): found internal node with values", r)
            }
            s = &s[1..];
        }
        if n.borrow().values.len() == 0 { n.borrow_mut().values = vec![0; BLOCK_SIZE]; }
        n.borrow_mut().values[s[0] as usize - 0x80] = value;
        self.clone_from(&*root.borrow());
    }

    pub fn write_width_tables<'a, W: Write>(self, name: &'a str, f: &mut W) {
        use std::iter::{self, FromIterator};
        let mut ibs = Vec::new();
        for _ in 0..3 { ibs.push(Rc::new(RefCell::new(Trie::empty()))); }
        let mut b = Builder {
            value_type: "",
            index_type: "",
            trie: self,
            index_blocks: ibs,
            value_blocks: Vec::new(),
            index_block_idx: FromIterator::from_iter([(0, 0)].iter().map(|&x|x)),
            value_block_idx: FromIterator::from_iter([(0, 0)].iter().map(|&x|x)),
        };
        b.build();
        writeln!(f, "pub const {}_VALUES: &[i8] = &[", name);
        let mut w = 0;
        for (i, v) in b.value_blocks.iter().enumerate() {
            for (j, &x) in v.iter().enumerate() {
                let x = super::decode(x);
                let s = format!("{},", x);
                if w + s.len() > 100 { w = 0; write!(f, "\n"); }
                w += s.len();
                f.write(s.as_bytes());
            }
        }
        writeln!(f, "\n];\n");
        writeln!(f, "pub const {}_INDEX: &[{}] = &[", name, b.index_type);
        let mut w = 0;
        for (i, c) in b.index_blocks.iter().enumerate() {
            for (j, x) in c.borrow().children.as_ref()
                    .unwrap_or(&vec![None; BLOCK_SIZE])
                    .iter().enumerate() {
                let s = format!("{},", x.clone().map_or(0, |x| x.borrow().index));
                if w + s.len() > 100 { w = 0; write!(f, "\n"); }
                w += s.len();
                f.write(s.as_bytes());
            }
        }
        writeln!(f, "\n];");
    }
}

impl Builder {
    fn build(&mut self) {
        // Compute the sizes of the values.
        let vmax = max_value(&Some(Rc::new(RefCell::new(self.trie.clone()))), 0);
        self.value_type = get_int_type(vmax);

        self.value_blocks.push(self.trie.values[..BLOCK_SIZE].to_vec());
        self.value_blocks.push(self.trie.values[BLOCK_SIZE..].to_vec());
        self.value_blocks.push(vec![0; BLOCK_SIZE]);

        let rc = Rc::new(RefCell::new(self.trie.clone()));
        self.compute_offsets(rc.clone(), true);
        self.trie.clone_from(&*rc.borrow());

        let mut imax = 0;
        for ib in &self.index_blocks {
            let x = ib.borrow().index as u64;
            if x > imax { imax = x; }
        }
        self.index_type = get_int_type(imax);
    }

    fn compute_offsets(&mut self, n: Rc<RefCell<Trie>>, root: bool) -> u64 {
        // For the first trie, the root lookup block will be at position 3, which is
        // the offset for UTF-8 non-ASCII starter bytes.
        let first = self.index_blocks.len() == ROOT_BLOCK_OFFSET;
        if first { self.index_blocks.push(n.clone()); }

        // We special-case the cases where all values recursively are 0. This allows
        // for the use of a zero block to which all such values can be directed.
        let mut hasher = DHasher::new();
        for c in n.borrow().children.as_ref().unwrap_or(&vec![]).clone() {
            let v = match c {
                Some(ref n) => self.compute_offsets(n.clone(), false),
                None => 0,
            };
            hasher.write_u64(v);
        }
        for &v in &n.borrow().values { hasher.write_u64(v); }
        let hash = hasher.finish();

        if first { self.index_block_idx.insert(hash, ROOT_BLOCK_OFFSET - BLOCK_OFFSET); }

        // Compacters don't apply to internal nodes.
        if n.borrow().children.is_some() {
            let v = match self.index_block_idx.get(&hash) {
                Some(&v) => v,
                None => {
                    let v = self.index_blocks.len() - BLOCK_OFFSET;
                    self.index_blocks.push(n.clone());
                    self.index_block_idx.insert(hash, v);
                    v
                },
            };
            n.borrow_mut().index = v;
        } else {
            let v = match self.value_block_idx.get(&hash) {
                Some(&v) => v,
                None => {
                    let v = self.value_blocks.len() - BLOCK_OFFSET;
                    self.value_blocks.push(n.borrow().values.clone());
                    self.value_block_idx.insert(hash, v);
                    v
                },
            };
            n.borrow_mut().index = v;
        }

        return hash;
    }
}

fn max_value(n: &Option<Rc<RefCell<Trie>>>, mut max: u64) -> u64 {
    if n.is_none() { return max; }
    let n = n.clone().unwrap();
    if n.borrow().children.is_some() {
        for c in n.borrow().children.as_ref().unwrap() {
            max = max_value(&c, max);
        }
    }
    if n.borrow().values.len() != 0 {
        for &v in &n.borrow().values {
            if max < v { max = v; }
        }
    }
    return max;
}

fn get_int_type(v: u64) -> &'static str {
    if v < 1 << 8 { return "u8"; }
    if v < 1 << 16 { return "u16"; }
    if v < 1 << 32 { return "u32"; }
    return "u64";
}
