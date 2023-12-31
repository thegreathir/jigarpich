use std::{collections::HashMap, fs::File, sync::OnceLock};

use rand::{
    distributions::uniform::{UniformFloat, UniformSampler},
    seq::SliceRandom,
    thread_rng,
};
use serde_repr::Deserialize_repr;

#[derive(Deserialize_repr, Debug, Eq, PartialEq, Hash, Clone, Copy)]
#[repr(u8)]
enum Complexity {
    Easy = 1,
    Medium = 2,
    Hard = 3,
}

#[derive(Debug, serde::Deserialize)]
pub struct Word {
    pub text: String,
    complexity: Complexity,
}

static WORDS: OnceLock<HashMap<Complexity, Vec<Word>>> = OnceLock::new();

pub fn get_random_word() -> &'static Word {
    let words = WORDS.get_or_init(|| {
        let file_path = std::env::args()
            .nth(1)
            .expect("Words CSV file is not provided!");
        let file = File::open(file_path).unwrap();
        csv::Reader::from_reader(file)
            .deserialize::<Word>()
            .map(|w| w.expect("Can not parse word"))
            .fold(HashMap::new(), |mut res, w| {
                res.entry(w.complexity).or_default().push(w);
                res
            })
    });

    let mut rng = thread_rng();
    match UniformFloat::<f32>::new_inclusive(0.0, 1.0).sample(&mut rng) {
        x if x < 0.7 => words
            .get(&Complexity::Easy)
            .expect("No easy word")
            .choose(&mut rng)
            .unwrap(),
        x if x < 0.9 => words
            .get(&Complexity::Medium)
            .expect("No medium word")
            .choose(&mut rng)
            .unwrap(),
        _ => words
            .get(&Complexity::Hard)
            .expect("No hard word")
            .choose(&mut rng)
            .unwrap(),
    }
}
