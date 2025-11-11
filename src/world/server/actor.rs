use std::collections::VecDeque;

use crate::{
    common::{ObjectId, Position},
    ipc::zone::{CommonSpawn, NpcSpawn, PlayerSpawn},
};

#[derive(Debug, Clone)]
pub enum NetworkedActor {
    Player(PlayerSpawn),
    Npc {
        current_path: VecDeque<[f32; 3]>,
        current_path_lerp: f32,
        current_target: Option<ObjectId>,
        last_position: Option<Position>,
        spawn: NpcSpawn,
    },
}

impl NetworkedActor {
    pub fn get_common_spawn(&self) -> &CommonSpawn {
        match &self {
            NetworkedActor::Player(player_spawn) => &player_spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
        }
    }

    pub fn get_player_spawn(&self) -> Option<&PlayerSpawn> {
        match &self {
            NetworkedActor::Player(player_spawn) => Some(player_spawn),
            NetworkedActor::Npc { .. } => None,
        }
    }
}
