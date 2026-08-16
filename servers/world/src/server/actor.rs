use std::{collections::VecDeque, sync::Arc, time::Instant};

use crate::{
    ClientId, FromServer, GameData, StatusEffects,
    server::{
        WorldServer,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
    zone_connection::{BaseParameters, TeleportQuery},
};
use glam::Vec3A;
use kawari::{
    common::{
        CharacterMode, DEAD_FADE_OUT_TIME, DistanceRange, ObjectId, Position,
        SharedGroupTimelineState, Timeline, TimepointData, should_respawn_mobs,
    },
    config::get_config,
    ipc::zone::{
        ActorControlCategory, BattleNpcSubKind, CommonSpawn, Conditions, ObjectKind,
        ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc, SpawnObject, SpawnPlayer, SpawnTreasure,
    },
};
use parking_lot::Mutex;
use physis::TerritoryIntendedUse;

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
pub enum NpcTarget {
    Actor(ObjectId),
    Position(Vec3A),
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
        /// Whether the player can execute a combo action. If so, contains a Some of the last action used.
        last_combo_action: u16,
        /// Sequence into the current combo.
        combo_sequence: u8,
        /// Their current auto-attack target, if any.
        autoattack_target: Option<ObjectId>,
        /// In half-seconds (the current server logic tick.)
        autoattack_timing: i64,
    },
    Npc {
        state: NpcState,
        navmesh_path: VecDeque<Vec3A>,
        navmesh_path_lerp: f32,
        navmesh_target: Option<NpcTarget>,
        last_position: Option<Vec3A>,
        spawn: SpawnNpc,
        timeline: Timeline,
        /// In half-seconds (the current server logic tick.)
        timeline_position: i64,
        /// Used for aggros outside of the server logic loop (such as regular attacks.)
        newly_hated_actor: Option<ObjectId>,
        /// Whether this NPC is currently invulnerable to all attacks.
        currently_invulnerable: bool,
        /// This actor's status effects.
        status_effects: StatusEffects,
        /// The last time the mob wandered.
        last_wander_timestamp: Instant,
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

            let distance = Vec3A::distance(self_pos, other_pos);
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
}

pub fn set_player_minion(
    data: &mut WorldServer,
    network: &mut NetworkState,
    minion_id: u32,
    from_actor_id: ObjectId,
) {
    // Update our common spawn to reflect the new minion
    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
        return;
    };

    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
        return;
    };

    let NetworkedActor::Player { spawn, .. } = actor else {
        return;
    };

    spawn.common.active_minion = minion_id as u16;

    network.send_ac_in_range_inclusive(
        data,
        from_actor_id,
        ActorControlCategory::MinionSpawnControl { minion_id },
    );
}

pub fn set_character_mode(
    instance: &mut Instance,
    network: &mut NetworkState,
    from_actor_id: ObjectId,
    mode: CharacterMode,
    mode_arg: u8,
) {
    // Update internal data model for new spawns
    {
        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        // Skip if this mode is already set.
        if actor.get_common_spawn().mode == mode && actor.get_common_spawn().mode_arg == mode_arg {
            return;
        }

        actor.get_common_spawn_mut().mode = mode;
        actor.get_common_spawn_mut().mode_arg = mode_arg;
    }

    // Inform actors
    network.send_ac_in_range_inclusive_instance(
        instance,
        from_actor_id,
        ActorControlCategory::SetMode {
            mode,
            mode_arg: mode_arg as u32,
        },
    );
}

