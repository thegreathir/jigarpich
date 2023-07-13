use std::{env, sync::Arc};

use dashmap::DashMap;
use room::{create_team_choice_data, get_new_id, get_teams, Room, RoomId};
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

    let command_handler = Update::filter_message()
        .filter_command::<Command>()
        .endpoint(answer_command);
    let cb_query_handler = Update::filter_callback_query().endpoint(handle_cb_query);

    let handler = dptree::entry().branch(cb_query_handler).branch(command_handler);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![rooms])
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(
            listener,
            LoggingErrorHandler::with_custom_text("An error from the update listener"),
        )
        .await
}

async fn answer_command(bot: Bot, rooms: Rooms, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::New(number_of_teams) => {
            handle_new_command(bot, msg, rooms, number_of_teams).await?;
        }
        Command::Join(room_id) => {
            handle_join_command(bot, msg, rooms, room_id).await?;
        }
    };
    Ok(())
}

async fn handle_cb_query(bot: Bot, rooms: Rooms, q: CallbackQuery) -> ResponseResult<()> {
    println!("Callback query update");
    Ok(())
}

async fn handle_new_command(
    bot: Bot,
    msg: Message,
    rooms: Rooms,
    number_of_teams: usize,
) -> ResponseResult<()> {
    if !(2..=4).contains(&number_of_teams) {
        bot.send_message(msg.chat.id, "Number of teams should be between 2 and 4")
            .await?;
        return Ok(());
    }

    let new_id = get_new_id();
    rooms.insert(new_id, Room::new(number_of_teams));
    bot.send_message(msg.chat.id, "Room created! Forward following message join:")
        .await?;
    bot.send_message(msg.chat.id, format!("/join {}", new_id.0))
        .await?;
    Ok(())
}

async fn handle_join_command(
    bot: Bot,
    msg: Message,
    rooms: Rooms,
    room_id: u32,
) -> ResponseResult<()> {
    if let Some(mut room) = rooms.get_mut(&RoomId(room_id)) {
        if let Some(user) = msg.from() {
            match room.join(user.clone()) {
                Ok((others, number_of_teams)) => {
                    for other in others {
                        bot.send_message(other, format!("{} joined to room", user.full_name()))
                            .await?;
                    }

                    let teams =
                        get_teams(number_of_teams)
                            .into_iter()
                            .enumerate()
                            .map(|(idx, team)| {
                                InlineKeyboardButton::callback(
                                    team,
                                    create_team_choice_data(room_id, idx),
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
    Ok(())
}
