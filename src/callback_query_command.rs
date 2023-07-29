use crate::room::RoomId;

pub enum CbQueryCommand {
    Join { team_index: usize },
    GetTeams,
    Play,
    Start,
    Correct,
    Skip,
}

pub fn serialize_command(room_id: RoomId, query_command: CbQueryCommand) -> String {
    match query_command {
        CbQueryCommand::Join { team_index } => format!("join {} {}", room_id.0, team_index),
        CbQueryCommand::GetTeams => format!("get_teams {}", room_id.0),
        CbQueryCommand::Play => format!("play {}", room_id.0),
        CbQueryCommand::Start => format!("start {}", room_id.0),
        CbQueryCommand::Correct => format!("correct {}", room_id.0),
        CbQueryCommand::Skip => format!("skip {}", room_id.0),
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
        "start" => Some((room_id, CbQueryCommand::Start)),
        "correct" => Some((room_id, CbQueryCommand::Correct)),
        "skip" => Some((room_id, CbQueryCommand::Skip)),
        _ => None,
    }
}
