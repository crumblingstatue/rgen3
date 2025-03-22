use std::io::{BufReader, prelude::*};

fn main() {
    for b in BufReader::new(std::io::stdin()).bytes().map(|b| b.unwrap()) {
        print!("{}", rgen3_string::decode_byte(b).to_char());
    }
}
