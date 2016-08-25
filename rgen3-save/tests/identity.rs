extern crate rgen3_save;
extern crate env_logger;

use rgen3_save::Save;
use std::io::prelude::*;
use std::io::Cursor;
use std::fs::File;

fn cmp(a: &[u8], b: &[u8]) {
    assert_eq!(a.len(), b.len());
    for (i, (&b1, &b2)) in a.iter().zip(b).enumerate() {
        if b1 != b2 {
            panic!("Mismatch: {} (orig) vs {} (new) @ {} (0x{:X})",
                   b1,
                   b2,
                   i,
                   i);
        }
    }
}

#[test]
fn identity() {
    env_logger::init().unwrap();
    let paths = std::env::var("RGEN3_TEST_SAVES")
        .expect("Need RGEN3_TEST_SAVES env var set to a path");
    let paths = paths.split(';');
    for path in paths {
        let mut file = File::open(path).unwrap();
        const SAVE_LEN: usize = 131072;
        // Rust's test runner has small stacks, so gotta use vec instead of array.
        let mut data = vec![0; SAVE_LEN];
        file.read_exact(&mut data[..]).unwrap();
        let mut save = Save::read(&mut Cursor::new(&data[..])).unwrap();
        let mut writeout = vec![0; SAVE_LEN];
        {
            let mut writer = &mut writeout[..];
            save.write(&mut writer).unwrap();
        }
        cmp(&data, &writeout);
    }
}
