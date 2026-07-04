use std::collections::{HashMap, VecDeque};

use crate::{
    StatusEffects,
    server::combat_state::PlayerCombatState,
    zone_connection::{BaseParameters, TeleportQuery},
};
use glam::Vec3;
use kawari::{
    common::{DistanceRange, ObjectId, Position, STRIKING_DUMMY_NAME_ID, Timeline},
    ipc::zone::{CommonSpawn, Conditions, SpawnNpc, SpawnObject, SpawnPlayer, SpawnTreasure},
};

#[derive(Debug, Clone, PartialEq)]
pub enum NpcState {
    /// Wanders in random directions.
    Wander,
    /// Follows its owner NPC.
    Follow,
    /// Stays at its current position until explicitly told otherwise.
    Stay,
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
        /// This actor's status effects.
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
        combat_state: PlayerCombatState,
        /// Whether the player can execute a combo action. If so, contains a Some of the last action used.
        last_combo_action: u16,
        /// Sequence into the current combo.
        combo_sequence: u8,
        /// BNpcs currently tracking this player, mapped to the hate slot shown by the client.
        hated_by: HashMap<ObjectId, u8>,
        /// Last enmity snapshot sent to this player as a sorted `(npc_id, rate%)` list. Used to
        /// suppress redundant HaterList/EnmityList packets when nothing changed since last tick.
        last_enmity_sent: Vec<(ObjectId, u8)>,
    },
    Npc {
        state: NpcState,
        navmesh_path: VecDeque<Vec3>,
        navmesh_path_lerp: f32,
        navmesh_target: Option<ObjectId>,
        last_position: Option<Vec3>,
        /// The position this NPC spawned at, used as the leash anchor for deaggro.
        spawn_position: Vec3,
        spawn: SpawnNpc,
        timeline: Timeline,
        /// In half-seconds (the current server logic tick.)
        timeline_position: i64,
        /// Persistent hate values keyed by actor id.
        hate_list: HashMap<ObjectId, u32>,
        /// Whether this NPC is currently invulnerable to all attacks.
        currently_invulnerable: bool,
        /// When true, the NPC's behavior (chase + auto-attack/abilities) is paused — used while a
        /// boss is "off the field" doing a mechanic (e.g. Ifrit's Crimson Cyclone jump-away) so it
        /// doesn't keep hitting players while hidden. It stays alive and keeps its hate list.
        ai_paused: bool,
        /// Whether this NPC can be targeted by players. Defaults to true. Visual-only actors (e.g.
        /// Crimson Cyclone clones) set this false; it's re-applied via an ActorControl on every
        /// walked-in spawn, since the spawn packet itself has no targetable field and the client
        /// defaults a fresh spawn to targetable.
        targetable: bool,
        /// Whether this NPC is rendered. Defaults to true. Clones idle hidden on the arena edge and
        /// are only shown during their mechanic; like `targetable`, re-applied via an ActorControl
        /// (ToggleVisibility) on every walked-in spawn so a fresh spawn doesn't pop into view.
        visible: bool,
        /// True while the NPC is locked in a cast bar (set when a `CastBar` is sent, cleared when the
        /// cast's effect resolves). Like `ai_paused` it suppresses chase + auto-attack, so a boss
        /// freezes while casting instead of walking up and meleeing — but it's separate so it never
        /// disturbs the persistent `ai_paused` of visual-only actors (clones).
        cast_locked: bool,
        /// This actor's status effects.
        status_effects: StatusEffects,
    },
    Object {
        object: SpawnObject,
        /// Name of the layer that the object originates from. Can be empty.
        layer_name: String,
    },
    Treasure {
        treasure: SpawnTreasure,
    },
}

impl NetworkedActor {
    pub fn common_spawn(&self) -> Option<&CommonSpawn> {
        match &self {
            NetworkedActor::Player { spawn, .. } => Some(&spawn.common),
            NetworkedActor::Npc { spawn, .. } => Some(&spawn.common),
            _ => None,
        }
    }