pub fn set_shared_group_timeline_state(
    instance: &mut Instance,
    network: &mut NetworkState,
    from_actor_id: ObjectId,
    timelines: &[u32],
) {
    let mut state = SharedGroupTimelineState::empty();
    for timeline in timelines {
        state.toggle(match timeline {
            1 => SharedGroupTimelineState::TIMELINE_1,
            2 => SharedGroupTimelineState::TIMELINE_2,
            3 => SharedGroupTimelineState::TIMELINE_3,
            4 => SharedGroupTimelineState::TIMELINE_4,
            5 => SharedGroupTimelineState::TIMELINE_5,
            6 => SharedGroupTimelineState::TIMELINE_6,
            7 => SharedGroupTimelineState::TIMELINE_7,
            8 => SharedGroupTimelineState::TIMELINE_8,
            9 => SharedGroupTimelineState::TIMELINE_9,
            10 => SharedGroupTimelineState::TIMELINE_10,
            11 => SharedGroupTimelineState::TIMELINE_11,
            12 => SharedGroupTimelineState::TIMELINE_12,
            13 => SharedGroupTimelineState::TIMELINE_13,
            14 => SharedGroupTimelineState::TIMELINE_14,
            15 => SharedGroupTimelineState::TIMELINE_15,
            16 => SharedGroupTimelineState::TIMELINE_16,
            _ => unimplemented!(),
        });
    }

    // Update internal data model for new spawns
    {
        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let NetworkedActor::Object { object, .. } = actor else {
            return;
        };

        object.args1 = state.bits();
    }

    // Inform actors
    network.send_ac_in_range_inclusive_instance(
        instance,
        from_actor_id,
        ActorControlCategory::SetSharedGroupTimelineState {
            state,
            arg2: 0,
            object_type: 0,
            layout_id: 0,
        },
    );
}

// Sends the ActorControls to inform the actor that they're dead.
pub fn kill_actor(network: Arc<Mutex<NetworkState>>, instance: &mut Instance, actor_id: ObjectId) {
    let mut network = network.lock();

    // First, set their state (otherwise they can still walk)
    set_character_mode(instance, &mut network, actor_id, CharacterMode::Dead, 0);

    // Then, play the death animation.
    {
        let ac = ActorControlCategory::Kill { animation_id: 0 };

        network.send_ac_in_range_inclusive_instance(instance, actor_id, ac);
    }

    // Inform the director that their actor died
    let mut npc_id = None;
    let mut position = None;
    if let Some(actor) = instance.find_actor(actor_id)
        && let Some(npc) = actor.get_npc_spawn()
    {
        npc_id = Some(npc.common.layout_id);
    }

    // Transistion into the dead state so they stop moving.
    if let Some(actor) = instance.find_actor_mut(actor_id)
        && let NetworkedActor::Npc { state, spawn, .. } = actor
    {
        *state = NpcState::Dead;
        position = Some(spawn.common.position);
    }

    if let Some(npc_id) = npc_id
        && let Some(director) = &mut instance.director
    {
        director.on_actor_death(npc_id, position.unwrap());
    }

    // Cancel existing tasks
    instance.cancel_actor_tasks(actor_id);
    let intended_use = instance.zone.intended_use;

    // Queue up despawn if this is an NPC
    if let Some(actor) = instance.find_actor_mut(actor_id)
        && let NetworkedActor::Npc {
            spawn, timeline, ..
        } = actor
    {
        let mut new_timeline_states = Vec::new();

        // Play any timeline actions on death.
        // TODO: please de-duplicate with the other handler if possible!
        for action in &timeline.on_death {
            match action {
                TimepointData::TimelineState { states } => {
                    // Find the event object bound to our gimmick.
                    let gimmick_id = spawn.gimmick_id;
                    new_timeline_states.push((gimmick_id, states.clone()));
                }
                _ => unimplemented!(),
            }
        }

        let respawn_layout_id = if actor.get_common_spawn().layout_id != 0
            && should_respawn_mobs(
                TerritoryIntendedUse::from_repr(intended_use).unwrap_or(TerritoryIntendedUse::Town),
            ) {
            Some(actor.get_common_spawn().layout_id)
        } else {
            None
        };

        for (gimmick_id, states) in new_timeline_states {
            let actor_id;
            {
                actor_id = instance.find_object_by_bind_layout_id(gimmick_id);
            }
            if let Some(actor_id) = actor_id {
                set_shared_group_timeline_state(instance, &mut network, actor_id, &states);
            }
        }

        instance.insert_task(
            ClientId::default(),
            actor_id,
            DEAD_FADE_OUT_TIME,
            QueuedTaskData::DeadFadeOut {
                actor_id,
                respawn_layout_id,
            },
        );
    }
}

