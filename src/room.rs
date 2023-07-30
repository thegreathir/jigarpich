use std::{
    collections::{BTreeSet, HashMap, HashSet},
    time::{Duration, Instant},
};

use rand::{seq::SliceRandom, Rng};
use teloxide::types::{ChatId, MessageId, User, UserId};

use crate::words::get_random_word;

const ROUNDS_COUNT: u8 = 7;
pub const ROUND_DURATION_IN_MINUTES: usize = 2;
pub const SKIP_COOL_DOWN_IN_SECONDS: usize = 10;

pub fn get_new_id() -> RoomId {
    RoomId(rand::thread_rng().gen_range(10_000..=99_999))
}

pub fn get_team_name(team_id: usize) -> String {
    format!("Team {}", team_id + 1)
}

pub fn get_teams(number_of_teams: usize) -> Vec<String> {
    (0..number_of_teams).map(get_team_name).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct RoomId(pub u32);

#[derive(Debug)]
pub enum GameLogicError {
    AlreadyJoined,
    NotJoinedToRoom,
    JoinAfterPlay,
    TeamChangeAfterPlay,
    AlreadyPlaying,
    NotBalancedTeams,
    IsNotPlaying,
}

#[derive(Default)]
pub struct NewRoom {
    players: HashMap<UserId, User>,
    number_of_teams: usize,
    teams: Vec<HashSet<UserId>>,
}

impl NewRoom {
    fn new(number_of_teams: usize) -> Self {
        NewRoom {
            players: HashMap::new(),
            teams: vec![HashSet::new(); number_of_teams],
            number_of_teams,
        }
    }

    fn join(&mut self, user: User) -> Result<(Vec<UserId>, usize), GameLogicError> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.players.entry(user.id) {
            e.insert(user);
            Ok((self.players.keys().cloned().collect(), self.number_of_teams))
        } else {
            Err(GameLogicError::AlreadyJoined)
        }
    }

    fn join_team(
        &mut self,
        user_id: UserId,
        team_index: usize,
    ) -> Result<Vec<UserId>, GameLogicError> {
        if !self.players.contains_key(&user_id) {
            Err(GameLogicError::NotJoinedToRoom)
        } else {
            self.teams.iter_mut().for_each(|team| {
                team.remove(&user_id);
            });

            self.teams[team_index].insert(user_id);

            Ok(self.players.keys().cloned().collect())
        }
    }

    fn get_teams(&self) -> String {
        self.teams
            .iter()
            .enumerate()
            .fold("".to_owned(), |mut res, (i, members)| {
                res += &format!("{}:\n", get_team_name(i));

                res += &members.iter().fold("".to_owned(), |mut res, member| {
                    if let Some(player) = self.players.get(member) {
                        res += &format!("\t- {}\n", player.full_name());
                    }

                    res
                });

                res
            })
    }

    fn check_teams_ready(&self) -> Result<(), GameLogicError> {
        if self
            .teams
            .iter()
            .fold(BTreeSet::<usize>::new(), |mut res, members| {
                res.insert(members.len());
                res
            })
            .into_iter()
            .collect::<Vec<_>>()
            != vec![2]
        {
            return Err(GameLogicError::NotBalancedTeams);
        }

        Ok(())
    }
}

struct PlayingTeam {
    first: User,
    second: User,
    time: Duration,
    turn: u8,
}

impl PlayingTeam {
    fn get_describing_player(&self) -> User {
        if self.turn == 0 {
            self.first.clone()
        } else {
            self.second.clone()
        }
    }
    fn get_guessing_player(&self) -> User {
        if self.turn == 0 {
            self.second.clone()
        } else {
            self.first.clone()
        }
    }

    fn advance_turn(&mut self) {
        if self.turn == 0 {
            self.turn = 1;
        } else {
            self.turn = 0;
        }
    }

    fn update_time(&mut self, instant: Instant) {
        self.time += Instant::now() - instant;
    }
}

pub struct PlayingRoom {
    teams: Vec<PlayingTeam>,
    turn: u8,
    round: u8,
    instant: Instant,
    message_stack: Vec<(ChatId, MessageId)>,
}

impl PlayingRoom {
    fn from(lobby: NewRoom) -> PlayingRoom {
        let mut rng = rand::thread_rng();
        let mut teams = lobby
            .teams
            .into_iter()
            .map(|team| {
                let team: Vec<_> = team.into_iter().collect();
                PlayingTeam {
                    first: lobby.players.get(team.get(0).unwrap()).unwrap().to_owned(),
                    second: lobby.players.get(team.get(1).unwrap()).unwrap().to_owned(),
                    time: Duration::from_secs(0),
                    turn: 0,
                }
            })
            .collect::<Vec<_>>();
        teams.shuffle(&mut rng);
        PlayingRoom {
            teams,
            turn: 0,
            round: 0,
            instant: Instant::now(),
            message_stack: Vec::new(),
        }
    }

    fn get_describing_player(&self) -> User {
        self.teams[self.turn as usize].get_describing_player()
    }

    fn get_guessing_player(&self) -> User {
        self.teams[self.turn as usize].get_guessing_player()
    }

