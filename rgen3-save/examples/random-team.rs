extern crate rgen3_save;
extern crate rgen3_string;
extern crate rand;

use rand::{Rng, thread_rng};
use rgen3_save::{Pokemon, SaveSections};
use std::collections::HashSet;

static NAMES: [&'static str; 16] = ["Tubby",
                                    "Chewbacca",
                                    "Huey",
                                    "Dewey",
                                    "Louie",
                                    "Abby",
                                    "Jay Leno",
                                    "Bird Jesus",
                                    "Cutie Pie",
                                    "Saitama",
                                    "Genos",
                                    "Princess",
                                    "Duke Nukem",
                                    "Jaden",
                                    "Goku",
                                    "Vegeta"];

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("Need path to save as first arg");
    let mut save = rgen3_save::Save::load_from_file(&path).unwrap();
    let mut rng = thread_rng();
    {
        let SaveSections { trainer, team } = save.sections();
        let mut chosen_names = HashSet::new();
        team.clear();
        for _ in 0..6 {
            let mut pokemon = Pokemon::default();
            let mut name;
            loop {
                name = rng.choose(&NAMES).unwrap();
                // Make sure we don't use the same name twice in our team
                if chosen_names.get(name).is_none() {
                    break;
                }
            }
            chosen_names.insert(name);
            rgen3_string::encode_string(name, &mut pokemon.nickname.0);
            pokemon.personality = rng.gen();
            pokemon.level = rng.gen_range(80, 100);
            pokemon.total_hp = rng.gen_range(800, 999);
            pokemon.current_hp = pokemon.total_hp;
            pokemon.attack = rng.gen_range(800, 999);
            pokemon.defense = rng.gen_range(800, 999);
            pokemon.speed = rng.gen_range(800, 999);
            pokemon.sp_attack = rng.gen_range(800, 999);
            pokemon.sp_defense = rng.gen_range(800, 999);
            pokemon.data.growth.species = rng.gen_range(0, 386);
            pokemon.data.growth.experience = std::u32::MAX / 2;
            pokemon.data.attacks.move1 = rng.gen_range(0, 354);
            pokemon.data.attacks.move2 = rng.gen_range(0, 354);
            pokemon.data.attacks.move3 = rng.gen_range(0, 354);
            pokemon.data.attacks.move4 = rng.gen_range(0, 354);
            pokemon.data.attacks.pp1 = 99;
            pokemon.data.attacks.pp2 = 99;
            pokemon.data.attacks.pp3 = 99;
            pokemon.data.attacks.pp4 = 99;
            pokemon.ot_id = trainer.full_id();
            pokemon.ot_name = trainer.name;
            team.push(pokemon);
        }
    }
    save.save_to_file(path).unwrap();
}
