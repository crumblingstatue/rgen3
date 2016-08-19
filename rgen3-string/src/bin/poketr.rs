extern crate rgen3_string;

use std::io::prelude::*;

fn main() {
    for b in std::io::stdin().bytes().map(|b| b.unwrap()) {
        print!("{}", rgen3_string::decode_byte(b).to_char());
    }
}
