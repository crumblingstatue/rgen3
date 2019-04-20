extern crate env_logger;
extern crate rgen3_save;

use rgen3_save::{Pokemon, Save, SaveSectionsMut};
use std::fs::File;
use std::io::prelude::*;
use std::io::Cursor;

const SAVE_LEN: usize = 131072;

fn cmp(orig: &[u8], new: &[u8]) {
    assert_eq!(orig.len(), new.len());
    for (i, (&b1, &b2)) in orig.iter().zip(new).enumerate() {
        if b1 != b2 {
            panic!(
                "Mismatch: {} (orig) vs {} (new) @ {} (0x{:X})",
                b1, b2, i, i
            );
        }
    }
}

fn run_test<F: Fn(&[u8], Save)>(test: F) {
    let _ = env_logger::init();
    let paths =
        std::env::var("RGEN3_TEST_SAVES").expect("Need RGEN3_TEST_SAVES env var set to a path");
    let paths = paths.split(';');
    for path in paths {
        let mut file = File::open(path).unwrap();
        // Rust's test runner has small stacks, so gotta use vec instead of array.
        let mut data = vec![0; SAVE_LEN];
        file.read_exact(&mut data[..]).unwrap();
        let save = Save::read(&mut Cursor::new(&data[..])).unwrap();
        test(&data, save);
    }
}

#[test]
fn no_change() {
    run_test(|data, mut save| {
        let mut writeout = vec![0; SAVE_LEN];
        {
            let mut writer = &mut writeout[..];
            save.write(&mut writer).unwrap();
        }
        cmp(data, &writeout);
    })
}

#[test]
fn pc_fill() {
    run_test(|_, mut save| {
        {
            let SaveSectionsMut { pc_boxes, .. } = save.sections_mut();
            for b in pc_boxes.iter_mut() {
                for p in &mut b.pokemon {
                    let mut poke = Pokemon::default();
                    poke.nickname.0[0] = 0x01;
                    poke.ot_name.0[0] = 0x01;
                    *p = Some(poke);
                }
            }
        }
        let mut data_1 = vec![0; SAVE_LEN];
        {
            let mut writer = &mut data_1[..];
            save.write(&mut writer).unwrap();
        }
        let mut save_2 = Save::read(&mut Cursor::new(&data_1)).unwrap();
        let mut data_2 = vec![0; SAVE_LEN];
        {
            let mut writer = &mut data_2[..];
            save_2.write(&mut writer).unwrap();
        }
        cmp(&data_1, &data_2);
    })
}