/// Updates other actors about this actor's HP and MP.
pub fn update_actor_hp_mp(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    target_actor_id: ObjectId,
) {
    let mut send_kill_actor = false;
    // Inform the client of the new actor's HP/MP
    {
        let Some(actor) = instance.find_actor(target_actor_id) else {
            return;
        };

        let common_spawn = actor.get_common_spawn();

        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp {
                hp: common_spawn.health_points,
                mp: common_spawn.resource_points,
                unk: 0,
            });
            let mut network = network.lock();
            network.send_in_range_inclusive_instance(
                target_actor_id,
                instance,
                FromServer::PacketSegment(ipc, target_actor_id),
                DestinationNetwork::ZoneClients,
            );
        }

        if common_spawn.health_points == 0 && common_spawn.mode != CharacterMode::Dead {
            send_kill_actor = true;
        }
    }

    if send_kill_actor {
        kill_actor(network.clone(), instance, target_actor_id);
    }
}

pub fn spawn_custom_bnpc(
    data: &mut WorldServer,
    game_data: &mut GameData,
    from_actor_id: ObjectId,
    base_id: u32,
    name_id: u32,
) {
    let actor_id = Instance::generate_actor_id();
    {
        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        let NetworkedActor::Player { spawn, .. } = actor else {
            return;
        };

        let base_npc = create_npc_common_spawn(game_data, base_id, name_id, None, 1);
        let npc_spawn = SpawnNpc {
            common: CommonSpawn {
                position: spawn.common.position,
                ..base_npc.common
            },
            ..base_npc
        };

        let config = get_config();
        instance.insert_npc(actor_id, npc_spawn.clone(), &config);
    }
}

/// Creates the NPC spawn data based off of game data and whatever parameters you require.
pub fn create_npc_common_spawn(
    game_data: &mut GameData,
    base_id: u32,
    name_id: u32,
    hp: Option<u32>,
    level: u32,
) -> SpawnNpc {
    let (model_chara, battalion, customize, rank, equip, behavior) =
        game_data.find_bnpc(base_id).unwrap();

    let usable_hp;
    if let Some(hp) = hp {
        usable_hp = hp;
    } else {
        let classjob_id = 0; // Pretty sure it's this for all enemies
        let modifiers = game_data
            .get_class_job_modifiers(classjob_id as u32)
            .expect("Failed to read param grow");

        let attributes = game_data
            .get_racial_base_attributes(classjob_id)
            .expect("Failed to read racial attributes");

        let param_grow = game_data
            .get_param_grow(level)
            .expect("Failed to read param grow");

        let mut base_parameters = BaseParameters::default();
        let primary_stat = game_data
            .get_job_primary_stat(classjob_id as u16)
            .unwrap_or(1);
        base_parameters.perform_calculations(primary_stat, &attributes, &param_grow, &modifiers);
        base_parameters.calculate_potencies(&param_grow, None); // TODO: If NPCs have classjob modifiers and such, change that None!

        usable_hp = base_parameters.hp;
    }

    SpawnNpc {
        character_data_icon: rank,
        common: CommonSpawn {
            base_id,
            name_id,
            max_health_points: usable_hp,
            health_points: usable_hp,
            model_chara,
            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
            battalion,
            level: level as u8,
            look: customize,
            behavior,
            ..game_data.get_npc_equip(equip as u32).unwrap_or_default()
        },
        ..Default::default()
    }
}
