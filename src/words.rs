use std::{collections::HashMap, fmt::Display, fs::File, sync::OnceLock};

use rand::{
    distributions::uniform::{UniformFloat, UniformSampler},
    seq::SliceRandom,
    thread_rng,
};
use serde_repr::Deserialize_repr;
use serde_repr::Serialize_repr;

#[derive(Deserialize_repr, Serialize_repr, Debug, Eq, PartialEq, Hash, Clone, Copy)]
#[repr(u8)]
enum Complexity {
    Easy = 1,
    Medium = 2,
    Hard = 3,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Word {
    pub text: String,
    complexity: Complexity,

    #[serde(flatten)]
    taboo_words: HashMap<String, String>,

    #[serde(skip)]
    selected_taboo_words: Vec<String>,
}

impl Display for Word {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string(self).unwrap();
        write!(f, "{}", json)
    }
}

impl Word {
    fn select_taboo_words(other: &Word) -> Word {
        let mut rng = thread_rng();
        let mut taboo_words: Vec<String> = other.taboo_words.values().cloned().collect();
        taboo_words.shuffle(&mut rng);
        let selected_taboo_words = taboo_words.into_iter().take(4).collect();
        Word {
            text: other.text.clone(),
            complexity: other.complexity,
            taboo_words: other.taboo_words.clone(),
            selected_taboo_words,
        }
    }

    pub fn get_message_string(&self, use_taboo_words: bool) -> String {
        let cross = "❌";

        if use_taboo_words {
            let taboo_words = self
                .selected_taboo_words
                .iter()
                .map(|s| format!("{} {}", cross, s))
                .collect::<Vec<String>>()
                .join("\n");

            format!("{}\n\n{}", self.text, taboo_words)
        } else {
            self.text.to_string()
        }
    }
}

static WORDS: OnceLock<HashMap<Complexity, Vec<Word>>> = OnceLock::new();

pub fn get_random_word() -> Word {
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
    let word = match UniformFloat::<f32>::new_inclusive(0.0, 1.0).sample(&mut rng) {
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
    };

    Word::select_taboo_words(word)
}
