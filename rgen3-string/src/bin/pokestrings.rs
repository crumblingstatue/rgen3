extern crate rgen3_string;

use std::io::prelude::*;

use rgen3_string::PokeChar;

fn main() {
    let mut consecutive_spaces = 0;
    let mut consecutive_terms = 0;
    for b in std::io::stdin().bytes().map(|b| b.unwrap()) {
        match rgen3_string::decode_byte(b) {
            PokeChar::Print(ch) => {
                consecutive_terms = 0;
                // If there are too many consecutive spaces (zero byte), then it's probably
                // not a valid string, so don't print it.
                if ch == ' ' {
                    consecutive_spaces += 1;
                } else if consecutive_spaces > 0 {
                    consecutive_spaces = 0;
                }
                if consecutive_spaces < 10 {
                    print!("{}", ch)
                }
            }
            PokeChar::Term => {
                if consecutive_terms == 0 {
                    println!("");
                }
                consecutive_terms += 1;
            }
            _ => {}
        }
    }
}
