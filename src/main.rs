use std::{
    collections::{BTreeSet, HashMap},
    env,
    sync::Arc,
    time::Duration,
};

use callback_query_command::{parse_command, serialize_command, CbQueryCommand};
use dashmap::DashMap;
use room::{
    get_new_id, get_team_emoji, get_teams, GameLogicError, Room, RoomId, ROUND_DURATION_IN_SECONDS,
    SKIP_COOL_DOWN_IN_SECONDS,
};
use teloxide::{
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup, User},
    update_listeners::webhooks,
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

mod room;

mod callback_query_command;

mod words;

type Rooms = Arc<DashMap<RoomId, Mutex<Room>>>;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "Display this text")]
    Help,
    #[command(description = "Create new room (the number of teams must be given)")]
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
        Command::Help => {
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?;
        }

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

    let Some(room) = rooms.get(&room_id) else {
        return Ok(());
    };
    let mut room = room.lock().await;

    match command {
        CbQueryCommand::Join { team_index } => {
            handle_team_join(bot, &mut room, q.from, team_index).await?
        }
        CbQueryCommand::GetTeams => handle_get_teams(bot, &room, q.from).await?,
        CbQueryCommand::Play => handle_play(&mut room, room_id, bot, q.from).await?,
        CbQueryCommand::Start => handle_start_round(rooms.clone(), &mut room, room_id, bot).await?,
        CbQueryCommand::Correct => handle_correct(rooms.clone(), &mut room, room_id, bot).await?,
        CbQueryCommand::Skip => handle_skip(rooms.clone(), &mut room, room_id, bot).await?,
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
    rooms.insert(new_id, Mutex::new(Room::new(number_of_teams)));
    bot.send_message(
        msg.chat.id,
        "Room created! Forward following message to join:",
    )
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
    let Some(user) = msg.from() else {
        return Ok(());
    };
    let room_id = RoomId(room_id);
    let Some(room) = rooms.get(&room_id) else {
        bot.send_message(msg.chat.id, "Room number is wrong!")
            .await?;
        return Ok(());
    };

    let mut room = room.lock().await;

    match room.join(user.clone()) {
        Ok((others, number_of_teams)) => {
            broadcast(others, &bot, format!("{} joined room", user.full_name())).await?;

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
            bot.send_message(msg.chat.id, "Game has started. You can't join anymore!")
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
                format!("{} joined {}", user.full_name(), get_team_emoji(team_index)),
            )
            .await?;
        }
        Err(room::GameLogicError::TeamChangeAfterPlay) => {
            bot.send_message(
                user.id,
                "Game has started. You can't change your team anymore!",
            )
            .await?;
        }
        Err(_) => (),
    }
    Ok(())
}

async fn handle_get_teams(bot: Bot, room: &Room, user: User) -> ResponseResult<()> {
    bot.send_message(user.id, room.get_teams()).await?;
    Ok(())
}

async fn handle_play(room: &mut Room, room_id: RoomId, bot: Bot, user: User) -> ResponseResult<()> {
    match room.play() {
        Ok(describing_player) => {
            broadcast(
                room.get_all_players(),
                &bot,
                format!(
                    "Game has started. {} should start the first round!",
                    describing_player.full_name()
                ),
            )
            .await?;

            let sent_message = bot
                .send_message(describing_player.id, "Start round")
                .reply_markup(InlineKeyboardMarkup::new([vec![
                    InlineKeyboardButton::callback(
                        "▶️",
                        serialize_command(room_id, CbQueryCommand::Start),
                    ),
                ]]))
                .await?;

            if room
                .push_to_message_stack(sent_message.chat.id, sent_message.id)
                .is_err()
            {
                log::warn!("Error while pushing to message stack {:?}", room_id);
            }
        }
        Err(GameLogicError::NotBalancedTeams) => {
            bot.send_message(user.id, "Teams are not balanced").await?;
        }
        Err(_) => (),
    }
    Ok(())
}

async fn finish_round(rooms: Rooms, room_id: RoomId, players: Vec<UserId>, bot: Bot) {
    let mut time_alerts = HashMap::new();
    time_alerts.insert(60, "⏱️📢 1 min ❗");
    time_alerts.insert(30, "⏱️📢 30 secs ❗");
    time_alerts.insert(10, "⏱️📢 10 secs ❗");

    time_alerts.into_iter().for_each(|(time, message)| {
        tokio::spawn({
            let bot = bot.clone();
            let players = players.clone();
            async move {
                tokio::time::sleep(Duration::from_secs(ROUND_DURATION_IN_SECONDS as u64 - time))
                    .await;
                if let Err(err) = broadcast(players, &bot, message.to_string()).await {
                    log::warn!("Can not broadcast time alert: {}", err);
                }
            }
        });
    });

    tokio::time::sleep(Duration::from_secs(ROUND_DURATION_IN_SECONDS as u64)).await;
    let Some(room) = rooms.get(
        &room_id) else {
        return;
    };

    let mut room = room.lock().await;

    if let Err(err) = clear_last_buttons(&bot, &room).await {
        log::warn!("Can not clear buttons: {}", err);
    }

    let Ok(round_stop_state) = room.stop_round() else {
        log::warn!("Room in bad state while stopping round {:?}", room_id);
        return;
    };

    match round_stop_state {
        room::RoundStopState::RoundFinished(results, describing_player, round) => {
            if let Err(err) = broadcast(room.get_all_players(), &bot, results).await {
                log::warn!("Can not broadcast results: {}", err);
            }

            if let Err(err) = broadcast(
                room.get_all_players(),
                &bot,
                format!(
                    "Round has finished! {} should start round {}!",
                    describing_player.full_name(),
                    round
                ),
            )
            .await
            {
                log::warn!("Can not broadcast round finished alert: {}", err);
            }

            let sent_message = match bot
                .send_message(describing_player.id, "Start round")
                .reply_markup(InlineKeyboardMarkup::new([vec![
                    InlineKeyboardButton::callback(
                        "▶️",
                        serialize_command(room_id, CbQueryCommand::Start),
                    ),
                ]]))
                .await
            {
                Ok(sent_message) => sent_message,
                Err(err) => {
                    log::warn!("Can not send start round message: {}", err);
                    return;
                }
            };

            if room
                .push_to_message_stack(sent_message.chat.id, sent_message.id)
                .is_err()
            {
                log::warn!("Error while pushing to message stack {:?}", room_id);
            }
        }
        room::RoundStopState::GameFinished(results) => {
            if let Err(err) =
                broadcast(room.get_all_players(), &bot, "Game finished!".to_owned()).await
            {
                log::warn!("Can not broadcast game finished alert: {}", err);
            }
            if let Err(err) = broadcast(room.get_all_players(), &bot, results).await {
                log::warn!("Can not broadcast results: {}", err);
            }
        }
    }
}

