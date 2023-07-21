use std::collections::{HashMap, HashSet};

use rand::Rng;
use teloxide::types::{User, UserId};

pub fn get_new_id() -> RoomId {
    RoomId(rand::thread_rng().gen_range(10_000..=99_999))
}

pub fn get_team_name(team_id: usize) -> String {
    format!("Team {}", team_id + 1)
}

pub fn get_teams(number_of_teams: usize) -> Vec<String> {
    (0..number_of_teams).map(get_team_name).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(pub u32);

#[derive(Debug)]
pub enum GameLogicError {
    AlreadyJoined,
    NotJoinedToRoom,
    JoinAfterPlay,
    TeamChangeAfterPlay,
}

pub struct NewRoom {
    players: HashMap<UserId, User>,
    number_of_teams: usize,
    teams: Vec<HashSet<UserId>>,
}

impl NewRoom {
    pub fn new(number_of_teams: usize) -> Self {
        NewRoom {
            players: HashMap::new(),
            teams: vec![HashSet::new(); number_of_teams],
            number_of_teams,
        }
    }

    pub fn join(&mut self, user: User) -> Result<(Vec<UserId>, usize), GameLogicError> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.players.entry(user.id) {
            e.insert(user);
            Ok((self.players.keys().cloned().collect(), self.number_of_teams))
        } else {
            Err(GameLogicError::AlreadyJoined)
        }
    }

    pub fn join_team(
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

    pub fn get_teams(&self) -> Result<String, GameLogicError> {
        let result = self.teams
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
            });

        Ok(result)
    }
}

pub struct PlayingTeam {
    first: User,
    second: User,
    time: f32,
    turn: u8,
}

pub struct PlayingRoom {
    teams: Vec<PlayingTeam>,
    turn: u8,
    round: u8,
}

pub enum Room {
    Lobby(NewRoom),
    Playing(PlayingRoom),
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

    pub fn get_teams(&self) -> Result<String, GameLogicError> {
        match self {
            Room::Lobby(lobby) => lobby.get_teams(),
            Room::Playing(_) => todo!(),
        }
    }
}
