use std::{env, fmt::format, sync::Arc};

use dashmap::DashMap;
use room::{get_new_id, get_teams, Room, RoomId};
use teloxide::{
    macros::BotCommands,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    update_listeners::webhooks,
};

mod room;

type Rooms = Arc<DashMap<RoomId, Room>>;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "Create new room")]
    New(usize),
    #[command(description = "Join a room")]
    Join(u32),
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

    let rooms: Rooms = Rooms::new(DashMap::new());

    Command::repl_with_listener(
        bot,
        move |bot: Bot, msg: Message, cmd: Command| {
            let rooms = rooms.clone();
            async move {
                match cmd {
                    Command::New(number_of_teams) => {
                        if !(2..=4).contains(&number_of_teams) {
                            bot.send_message(
                                msg.chat.id,
                                "Number of teams should be between 2 and 4",
                            )
                            .await?;
                            return Ok(());
                        }

                        let new_id = get_new_id();
                        rooms.insert(new_id, Room::new(number_of_teams));
                        bot.send_message(
                            msg.chat.id,
                            "Room created! Forward following message join:",
                        )
                        .await?;
                        bot.send_message(msg.chat.id, format!("/join {}", new_id.0))
                            .await?;
                    }
                    Command::Join(room_id) => {
                        if let Some(mut room) = rooms.get_mut(&RoomId(room_id)) {
                            if let Some(user) = msg.from() {
                                match room.join(user.clone()) {
                                    Ok((others, number_of_teams)) => {
                                        for other in others {
                                            bot.send_message(
                                                other,
                                                format!("{} joined to room", user.full_name()),
                                            )
                                            .await?;
                                        }

                                        let teams = get_teams(number_of_teams)
                                            .into_iter()
                                            .enumerate()
                                            .map(|(idx, team)| {
                                                InlineKeyboardButton::callback(
                                                    team,
                                                    format!("{} {}", room_id, idx),
                                                )
                                            });

                                        bot.send_message(msg.chat.id, "Choose your team")
                                            .reply_markup(InlineKeyboardMarkup::new([teams]))
                                            .await?;
                                    }
                                    Err(room::GameLogicError::AlreadyJoined) => {
                                        bot.send_message(msg.chat.id, "You've already joined!")
                                            .await?;
                                    }
                                    Err(_) => {}
                                }
                            }
                        } else {
                            bot.send_message(msg.chat.id, "Room number is wrong!")
                                .await?;
                        }
                    }
                };
                Ok(())
            }
        },
        listener,
    )
    .await;
}
