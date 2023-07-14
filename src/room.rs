use std::collections::{HashMap, HashSet};

use rand::Rng;
use teloxide::types::{User, UserId};

pub fn get_new_id() -> RoomId {
    RoomId(rand::thread_rng().gen_range(10_000..=99_999))
}

pub enum CbQueryCommand {
    Join { team_index: usize },
    GetTeams,
    Play,
}

pub fn serialize_command(room_id: u32, query_command: CbQueryCommand) -> String {
    match query_command {
        CbQueryCommand::Join { team_index } => format!("join {} {}", room_id, team_index),
        CbQueryCommand::GetTeams => format!("get_teams {}", room_id),
        CbQueryCommand::Play => format!("play {}", room_id),
    }
}

pub fn parse_command(data: String) -> Option<(RoomId, CbQueryCommand)> {
    let (command, room_id, tail) = if let Some((index, _)) = data.match_indices(' ').nth(1) {
        let (header, tail) = data.split_at(index);
        let (command, room_id) = sscanf::sscanf!(header, "{} {}", String, u32).ok()?;
        (
            command,
            room_id,
            // Drop starting " "
            &tail[tail.char_indices().nth(1).unwrap().0..],
        )
    } else {
        let (command, room_id) = sscanf::sscanf!(data, "{} {}", String, u32).ok()?;
        (command, room_id, "")
    };

    let room_id = RoomId(room_id);
    match command.as_str() {
        "join" => {
            let team_index = sscanf::sscanf!(tail, "{}", usize).ok()?;
            Some((room_id, CbQueryCommand::Join { team_index }))
        }
        "get_teams" => Some((room_id, CbQueryCommand::GetTeams)),
        "play" => Some((room_id, CbQueryCommand::Play)),
        _ => None,
    }
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
