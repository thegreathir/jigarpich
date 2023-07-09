use random_word::Lang;

pub fn get_new_id() -> String {
    format!(
        "{}-{}-{}",
        random_word::gen(Lang::En),
        random_word::gen(Lang::En),
        random_word::gen(Lang::En)
    )

}

#[derive(PartialEq, Eq, Hash)]
pub struct GameId(String);

pub struct GameState {

}
