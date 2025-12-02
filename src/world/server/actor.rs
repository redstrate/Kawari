use std::collections::VecDeque;

use crate::{
    common::{ObjectId, Position},
    ipc::zone::{CommonSpawn, NpcSpawn, PlayerSpawn},
    world::StatusEffects,
};

#[derive(Debug, Clone)]
pub enum NetworkedActor {
    Player {
        spawn: PlayerSpawn,
        // TODO: of course, Npcs will need status effects as well!
        status_effects: StatusEffects,
    },
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
            NetworkedActor::Player { spawn, .. } => &spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
        }
    }

    pub fn get_common_spawn_mut(&mut self) -> &mut CommonSpawn {
        match self {
            NetworkedActor::Player { spawn, .. } => &mut spawn.common,
            NetworkedActor::Npc { spawn, .. } => &mut spawn.common,
        }
    }

    pub fn get_player_spawn(&self) -> Option<&PlayerSpawn> {
        match &self {
            NetworkedActor::Player { spawn, .. } => Some(spawn),
            NetworkedActor::Npc { .. } => None,
        }
    }
}
