use std::collections::{HashMap, HashSet};

use rand::Rng;
use teloxide::types::{User, UserId};

pub fn get_new_id() -> RoomId {
    RoomId(rand::thread_rng().gen_range(10_000..=99_999))
}

pub fn create_team_choice_data(room_id: u32, team_index: usize) -> String {
    format!("{} {}", room_id, team_index)
}

pub fn parse_team_choice_data(data: String) -> Option<(RoomId, usize)> {
    let parsed = sscanf::sscanf!(data, "{} {}", u32, usize).ok()?;
    Some((RoomId(parsed.0), parsed.1))
}

pub fn get_teams(number_of_teams: usize) -> Vec<String> {
    (0..number_of_teams)
        .map(|n| format!("Team {}", n + 1))
        .collect()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoomId(pub u32);

pub struct Room {
    players: HashMap<UserId, User>,
    number_of_teams: usize,
    teams: Vec<HashSet<UserId>>,
}

#[derive(Debug)]
pub enum GameLogicError {
    AlreadyJoined,
    NotJoinedToRoom,
}

impl Room {
    pub fn new(number_of_teams: usize) -> Room {
        Room {
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
}
