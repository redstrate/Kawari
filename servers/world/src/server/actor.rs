use std::{collections::VecDeque, sync::Arc};

use crate::{
    ClientId, FromServer, StatusEffects, server::{WorldServer, instance::{Instance, QueuedTaskData}, network::{DestinationNetwork, NetworkState}}, zone_connection::{BaseParameters, TeleportQuery}
};
use glam::Vec3;
use kawari::{
    common::{CharacterMode, DEAD_FADE_OUT_TIME, DistanceRange, ObjectId, Position, SharedGroupTimelineState, Timeline, TimepointData},
    ipc::zone::{ActorControlCategory, CommonSpawn, Conditions, ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc, SpawnObject, SpawnPlayer, SpawnTreasure},
};
use parking_lot::Mutex;

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
    },
    Npc {
        state: NpcState,
        navmesh_path: VecDeque<Vec3>,
        navmesh_path_lerp: f32,
        navmesh_target: Option<ObjectId>,
        last_position: Option<Vec3>,
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
pub fn kill_actor(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    from_actor_id: ObjectId,
) {
    // TODO: set HP/MP to zero here

    let mut network = network.lock();

    // First, set their state (otherwise they can still walk)
    set_character_mode(
        instance,
        &mut network,
        from_actor_id,
        CharacterMode::Dead,
        0,
    );

    // Then, play the death animation.
    {
        let ac = ActorControlCategory::Kill { animation_id: 0 };

        network.send_ac_in_range_inclusive_instance(instance, from_actor_id, ac);
    }

    // Inform the director that their actor died
    let mut npc_id = None;
    let mut position = None;
    if let Some(actor) = instance.find_actor(from_actor_id)
        && let Some(npc) = actor.get_npc_spawn()
        {
            npc_id = Some(npc.common.layout_id);
        }

        // Transistion into the dead state so they stop moving.
        if let Some(actor) = instance.find_actor_mut(from_actor_id)
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
                instance.cancel_actor_tasks(from_actor_id);

            // Queue up despawn if this is an NPC
            if let Some(actor) = instance.find_actor_mut(from_actor_id)
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
                                         from_actor_id,
                                         DEAD_FADE_OUT_TIME,
                                         QueuedTaskData::DeadFadeOut {
                                             actor_id: from_actor_id,
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
