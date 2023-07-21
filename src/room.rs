use std::collections::{BTreeSet, HashMap, HashSet};

use rand::Rng;
use teloxide::types::{User, UserId};

use crate::words::{get_random_word, Word};

pub const ROUNDS_COUNT: usize = 7;
pub const ROUND_DURATION_IN_MINUTES: usize = 2;

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
    AlreadyPlaying,
    NotBalancedTeams,
    IsNotPlaying,
}

#[derive(Default)]
struct NewRoom {
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

    fn get_teams(&self) -> Result<String, GameLogicError> {
        let result = self
            .teams
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

    fn check_teams_ready(&self) -> Result<(), GameLogicError> {
        // TODO: Check if teams contain wrong user ID
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
    time: f32,
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
}

struct PlayingRoom {
    teams: Vec<PlayingTeam>,
    turn: u8,
    round: u8,
}

impl PlayingRoom {
    fn from(lobby: NewRoom) -> PlayingRoom {
        // TODO: Shuffle teams
        PlayingRoom {
            teams: lobby
                .teams
                .into_iter()
                .map(|team| {
                    let team: Vec<_> = team.into_iter().collect();
                    PlayingTeam {
                        first: lobby.players.get(team.get(0).unwrap()).unwrap().to_owned(),
                        second: lobby.players.get(team.get(1).unwrap()).unwrap().to_owned(),
                        time: 0.0,
                        turn: 0,
                    }
                })
                .collect(),
            turn: 0,
            round: 0,
        }
    }

    fn get_describing_player(&self) -> User {
        self.teams[self.turn as usize].get_describing_player()
    }

    fn get_guessing_player(&self) -> User {
        self.teams[self.turn as usize].get_guessing_player()
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
            // TODO What to do?
            Room::Playing(_) => Ok("".to_owned()),
        }
    }

    fn get_playing(&self) -> Result<&PlayingRoom, GameLogicError> {
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

    pub fn start_round(&self) -> Result<WordGuessTry, GameLogicError> {
        let Room::Playing(playing) = self else {
            return Err(GameLogicError::IsNotPlaying);
        };

        Ok(WordGuessTry {
            word: get_random_word().text.clone(),
            describing: playing.get_describing_player(),
            guessing: playing.get_guessing_player(),
        })
    }
}
