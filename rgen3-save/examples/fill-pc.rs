extern crate rand;
extern crate rgen3_save;
extern crate rgen3_string;

use rand::{thread_rng, Rng, ThreadRng};
use rgen3_save::{Pokemon, SaveSections, TrainerInfo};
use std::collections::HashSet;

static PREFIX_NAMES: [&'static str; 52] = [
    "Acid", "Axel", "Baal", "Bike", "Bull", "Bald", "Cave", "Diet", "Dray", "Duke", "Easy", "Fact",
    "Face", "Fowl", "Fuzz", "Goat", "Hair", "Head", "Hiss", "Idea", "Iris", "Iron", "Jack", "John",
    "Kart", "Lady", "Limb", "Lime", "Mace", "Mars", "Naga", "Nuke", "Nude", "Omen", "Orca", "Poke",
    "Pyre", "Quiz", "Risk", "Road", "Sock", "Swag", "Teal", "Tree", "Ugly", "Vamp", "Vibe", "Wolf",
    "Xray", "Yoga", "Zeta", "Zoom",
];

static SUFFIX_NAMES: [&'static str; 24] = [
    "Adder", "Baker", "Biter", "Coder", "Curer", "Diver", "Eater", "Faker", "Flier", "Frier",
    "Gamer", "Gazer", "Giver", "Laser", "Lover", "Maker", "Order", "Racer", "Taker", "Tamer",
    "Voter", "Waker", "Zoner", "Hugger",
];

struct PokeGen<'a> {
    chosen_names: HashSet<String>,
    chosen_species: HashSet<u16>,
    n_unique: u16,
    trainer: &'a TrainerInfo,
    rng: ThreadRng,
}

impl<'a> PokeGen<'a> {
    fn new(trainer: &'a TrainerInfo) -> Self {
        PokeGen {
            chosen_names: HashSet::new(),
            chosen_species: HashSet::new(),
            n_unique: 0,
            trainer: trainer,
            rng: thread_rng(),
        }
    }
    fn gen(&mut self) -> Pokemon {
        let mut pokemon = Pokemon::default();
        let mut name;
        loop {
            let prefix = self.rng.choose(&PREFIX_NAMES).unwrap();
            let suffix = self.rng.choose(&SUFFIX_NAMES).unwrap();
            name = format!("{} {}", prefix, suffix);
            // Make sure we don't use the same name twice in our team
            if !self.chosen_names.contains(&name) {
                break;
            }
        }
        rgen3_string::encode_string(&name, &mut pokemon.nickname.0);
        self.chosen_names.insert(name);
        {
            loop {
                let species = self.rng.gen_range(1, 412);
                let result = pokemon.set_species(species);
                if result.is_ok() {
                    if self.chosen_species.contains(&species) {
                        if self.n_unique >= 386 {
                            break;
                        }
                    } else {
                        assert!(self.chosen_species.insert(species));
                        self.n_unique += 1;
                        break;
                    }
                }
            }
            pokemon.data.growth.experience = 1_640_000;
            pokemon.data.growth.friendship = 0xFF;
            pokemon.data.growth.pp_bonuses = 0xFF;
            pokemon.data.attacks.move1 = self.rng.gen_range(0, 354);
            pokemon.data.attacks.move2 = self.rng.gen_range(0, 354);
            pokemon.data.attacks.move3 = self.rng.gen_range(0, 354);
            pokemon.data.attacks.move4 = self.rng.gen_range(0, 354);
            pokemon.data.attacks.pp1 = 99;
            pokemon.data.attacks.pp2 = 99;
            pokemon.data.attacks.pp3 = 99;
            pokemon.data.attacks.pp4 = 99;
            pokemon.data.evs_and_condition.hp = 0xFF;
            pokemon.data.evs_and_condition.attack = 0xFF;
            pokemon.data.evs_and_condition.defense = 0xFF;
            pokemon.data.evs_and_condition.speed = 0xFF;
            pokemon.data.evs_and_condition.sp_attack = 0xFF;
            pokemon.data.evs_and_condition.sp_defense = 0xFF;
            pokemon.data.evs_and_condition.coolness = 0xFF;
            pokemon.data.evs_and_condition.beauty = 0xFF;
            pokemon.data.evs_and_condition.cuteness = 0xFF;
            pokemon.data.evs_and_condition.smartness = 0xFF;
            pokemon.data.evs_and_condition.toughness = 0xFF;
            pokemon.ot_id = self.trainer.full_id();
            pokemon.ot_name = self.trainer.name;
        }
        pokemon
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("Need path to save as first arg");
    let mut save = rgen3_save::Save::load_from_file(&path).unwrap();
    {
        let SaveSections {
            trainer, pc_boxes, ..
        } = save.sections();
        let mut generator = PokeGen::new(trainer);
        for b in pc_boxes.iter_mut() {
            for p in &mut b.pokemon {
                *p = Some(generator.gen());
            }
        }
    }
    save.save_to_file(path).unwrap();
}
