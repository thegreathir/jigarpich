use std::collections::{HashMap, HashSet};

use rand::Rng;
use teloxide::types::{User, UserId};

pub fn get_new_id() -> RoomId {
    RoomId(rand::thread_rng().gen_range(10_000..=99_999))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(pub u32);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct TeamId(pub u32);

pub struct Room {
    players: HashMap<UserId, User>,
    teams: HashMap<TeamId, HashSet<UserId>>,
}

#[derive(Debug)]
pub enum GameLogicError {
    AlreadyJoined,
    NotJoinedToRoom,
}

impl Room {
    pub fn new() -> Room {
        Room {
            players: HashMap::new(),
            teams: HashMap::new(),
        }
    }

    pub fn join(&mut self, user: User) -> Result<Vec<UserId>, GameLogicError> {
        if let std::collections::hash_map::Entry::Vacant(e) = self.players.entry(user.id) {
            e.insert(user);
            Ok(self.players.keys().cloned().collect())
        } else {
            Err(GameLogicError::AlreadyJoined)
        }
    }

    pub fn join_team(&mut self, user_id: UserId, team_id: TeamId) -> Result<Vec<UserId>, GameLogicError> {
        if !self.players.contains_key(&user_id) {
            Err(GameLogicError::NotJoinedToRoom)
        } else {
            self.teams.values_mut().for_each(|team| {
                team.remove(&user_id);
            });

            self.teams.entry(team_id).or_default().insert(user_id);

            Ok(self.players.keys().cloned().collect())
        }
    }
}