    pub fn common_spawn_mut(&mut self) -> Option<&mut CommonSpawn> {
        match self {
            NetworkedActor::Player { spawn, .. } => Some(&mut spawn.common),
            NetworkedActor::Npc { spawn, .. } => Some(&mut spawn.common),
            _ => None,
        }
    }

    pub fn get_common_spawn(&self) -> &CommonSpawn {
        self.common_spawn()
            .expect("networked actor does not have a common spawn")
    }

    pub fn get_common_spawn_mut(&mut self) -> &mut CommonSpawn {
        self.common_spawn_mut()
            .expect("networked actor does not have a common spawn")
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
            NetworkedActor::Object { object, .. } => object.position,
            NetworkedActor::Treasure { treasure } => treasure.position,
        }
    }

    pub fn rotation(&self) -> f32 {
        match &self {
            NetworkedActor::Player { spawn, .. } => spawn.common.rotation,
            NetworkedActor::Npc { spawn, .. } => spawn.common.rotation,
            NetworkedActor::Object { object, .. } => object.rotation,
            NetworkedActor::Treasure { treasure } => treasure.rotation,
        }
    }

    pub fn in_range_of(&self, other: &NetworkedActor) -> bool {
        // This only makes sense for players
        if let NetworkedActor::Player { distance_range, .. } = self {
            // Retail doesn't take into account Y
            let mut self_pos = self.position().0;
            self_pos.y = 0.0;

            let mut other_pos = other.position().0;
            other_pos.y = 0.0;

            let distance = Vec3::distance(self_pos, other_pos);
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

    /// Returns this actor's status effects list.
    pub fn status_effects(&self) -> Option<&StatusEffects> {
        match self {
            NetworkedActor::Player { status_effects, .. } => Some(status_effects),
            NetworkedActor::Npc { status_effects, .. } => Some(status_effects),
            _ => None,
        }
    }

    /// Returns this actor's status effects list.
    pub fn status_effects_mut(&mut self) -> Option<&mut StatusEffects> {
        match self {
            NetworkedActor::Player { status_effects, .. } => Some(status_effects),
            NetworkedActor::Npc { status_effects, .. } => Some(status_effects),
            _ => None,
        }
    }

    pub fn shield_percent(&self) -> u8 {
        let Some(common) = self.common_spawn() else {
            return 0;
        };
        self.status_effects()
            .map(|status_effects| status_effects.shield_percent(common.max_health_points))
            .unwrap_or(0)
    }

    /// Applies incoming damage after consuming any active barrier effects. The returned amount is
    /// the damage that reached HP; callers can still display the original hit amount if desired.
    pub fn apply_damage(&mut self, damage: u32) -> u32 {
        let hp_damage = self
            .status_effects_mut()
            .map(|status_effects| status_effects.absorb_damage(damage))
            .unwrap_or(damage);

        let Some(common) = self.common_spawn_mut() else {
            return hp_damage;
        };

        if common.name_id == STRIKING_DUMMY_NAME_ID {
            if hp_damage >= common.health_points {
                common.health_points = common.max_health_points;
            } else {
                common.health_points -= hp_damage;
            }
        } else {
            common.health_points = common.health_points.saturating_sub(hp_damage);
        }

        hp_damage
    }

    pub fn npc_hate_list(&self) -> Option<&HashMap<ObjectId, u32>> {
        match self {
            NetworkedActor::Npc { hate_list, .. } => Some(hate_list),
            _ => None,
        }
    }

    pub fn npc_hate_list_mut(&mut self) -> Option<&mut HashMap<ObjectId, u32>> {
        match self {
            NetworkedActor::Npc { hate_list, .. } => Some(hate_list),
            _ => None,
        }
    }

    pub fn player_hated_by_mut(&mut self) -> Option<&mut HashMap<ObjectId, u8>> {
        match self {
            NetworkedActor::Player { hated_by, .. } => Some(hated_by),
            _ => None,
        }
    }
}
