use std::collections::HashMap;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum PokeChar {
    /// Printable character
    Print(char),
    /// String terminator
    Term,
    Unmapped,
}

impl PokeChar {
    pub fn to_char(self) -> char {
        if let PokeChar::Print(ch) = self {
            ch
        } else {
            '?'
        }
    }
}

use PokeChar::Print as P;

macro_rules! map {
    ($($nv:expr => $pc:expr,)+) => {
        thread_local! {
            static POKECHAR_TO_ENCODED: HashMap<PokeChar, u8> = {
                let mut map = HashMap::new();
                $(
                    map.insert($pc, $nv);
                )+
                map
            };
            static ENCODED_TO_POKECHAR: HashMap<u8, PokeChar> = {
                let mut map = HashMap::new();
                $(
                    map.insert($nv, $pc);
                )+
                map
            };
        }
    }
}

map! {
    0x00 => P(' '),
    0x05 => P('È'), 0x06 => P('É'), 0x1A => P('è'), 0x1B => P('é'),
    0xA1 => P('0'), 0xA2 => P('1'), 0xA3 => P('2'), 0xA4 => P('3'), 0xA5 => P('4'),
    0xA6 => P('5'), 0xA7 => P('6'), 0xA8 => P('7'), 0xA9 => P('8'), 0xAA => P('9'),
    0xAB => P('!'), 0xAC => P('?'), 0xAD => P('.'), 0xAE => P('-'),
    0xBB => P('A'), 0xBC => P('B'), 0xBD => P('C'), 0xBE => P('D'), 0xBF => P('E'),
    0xC0 => P('F'), 0xC1 => P('G'), 0xC2 => P('H'), 0xC3 => P('I'), 0xC4 => P('J'),
    0xC5 => P('K'), 0xC6 => P('L'), 0xC7 => P('M'), 0xC8 => P('N'), 0xC9 => P('O'),
    0xCA => P('P'), 0xCB => P('Q'), 0xCC => P('R'), 0xCD => P('S'), 0xCE => P('T'),
    0xCF => P('U'), 0xD0 => P('V'), 0xD1 => P('W'), 0xD2 => P('X'), 0xD3 => P('Y'),
    0xD4 => P('Z'),
    0xD5 => P('a'), 0xD6 => P('b'), 0xD7 => P('c'), 0xD8 => P('d'), 0xD9 => P('e'),
    0xDA => P('f'), 0xDB => P('g'), 0xDC => P('h'), 0xDD => P('i'), 0xDE => P('j'),
    0xDF => P('k'), 0xE0 => P('l'), 0xE1 => P('m'), 0xE2 => P('n'), 0xE3 => P('o'),
    0xE4 => P('p'), 0xE5 => P('q'), 0xE6 => P('r'), 0xE7 => P('s'), 0xE8 => P('t'),
    0xE9 => P('u'), 0xEA => P('v'), 0xEB => P('w'), 0xEC => P('x'), 0xED => P('y'),
    0xEE => P('z'),
    0xFF => PokeChar::Term,
}

pub fn decode_byte(value: u8) -> PokeChar {
    ENCODED_TO_POKECHAR.with(|map| map.get(&value).cloned().unwrap_or(PokeChar::Unmapped))
}

pub fn decode_string(poke: &[u8]) -> String {
    poke.iter()
        .cloned()
        .map(decode_byte)
        .take_while(|&pc| pc != PokeChar::Term)
        .map(PokeChar::to_char)
        .collect()
}

pub fn encode_string(src: &str, dst: &mut [u8]) {
    let mut dst_bytes = dst.iter_mut();
    for ch in src.chars().map(|ch| {
        POKECHAR_TO_ENCODED.with(|map| map.get(&PokeChar::Print(ch)).cloned().expect("No mapping"))
    }) {
        *dst_bytes.next().unwrap() = ch;
    }
    while let Some(b) = dst_bytes.next() {
        *b = 0xFF;
    }
}

#[test]
fn test_decode_string() {
    assert_eq!(decode_string(&[0xC2, 0xD9, 0xE0, 0xE0, 0xE3, 0xFF]),
               "Hello");
}
