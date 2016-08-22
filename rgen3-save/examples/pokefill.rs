extern crate rgen3_save;
extern crate rgen3_string;
extern crate rand;

use rand::{Rng, thread_rng};
use rgen3_save::{Pokemon, SaveSections, TrainerInfo};
use std::collections::HashSet;

static PREFIX_NAMES: [&'static str; 52] =
    ["Acid", "Axel", "Baal", "Bike", "Bull", "Bald", "Cave", "Diet", "Dray", "Duke", "Easy",
     "Fact", "Face", "Fowl", "Fuzz", "Goat", "Hair", "Head", "Hiss", "Idea", "Iris", "Iron",
     "Jack", "John", "Kart", "Lady", "Limb", "Lime", "Mace", "Mars", "Naga", "Nuke", "Nude",
     "Omen", "Orca", "Poke", "Pyre", "Quiz", "Risk", "Road", "Sock", "Swag", "Teal", "Tree",
     "Ugly", "Vamp", "Vibe", "Wolf", "Xray", "Yoga", "Zeta", "Zoom"];

static SUFFIX_NAMES: [&'static str; 23] = ["Adder", "Baker", "Biter", "Coder", "Curer", "Diver",
                                           "Eater", "Faker", "Flier", "Frier", "Gamer", "Gazer",
                                           "Giver", "Laser", "Lover", "Maker", "Order", "Racer",
                                           "Taker", "Tamer", "Voter", "Waker", "Zoner"];

fn gen_pokemon<R: Rng>(rng: &mut R,
                       chosen_names: &mut HashSet<String>,
                       trainer: &TrainerInfo)
                       -> Pokemon {
    let mut pokemon = Pokemon::default();
    let mut name;
    loop {
        let prefix = rng.choose(&PREFIX_NAMES).unwrap();
        let suffix = rng.choose(&SUFFIX_NAMES).unwrap();
        name = format!("{} {}", prefix, suffix);
        // Make sure we don't use the same name twice in our team
        if !chosen_names.contains(&name) {
            break;
        }
    }
    rgen3_string::encode_string(&name, &mut pokemon.nickname.0);
    chosen_names.insert(name);
    {
        if pokemon.active_data.is_none() {
            pokemon.active_data = Some(Default::default());
        }
        let active_data = pokemon.active_data.as_mut().unwrap();
        pokemon.personality = rng.gen();
        active_data.level = rng.gen_range(80, 100);
        active_data.total_hp = rng.gen_range(800, 999);
        active_data.current_hp = active_data.total_hp;
        active_data.attack = rng.gen_range(800, 999);
        active_data.defense = rng.gen_range(800, 999);
        active_data.speed = rng.gen_range(800, 999);
        active_data.sp_attack = rng.gen_range(800, 999);
        active_data.sp_defense = rng.gen_range(800, 999);
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
    }
    pokemon
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("Need path to save as first arg");
    let mut save = rgen3_save::Save::load_from_file(&path).unwrap();
    let mut rng = thread_rng();
    {
        let SaveSections { trainer, team, pc_boxes } = save.sections();
        let mut chosen_names = HashSet::new();
        team.clear();
        for _ in 0..6 {
            team.push(gen_pokemon(&mut rng, &mut chosen_names, trainer));
        }
        for b in pc_boxes.iter_mut() {
            for p in &mut b.pokemon {
                *p = Some(gen_pokemon(&mut rng, &mut chosen_names, trainer));
            }
        }
    }
    save.save_to_file(path).unwrap();
}
