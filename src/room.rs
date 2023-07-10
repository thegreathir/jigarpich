use std::collections::HashMap;

use rand::Rng;
use teloxide::types::{User, UserId};

pub fn get_new_id() -> RoomId {
    RoomId(rand::thread_rng().gen_range(10_000..=99_999))
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(pub u32);

pub struct Room {
    players: HashMap<UserId, User>,
}

#[derive(Debug)]
pub enum GameLogicError {
    FullRoom,
    AlreadyJoined,
}

const ROOM_PLAYERS_CAPACITY: usize = 4;

impl Room {
    pub fn new() -> Room {
        Room {
            players: HashMap::new(),
        }
    }

    pub fn join(&mut self, user: User) -> Result<Vec<UserId>, GameLogicError> {
        if self.players.len() == ROOM_PLAYERS_CAPACITY {
            Err(GameLogicError::FullRoom)
        } else if let std::collections::hash_map::Entry::Vacant(e) = self.players.entry(user.id) {
            e.insert(user);
            Ok(self.players.keys().cloned().collect())
        } else {
            Err(GameLogicError::AlreadyJoined)
        }
    }
}