    fn next(&mut self) {
        self.update_time();
        self.teams[self.turn as usize].advance_turn();
        self.turn += 1;
        self.turn %= self.teams.len() as u8;
    }

    fn update_time(&mut self) {
        self.teams[self.turn as usize].update_time(self.instant);
    }

    fn get_teams(&self) -> String {
        self.teams
            .iter()
            .enumerate()
            .fold("".to_owned(), |mut res, (i, team)| {
                res += &format!(
                    "{}:\n\t- {}\n\t- {}\n\t⏱️ {:.2}\n",
                    get_team_name(i),
                    team.first.full_name(),
                    team.second.full_name(),
                    team.time.as_secs_f32()
                );
                res
            })
    }
}

pub enum Room {
    Lobby(NewRoom),
    Playing(PlayingRoom),
}

pub struct WordGuessTry {
    pub word: String,
    pub describing: User,
    pub guessing: User,
}

pub enum RoundStopState {
    RoundFinished(String, User, u8),
    GameFinished(String),
}

impl Room {
    pub fn new(number_of_teams: usize) -> Self {
        Room::Lobby(NewRoom::new(number_of_teams))
    }

    pub fn join(&mut self, user: User) -> Result<(Vec<UserId>, usize), GameLogicError> {
        match self {
            Room::Lobby(lobby) => lobby.join(user),
            Room::Playing(_) => Err(GameLogicError::JoinAfterPlay),
        }
    }

    pub fn join_team(
        &mut self,
        user_id: UserId,
        team_index: usize,
    ) -> Result<Vec<UserId>, GameLogicError> {
        match self {
            Room::Lobby(lobby) => lobby.join_team(user_id, team_index),
            Room::Playing(_) => Err(GameLogicError::TeamChangeAfterPlay),
        }
    }

    pub fn get_teams(&self) -> String {
        match self {
            Room::Lobby(lobby) => lobby.get_teams(),
            Room::Playing(playing) => playing.get_teams(),
        }
    }

    fn get_playing(&self) -> Result<&PlayingRoom, GameLogicError> {
        match self {
            Room::Lobby(_) => Err(GameLogicError::IsNotPlaying),
            Room::Playing(playing) => Ok(playing),
        }
    }

    fn get_playing_mut(&mut self) -> Result<&mut PlayingRoom, GameLogicError> {
        match self {
            Room::Lobby(_) => Err(GameLogicError::IsNotPlaying),
            Room::Playing(playing) => Ok(playing),
        }
    }

    pub fn get_all_players(&self) -> Vec<UserId> {
        match self {
            Room::Lobby(lobby) => lobby.players.clone().into_keys().collect::<Vec<_>>(),
            Room::Playing(playing) => playing
                .teams
                .iter()
                .map(|team| vec![team.first.id, team.second.id])
                .collect::<Vec<Vec<_>>>()
                .concat(),
        }
    }

    pub fn play(&mut self) -> Result<User, GameLogicError> {
        match self {
            Room::Lobby(new_game) => {
                new_game.check_teams_ready()?;

                let playing = PlayingRoom::from(std::mem::take(new_game));

                *self = Room::Playing(playing);

                Ok(self.get_playing().unwrap().get_describing_player())
            }
            Room::Playing(_) => Err(GameLogicError::AlreadyPlaying),
        }
    }

    pub fn start_round(&mut self) -> Result<WordGuessTry, GameLogicError> {
        let playing = self.get_playing_mut()?;

        playing.instant = Instant::now();

        Ok(WordGuessTry {
            word: get_random_word().text.clone(),
            describing: playing.get_describing_player(),
            guessing: playing.get_guessing_player(),
        })
    }

    pub fn correct(&mut self) -> Result<WordGuessTry, GameLogicError> {
        let playing = self.get_playing_mut()?;

        playing.next();
        playing.instant = Instant::now();

        Ok(WordGuessTry {
            word: get_random_word().text.clone(),
            describing: playing.get_describing_player(),
            guessing: playing.get_guessing_player(),
        })
    }

    pub fn skip(&self) -> Result<WordGuessTry, GameLogicError> {
        let playing = self.get_playing()?;

        Ok(WordGuessTry {
            word: get_random_word().text.clone(),
            describing: playing.get_describing_player(),
            guessing: playing.get_guessing_player(),
        })
    }

    pub fn push_to_message_stack(
        &mut self,
        chat_id: ChatId,
        message_id: MessageId,
    ) -> Result<(), GameLogicError> {
        let playing = self.get_playing_mut()?;
        playing.message_stack.push((chat_id, message_id));
        Ok(())
    }

    pub fn get_message_stack_top(&self) -> Result<Option<(ChatId, MessageId)>, GameLogicError> {
        let playing = self.get_playing()?;
        Ok(playing.message_stack.last().copied())
    }

    pub fn stop_round(&mut self) -> Result<RoundStopState, GameLogicError> {
        let playing = self.get_playing_mut()?;
        playing.update_time();

        let results = playing.get_teams();

        playing.round += 1;
        if playing.round == ROUNDS_COUNT {
            Ok(RoundStopState::GameFinished(results))
        } else {
            Ok(RoundStopState::RoundFinished(
                results,
                playing.get_describing_player(),
                playing.round + 1,
            ))
        }
    }
}
