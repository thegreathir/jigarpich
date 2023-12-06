use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

use crate::HandlerResult;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Initial,
    ReceiveNumberOfTeams,
    ReceiveNumberOfRounds {
        number_of_teams: u8,
    },
    ReceiveRoundDuration {
        number_of_teams: u8,
        number_of_rounds: u8,
    },
}

pub type MyDialogue = Dialogue<State, InMemStorage<State>>;

fn parse_number(msg: &Message) -> Option<u8> {
    let text = msg.text()?;
    text.parse::<u8>().ok()
}

pub async fn get_number_of_teams(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let Some(number_of_teams) = parse_number(&msg) else {
        bot.send_message(msg.chat.id, "Please send a number!")
            .await?;
        return Ok(());
    };

    if !(2..=7).contains(&number_of_teams) {
        bot.send_message(msg.chat.id, "Number of teams should be between 2 and 7")
            .await?;
        return Ok(());
    }

    dialogue
        .update(State::ReceiveNumberOfRounds { number_of_teams })
        .await?;
    bot.send_message(
        msg.chat.id,
        "How many rounds are you going to play?\n(1 to 7)",
    )
    .await?;
    Ok(())
}

pub async fn get_number_of_rounds(
    bot: Bot,
    dialogue: MyDialogue,
    number_of_teams: u8,
    msg: Message,
) -> HandlerResult {
    let Some(number_of_rounds) = parse_number(&msg) else {
        bot.send_message(msg.chat.id, "Please send a number!")
            .await?;
        return Ok(());
    };

    if !(1..=7).contains(&number_of_rounds) {
        bot.send_message(msg.chat.id, "Number of rounds should be between 1 and 7")
            .await?;
        return Ok(());
    }

    dialogue
        .update(State::ReceiveRoundDuration {
            number_of_teams,
            number_of_rounds,
        })
        .await?;
    bot.send_message(
        msg.chat.id,
        "How long is each round?\n(in minutes up to 10)",
    )
    .await?;
    Ok(())
}

pub async fn get_round_duration(
    bot: Bot,
    (number_of_teams, number_of_rounds): (u8, u8),
    rooms: crate::Rooms,
    msg: Message,
) -> HandlerResult {
    let Some(round_duration) = parse_number(&msg) else {
        bot.send_message(msg.chat.id, "Please send a number!")
            .await?;
        return Ok(());
    };

    if !(1..=10).contains(&round_duration) {
        bot.send_message(
            msg.chat.id,
            "Round duration should be between 1 minute and 10 minutes",
        )
        .await?;
        return Ok(());
    }

    bot.send_message(
        msg.chat.id,
        format!(
            "You are going to play {} rounds with {} teams, each round will last {} minutes.",
            number_of_rounds, number_of_teams, round_duration
        ),
    )
    .await?;

    crate::handle_new_command(
        bot,
        msg,
        rooms,
        number_of_teams as usize,
        number_of_rounds as usize,
        round_duration as usize,
    )
    .await?;

    Ok(())
}
