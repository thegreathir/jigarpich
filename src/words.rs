use std::{fs::File, sync::OnceLock};

use rand::{seq::SliceRandom, thread_rng};
use serde_repr::Deserialize_repr;

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
enum Complexity {
    Easy = 1,
    Medium = 2,
    Hard = 3,
}

#[derive(Debug, serde::Deserialize)]
pub struct Word {
    text: String,
    complexity: Complexity,
}

static WORDS: OnceLock<Vec<Word>> = OnceLock::new();

pub fn get_random_word() -> &'static Word {
    WORDS
        .get_or_init(|| {
            let file_path = std::env::args()
                .nth(1)
                .expect("Words CSV file is not provided!");
            let file = File::open(file_path).unwrap();
            csv::Reader::from_reader(file)
                .deserialize::<Word>()
                .map(|w| w.expect("Can not parse word"))
                .collect()
        })
        .choose(&mut thread_rng())
        .expect("Empty CSV file!")
}
