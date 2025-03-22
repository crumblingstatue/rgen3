use rgen3_save::{Pokemon, SaveSections};
use rgen3_string::decode_string;
use std::collections::HashMap;

macro_rules! make_poke_map {
    ($($id:literal,$name:literal,$($kind:literal,)+;)+) => {{
        let mut map = HashMap::new();
        $(
            let mut kinds = Vec::new();
            $(
                kinds.push($kind);
            )+
            map.insert($id, ($name, kinds));
        )+
        map
    }}
}

#[allow(clippy::zero_prefixed_literal)]
fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("Need path to save as first arg");
    let save = rgen3_save::Save::load_from_file(&path).unwrap();
    let SaveSections { team, pc_boxes, .. } = save.sections();
    let pokemap = include!("../../poke.incl");
    println!("== Team ==");
    for pokemon in team {
        print_pokemon(pokemon, &pokemap);
    }
    println!("== PC ==");
    for pbox in pc_boxes {
        for slot in &pbox.slots {
            if let Some(pokemon) = slot {
                print_pokemon(pokemon, &pokemap);
            }
        }
    }
}

fn print_pokemon(pokemon: &Pokemon, pokemap: &HashMap<u16, (&'static str, Vec<&'static str>)>) {
    let unk_string: String;
    let unk_vec = vec![""];
    let (name, kinds) = match pokemap.get(&pokemon.data.growth.species) {
        Some((name, kinds)) => (*name, kinds),
        None => {
            unk_string = format!("<Unknown> ({})", pokemon.data.growth.species);
            (&unk_string[..], &unk_vec)
        }
    };
    let data = &pokemon.data;
    let evc = &data.evs_and_condition;
    println!(
        "{} <{}> {:?}",
        decode_string(&pokemon.nickname.0),
        name,
        kinds
    );
    println!("friendship: {}", data.growth.friendship);
    println!("EVs:");
    println!("hp: {}", evc.hp);
    println!("atk: {}", evc.attack);
    println!("def: {}", evc.defense);
    println!("spd: {}", evc.speed);
    println!("sp atk: {}", evc.sp_attack);
    println!("sp def: {}", evc.sp_defense);
    println!(
        "total: {}/510",
        u16::from(evc.hp)
            + u16::from(evc.attack)
            + u16::from(evc.defense)
            + u16::from(evc.speed)
            + u16::from(evc.sp_attack)
            + u16::from(evc.sp_defense)
    );
    println!("-")
}
