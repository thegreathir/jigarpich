use std::{env, sync::Arc};

use dashmap::DashMap;
use game_model::{get_new_id, GameId, GameState};
use teloxide::{prelude::*, update_listeners::webhooks};

mod game_model;

type JoinTable = Arc<DashMap<UserId, GameId>>;
type StateTable = Arc<DashMap<GameId, GameState>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let bot = Bot::from_env();

    let addr = ([127, 0, 0, 1], 54647).into();
    let url: String = env::var("JIGARPICH_URL").unwrap();
    let url = url.parse().unwrap();
    let listener = webhooks::axum(bot.clone(), webhooks::Options::new(addr, url))
        .await
        .expect("Couldn't setup webhook");

    let join_table: JoinTable = JoinTable::new(DashMap::new());
    let state_table: StateTable = StateTable::new(DashMap::new());

    teloxide::repl_with_listener(
        bot,
        {
            let join_table = join_table.clone();
            let state_table = state_table.clone();
            |bot: Bot, msg: Message| async move {
                bot.send_message(msg.chat.id, get_new_id()).await?;
                Ok(())
            }
        },
        listener,
    )
    .await;
}
