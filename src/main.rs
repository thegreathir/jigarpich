use std::{env, sync::Arc};

use dashmap::DashMap;
use room::{create_team_choice_data, get_new_id, get_teams, parse_team_choice_data, Room, RoomId};
use teloxide::{
    macros::BotCommands,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, User},
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

    let handler = dptree::entry()
        .branch(cb_query_handler)
        .branch(command_handler);

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
    let Some(data) = q.data else {
        return Ok(());
    };

    let Some((room_id, team_index)) = parse_team_choice_data(data) else {
        return Ok(());
    };

    let Some(mut room) = rooms.get_mut(&room_id) else {
        return Ok(());
    };

    handle_team_join(bot, &mut room, q.from, team_index).await?;
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
    let Some(mut room) = rooms.get_mut(&RoomId(room_id)) else {
        bot.send_message(msg.chat.id, "Room number is wrong!")
            .await?;
        return Ok(());
    };
    let Some(user) = msg.from() else {
        return Ok(());
    };
    match room.join(user.clone()) {
        Ok((others, number_of_teams)) => {
            broadcast(others, &bot, format!("{} joined to room", user.full_name())).await?;

            let teams = get_teams(number_of_teams)
                .into_iter()
                .enumerate()
                .map(|(idx, team)| {
                    InlineKeyboardButton::callback(team, create_team_choice_data(room_id, idx))
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
    Ok(())
}

async fn broadcast(
    others: Vec<UserId>,
    bot: &Bot,
    msg: String,
) -> Result<(), teloxide::RequestError> {
    for other in others {
        bot.send_message(other, msg.as_str()).await?;
    }
    Ok(())
}

async fn handle_team_join(
    bot: Bot,
    room: &mut Room,
    user: User,
    team_index: usize,
) -> ResponseResult<()> {
    if let Ok(others) = room.join_team(user.id, team_index) {
        broadcast(
            others,
            &bot,
            format!("{} has joined to Team {}", user.full_name(), team_index + 1),
        )
        .await?;
    }
    Ok(())
}
