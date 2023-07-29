use std::{collections::BTreeSet, env, sync::Arc, time::Duration};

use callback_query_command::{parse_command, serialize_command, CbQueryCommand};
use dashmap::DashMap;
use room::{get_new_id, get_teams, GameLogicError, Room, RoomId};
use teloxide::{
    macros::BotCommands,
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, User},
    update_listeners::webhooks,
};

mod room;

mod callback_query_command;

mod words;

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

    let Some((room_id, command)) = parse_command(data) else {
        return Ok(());
    };

    let Some(mut room) = rooms.get_mut(&room_id) else {
        return Ok(());
    };

    match command {
        CbQueryCommand::Join { team_index } => {
            handle_team_join(bot, &mut room, q.from, team_index).await?
        }
        CbQueryCommand::GetTeams => handle_get_teams(bot, &room, q.from).await?,
        CbQueryCommand::Play => handle_play(bot, &mut room, room_id, q.from).await?,
        CbQueryCommand::Start => handle_start_round(bot, &mut room, room_id).await?,
        CbQueryCommand::Correct => todo!(),
        CbQueryCommand::Skip => todo!(),
    };
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
    let room_id = RoomId(room_id);
    let Some(mut room) = rooms.get_mut(&room_id) else {
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
                    InlineKeyboardButton::callback(
                        team,
                        serialize_command(room_id, CbQueryCommand::Join { team_index: idx }),
                    )
                })
                .collect();

            bot.send_message(msg.chat.id, "Choose your team")
                .reply_markup(InlineKeyboardMarkup::new([
                    teams,
                    vec![InlineKeyboardButton::callback(
                        "Show Teams",
                        serialize_command(room_id, CbQueryCommand::GetTeams),
                    )],
                    vec![InlineKeyboardButton::callback(
                        "Play",
                        serialize_command(room_id, CbQueryCommand::Play),
                    )],
                ]))
                .await?;
        }
        Err(room::GameLogicError::AlreadyJoined) => {
            bot.send_message(msg.chat.id, "You've already joined!")
                .await?;
        }
        Err(room::GameLogicError::JoinAfterPlay) => {
            bot.send_message(msg.chat.id, "Game is started. You can't join anymore!")
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
    match room.join_team(user.id, team_index) {
        Ok(others) => {
            broadcast(
                others,
                &bot,
                format!("{} has joined to Team {}", user.full_name(), team_index + 1),
            )
            .await?;
        }
        Err(room::GameLogicError::TeamChangeAfterPlay) => {
            bot.send_message(
                user.id,
                "Game is started. You can't change your team anymore!",
            )
            .await?;
        }
        Err(_) => (),
    }
    Ok(())
}

async fn handle_get_teams(bot: Bot, room: &Room, user: User) -> ResponseResult<()> {
    if let Ok(teams) = room.get_teams() {
        bot.send_message(user.id, teams).await?;
    }
    Ok(())
}

async fn handle_play(bot: Bot, room: &mut Room, room_id: RoomId, user: User) -> ResponseResult<()> {
    match room.play() {
        Ok(describing_player) => {
            broadcast(
                room.get_all_players(),
                &bot,
                format!(
                    "Game is started. {} should start the first round!",
                    describing_player.full_name()
                ),
            )
            .await?;

            bot.send_message(describing_player.id, "Start round")
                .reply_markup(InlineKeyboardMarkup::new([vec![
                    InlineKeyboardButton::callback(
                        "Start",
                        serialize_command(room_id, CbQueryCommand::Start),
                    ),
                ]]))
                .await?;
        }
        Err(GameLogicError::NotBalancedTeams) => {
            bot.send_message(user.id, "Teams are not balanced").await?;
        }
        Err(_) => (),
    }
    Ok(())
}

async fn handle_start_round(bot: Bot, room: &mut Room, room_id: RoomId) -> ResponseResult<()> {
    if let Ok(word_guess_try) = room.start_round() {
        let sent_word = bot
            .send_message(word_guess_try.describing.id, word_guess_try.word.clone())
            .reply_markup(InlineKeyboardMarkup::new([vec![
                InlineKeyboardButton::callback(
                    "✅",
                    serialize_command(room_id, CbQueryCommand::Correct),
                ),
            ]]))
            .await?;

        bot.send_message(word_guess_try.guessing.id, "Try to guess the word")
            .await?;

        let mut players = BTreeSet::from_iter(room.get_all_players().into_iter());
        players.remove(&word_guess_try.describing.id);
        players.remove(&word_guess_try.guessing.id);

        broadcast(
            players.into_iter().collect(),
            &bot,
            word_guess_try.word.clone(),
        )
        .await?;

        tokio::task::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;

            if (bot
                .edit_message_reply_markup(sent_word.chat.id, sent_word.id)
                .reply_markup(InlineKeyboardMarkup::new([vec![
                    InlineKeyboardButton::callback(
                        "✅",
                        serialize_command(room_id, CbQueryCommand::Correct),
                    ),
                    InlineKeyboardButton::callback(
                        "Skip",
                        serialize_command(room_id, CbQueryCommand::Skip),
                    ),
                ]]))
                .await)
                .is_err()
            {

                // TODO: Log if failed
            }
        });
    }
    Ok(())
}
