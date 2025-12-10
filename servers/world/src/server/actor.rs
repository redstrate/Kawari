use std::collections::VecDeque;

use crate::{StatusEffects, zone_connection::TeleportQuery};
use kawari::{
    common::{DistanceRange, ObjectId, Position, get_distance_range},
    ipc::zone::{CommonSpawn, NpcSpawn, ObjectSpawn, PlayerSpawn},
};

#[derive(Debug, Clone)]
pub enum NetworkedActor {
    Player {
        spawn: PlayerSpawn,
        // TODO: of course, Npcs will need status effects as well!
        status_effects: StatusEffects,
        teleport_query: TeleportQuery,
        distance_range: DistanceRange,
    },
    Npc {
        current_path: VecDeque<[f32; 3]>,
        current_path_lerp: f32,
        current_target: Option<ObjectId>,
        last_position: Option<Position>,
        spawn: NpcSpawn,
    },
    Object {
        object: ObjectSpawn,
    },
}

impl NetworkedActor {
    pub fn get_common_spawn(&self) -> &CommonSpawn {
        match &self {
            NetworkedActor::Player { spawn, .. } => &spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
            NetworkedActor::Object { .. } => unreachable!(),
        }
    }

    pub fn get_common_spawn_mut(&mut self) -> &mut CommonSpawn {
        match self {
            NetworkedActor::Player { spawn, .. } => &mut spawn.common,
            NetworkedActor::Npc { spawn, .. } => &mut spawn.common,
            NetworkedActor::Object { .. } => unreachable!(),
        }
    }

    pub fn get_player_spawn(&self) -> Option<&PlayerSpawn> {
        match &self {
            NetworkedActor::Player { spawn, .. } => Some(spawn),
            NetworkedActor::Npc { .. } => None,
            NetworkedActor::Object { .. } => None,
        }
    }

    pub fn position(&self) -> Position {
        match &self {
            NetworkedActor::Player { spawn, .. } => spawn.common.pos,
            NetworkedActor::Npc { spawn, .. } => spawn.common.pos,
            NetworkedActor::Object { object } => object.position,
        }
    }

    pub fn in_range_of(&self, other: &NetworkedActor) -> bool {
        // This only makes sense for players
        if let NetworkedActor::Player { distance_range, .. } = self {
            // Retail doesn't take into account Y
            let self_pos = Position {
                x: self.position().x,
                y: 0.0,
                z: self.position().z,
            };

            let other_pos = Position {
                x: other.position().x,
                y: 0.0,
                z: other.position().z,
            };

            let distance = Position::distance(self_pos, other_pos);
            distance < get_distance_range(*distance_range)
        } else {
            false
        }
    }

    /// Really only applies to Players, whether or not they have loaded in yet.
    pub fn is_valid(&self) -> bool {
        if let NetworkedActor::Player { spawn, .. } = self {
            !spawn.common.name.is_empty()
        } else {
            true
        }
    }
}
