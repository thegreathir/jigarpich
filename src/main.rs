use std::{env, sync::Arc};

use dashmap::DashMap;
use game_model::{get_new_id, GameId, GameState};
use teloxide::{macros::BotCommands, prelude::*, update_listeners::webhooks};

mod game_model;

type RoomTable = Arc<DashMap<UserId, GameId>>;
type StateTable = Arc<DashMap<GameId, GameState>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Create new room")]
    New,
    #[command(description = "Join a room")]
    Join(String),
}

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

    let room_table: RoomTable = RoomTable::new(DashMap::new());
    let state_table: StateTable = StateTable::new(DashMap::new());

    Command::repl_with_listener(
        bot,
        {
            let room_table = room_table.clone();
            let state_table = state_table.clone();
            |bot: Bot, msg: Message, cmd: Command| async move {
                match cmd {
                    Command::New => {
                        let new_id = get_new_id();

                    }
                    Command::Join(_) => {
                        bot.send_message(msg.chat.id, "Join called").await?;
                    }
                };
                Ok(())
            }
        },
        listener,
    )
    .await;
}