async fn handle_start_round(
    rooms: Rooms,
    room: &mut Room,
    room_id: RoomId,
    bot: Bot,
) -> ResponseResult<()> {
    if let Ok(word_guess_try) = room.start_round() {
        send_new_word(rooms.clone(), room, room_id, bot.clone(), word_guess_try).await?;

        tokio::task::spawn({
            let players = room.get_all_players().clone();
            async move {
                finish_round(rooms, room_id, players, bot).await;
            }
        });
    }
    Ok(())
}

async fn clear_last_buttons(bot: &Bot, room: &Room) -> ResponseResult<()> {
    let Ok(Some((chat_id, message_id))) = room.get_message_stack_top() else {
        return Ok(());
    };

    bot.edit_message_reply_markup(chat_id, message_id)
        .reply_markup(InlineKeyboardMarkup::new([[]]))
        .await?;

    Ok(())
}

async fn send_new_word(
    rooms: Rooms,
    room: &mut Room,
    room_id: RoomId,
    bot: Bot,
    word_guess_try: room::WordGuessTry,
) -> ResponseResult<()> {
    clear_last_buttons(&bot, room).await?;
    let sent_message = bot
        .send_message(word_guess_try.describing.id, word_guess_try.word.clone())
        .reply_markup(InlineKeyboardMarkup::new([vec![
            InlineKeyboardButton::callback(
                "✅",
                serialize_command(room_id, CbQueryCommand::Correct),
            ),
        ]]))
        .await?;

    if room
        .push_to_message_stack(sent_message.chat.id, sent_message.id)
        .is_err()
    {
        log::warn!("Error while pushing to message stack {:?}", room_id);
    }

    bot.send_message(word_guess_try.guessing.id, "🤔").await?;
    let mut players = BTreeSet::from_iter(room.get_all_players().into_iter());
    players.remove(&word_guess_try.describing.id);
    players.remove(&word_guess_try.guessing.id);
    broadcast(
        players.into_iter().collect(),
        &bot,
        format!(
            "{} -> {}\n\t{}",
            word_guess_try.describing.full_name(),
            word_guess_try.guessing.full_name(),
            word_guess_try.word.clone()
        ),
    )
    .await?;
    tokio::task::spawn(async move {
        add_skip_button(rooms, room_id, bot, sent_message).await;
    });
    Ok(())
}

async fn add_skip_button(rooms: Rooms, room_id: RoomId, bot: Bot, sent_message: Message) {
    tokio::time::sleep(Duration::from_secs(SKIP_COOL_DOWN_IN_SECONDS as u64)).await;
    let Some(room) = rooms.get(&room_id) else {
        return;
    };
    let room = room.lock().await;
    let Ok(Some((chat_id, message_id))) = room.get_message_stack_top() else {
        return;
    };

    if chat_id != sent_message.chat.id || message_id != sent_message.id {
        return;
    }

    if let Err(err) = bot
        .edit_message_reply_markup(sent_message.chat.id, sent_message.id)
        .reply_markup(InlineKeyboardMarkup::new([vec![
            InlineKeyboardButton::callback(
                "✅",
                serialize_command(room_id, CbQueryCommand::Correct),
            ),
            InlineKeyboardButton::callback("⏩️", serialize_command(room_id, CbQueryCommand::Skip)),
        ]]))
        .await
    {
        log::warn!("Can not add skip button: {:?} {}", room_id, err);
    }
}

async fn handle_correct(
    rooms: Rooms,
    room: &mut Room,
    room_id: RoomId,
    bot: Bot,
) -> ResponseResult<()> {
    if let Ok(word_guess_try) = room.correct() {
        send_new_word(rooms, room, room_id, bot, word_guess_try).await?;
    }
    Ok(())
}

async fn handle_skip(
    rooms: Rooms,
    room: &mut Room,
    room_id: RoomId,
    bot: Bot,
) -> ResponseResult<()> {
    if let Ok(word_guess_try) = room.skip() {
        send_new_word(rooms, room, room_id, bot, word_guess_try).await?;
    }
    Ok(())
}
