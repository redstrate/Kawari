use std::collections::VecDeque;

use crate::{
    StatusEffects,
    zone_connection::{BaseParameters, TeleportQuery},
};
use kawari::{
    common::{DistanceRange, ObjectId, Position, Timeline},
    ipc::zone::{CommonSpawn, Conditions, SpawnNpc, SpawnObject, SpawnPlayer, SpawnTreasure},
};

#[derive(Debug, Clone, PartialEq)]
pub enum NpcState {
    /// Wanders in random directions.
    Wander,
    /// Follows its owner NPC.
    Follow,
    /// Actively targetting another actor.
    Hate,
    /// DEAD!
    Dead,
}

impl NpcState {
    /// Determines the natural state of this NPC.
    ///
    /// For example, normal NPCs should wander after resetting agro. Pets need to follow their owners instead.
    pub fn natural_state_of(spawn: &SpawnNpc) -> Self {
        if spawn.common.owner_id.is_valid() {
            Self::Follow
        } else {
            Self::Wander
        }
    }
}

#[derive(Debug, Clone)]
pub enum NetworkedActor {
    Player {
        spawn: SpawnPlayer,
        // TODO: of course, Npcs will need status effects as well!
        status_effects: StatusEffects,
        teleport_query: TeleportQuery,
        distance_range: DistanceRange,
        // TODO: make this is the single source-of-truth, instead of ZoneConnection handling it?
        conditions: Conditions,
        /// If this actor is currently executing a gimmick jump, and has yet to land.
        executing_gimmick_jump: bool,
        // If this actor is currently inside of an instance exit range.
        inside_instance_exit: bool,
        parameters: BaseParameters,
        dueling_opponent_id: ObjectId,
        /// Whether or not cooldowns should be cheatily removed.
        remove_cooldowns: bool,
    },
    Npc {
        state: NpcState,
        navmesh_path: VecDeque<[f32; 3]>,
        navmesh_path_lerp: f32,
        navmesh_target: Option<ObjectId>,
        last_position: Option<Position>,
        spawn: SpawnNpc,
        timeline: Timeline,
        /// In half-seconds (the current server logic tick.)
        timeline_position: i64,
        /// Used for aggros outside of the server logic loop (such as regular attacks.)
        newly_hated_actor: Option<ObjectId>,
        /// Whether this NPC is currently invulnerable to all attacks.
        currently_invulnerable: bool,
    },
    Object {
        object: SpawnObject,
    },
    Treasure {
        treasure: SpawnTreasure,
    },
}

impl NetworkedActor {
    pub fn get_common_spawn(&self) -> &CommonSpawn {
        match &self {
            NetworkedActor::Player { spawn, .. } => &spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
            _ => unreachable!(),
        }
    }

    pub fn get_common_spawn_mut(&mut self) -> &mut CommonSpawn {
        match self {
            NetworkedActor::Player { spawn, .. } => &mut spawn.common,
            NetworkedActor::Npc { spawn, .. } => &mut spawn.common,
            _ => unreachable!(),
        }
    }

    pub fn get_player_spawn(&self) -> Option<&SpawnPlayer> {
        match &self {
            NetworkedActor::Player { spawn, .. } => Some(spawn),
            _ => None,
        }
    }

    pub fn get_npc_spawn(&self) -> Option<&SpawnNpc> {
        match &self {
            NetworkedActor::Npc { spawn, .. } => Some(spawn),
            _ => None,
        }
    }

    pub fn position(&self) -> Position {
        match &self {
            NetworkedActor::Player { spawn, .. } => spawn.common.position,
            NetworkedActor::Npc { spawn, .. } => spawn.common.position,
            NetworkedActor::Object { object } => object.position,
            NetworkedActor::Treasure { treasure } => treasure.position,
        }
    }

    pub fn rotation(&self) -> f32 {
        match &self {
            NetworkedActor::Player { spawn, .. } => spawn.common.rotation,
            NetworkedActor::Npc { spawn, .. } => spawn.common.rotation,
            NetworkedActor::Object { object } => object.rotation,
            NetworkedActor::Treasure { treasure } => treasure.rotation,
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
            distance < distance_range.distance()
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
