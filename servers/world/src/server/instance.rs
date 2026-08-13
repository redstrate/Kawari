use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    ClientId, FromServer, GameData, Navmesh, StatusEffects,
    lua::KawariLua,
    server::{
        action::cancel_action,
        actor::{NetworkedActor, NpcState},
        director::DirectorData,
        fate::FateData,
        network::{DestinationNetwork, NetworkState},
        zone::Zone,
    },
    zone_connection::{BaseParameters, TeleportQuery},
};
use kawari::{
    common::{
        DistanceRange, ENTRANCE_CIRCLE_IDS, FATE_TIME_LIMIT, FateRule, FateState, MAXIMUM_FATES,
        ObjectId, Position, timestamp_secs,
    },
    config::{Config, get_config},
    ipc::zone::{
        ActionRequest, ActorControlCategory, Conditions, ServerZoneIpcData, ServerZoneIpcSegment,
        SpawnNpc, SpawnObject, SpawnPlayer, SpawnTreasure,
    },
};
use parking_lot::Mutex;

#[derive(Default, Debug)]
pub enum NavmeshGenerationStep {
    /// No generation is currently happening.
    #[default]
    None,
    /// We need to generate a navmesh at this path.
    Needed(String),
    /// The process to write the navmesh has started, and we need to wait until the file exists.
    Started(String),
}

#[derive(Debug, Clone)]
pub enum QueuedTaskData {
    CastAction {
        request: ActionRequest,
        /// Currently means if it has a cast bar.
        interruptible: bool,
    },
    LoseStatusEffect {
        effect_id: u16,
        effect_param: u16,
        effect_source_actor_id: ObjectId,
    },
    /// Fade out a dead actor.
    DeadFadeOut {
        actor_id: ObjectId,
        respawn_layout_id: Option<u32>,
    },
    /// Despawn a dead actor.
    DeadDespawn {
        actor_id: ObjectId,
        respawn_layout_id: Option<u32>,
    },
    /// Complete an EventAction
    CastEventAction { target: ObjectId },
    /// Make a fish bite.
    FishBite,
    /// Seal a boss wall.
    SealBossWall { id: u32, place_name: u32 },
    /// Generically send a packet segment, only used for `do_change_zone`. Don't abuse this as a generic task, you almost certainly want to create a new variant.
    PacketSegment { segment: ServerZoneIpcSegment },
    /// Used by directors since its tough to fit this into the director logic.
    WarpToPopRange { id: u32 },
    /// Reset a player's action combo status.
    ResetCombo,
    /// Respawn a new mob.
    RespawnMob { layout_id: u32 },
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub point: Instant,
    pub from_id: ClientId,
    pub from_actor_id: ObjectId,
    pub data: QueuedTaskData,
}

impl PartialEq for QueuedTask {
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point
            && self.from_id == other.from_id
            && self.from_actor_id == other.from_actor_id
    }
}

#[derive(Debug, Clone)]
pub struct FateInstance {
    /// Index into the Fate Excel sheet.
    pub fate_id: u32,
    /// When this FATE was started.
    pub start_timestamp: u32,
    /// The current state of the FATE.
    pub fate_state: FateState,
    pub data: FateData,
    /// The start NPC for this FATE, if applicable.
    pub motivation_npc: Option<(ObjectId, Position)>,
}

impl FateInstance {
    pub fn new(fate_id: u32, game_data: &mut GameData) -> Self {
        let fate_rule = game_data.get_fate_rule(fate_id).unwrap_or_default();
        let fate_state = match fate_rule {
            FateRule::Gathering => FateState::Preparing,
            _ => FateState::Running,
        };

        // Setup Lua state
        let lua = KawariLua::new();

        // Find the script for this FATE
        let file_name = get_config()
            .filesystem
            .locate_script_file("content/test_fate_soul.lua");

        let mut data = FateData::default();

        // HACK: hardcoded to this FATE for now
        if fate_id == 603 {
            let result = std::fs::read(&file_name);
            if let Err(err) = result {
                tracing::warn!(
                    "Failed to load {}: {:?} instance content won't be scripted!",
                    file_name,
                    err
                );
            } else {
                let file = result.unwrap();

                if let Err(err) = lua
                    .0
                    .load(file)
                    .set_name("@".to_string() + &file_name)
                    .exec()
                {
                    tracing::warn!(
                        "Syntax error in {}: {:?} instance content won't be scripted!",
                        file_name,
                        err
                    );
                } else {
                    data.lua = lua;

                    // Call into the onSetup function before returning, as we need the flag to be initialized before any players change zones.
                    data.setup();
                }
            }
        }

        Self {
            fate_id,
            start_timestamp: timestamp_secs(),
            fate_state,
            data,
            motivation_npc: None,
        }
    }
}

#[derive(Default, Debug)]
pub struct Instance {
    pub actors: HashMap<ObjectId, NetworkedActor>,
    pub navmesh: Navmesh,
    pub zone: Zone,
    pub weather_id: u16,
    pub content_finder_condition_id: u16,
    /// If Some, then this is the path of the navmesh we need to generate.
    pub generate_navmesh: NavmeshGenerationStep,
    /// List of tasks that has to be executed an arbitrary point in the future.
    pub queued_task: Vec<QueuedTask>,
    /// Director for this instance.
    pub director: Option<DirectorData>,
    pub enemy_ai_disabled: bool,
    pub fates: Vec<FateInstance>,
}

impl Instance {
    pub fn new(id: u16, game_data: &mut GameData) -> Self {
        let mut instance = Instance {
            zone: Zone::load(game_data, id),
            weather_id: game_data.get_weather(id as u32).unwrap_or_default() as u16,
            ..Default::default()
        };

        let config = get_config();
        if config.filesystem.navimesh_path.is_empty() {
            tracing::warn!("Navimesh path is not set! Monsters will not function correctly!");
        } else if instance.zone.navimesh_path.is_empty() {
            tracing::warn!("No navimesh path for this zone, skipping generation!");
        } else {
            let mut nvm_path = PathBuf::from(config.filesystem.navimesh_path);
            nvm_path.push(instance.zone.navimesh_path.clone());

            if let Ok(nvm_bytes) = std::fs::read(&nvm_path) {
                if let Some(navmesh) = Navmesh::from_existing(&nvm_bytes) {
                    instance.navmesh = navmesh;

                    tracing::info!("Successfully loaded navimesh from {nvm_path:?}");
                } else {
                    tracing::warn!(
                        "Failed to read {nvm_path:?}, monsters will not function correctly!"
                    );
                }
            } else if config.world.generate_navmesh {
                instance.generate_navmesh =
                    NavmeshGenerationStep::Needed(nvm_path.to_str().unwrap().to_string());
            } else {
                tracing::warn!(
                    "Failed to read {nvm_path:?}, monsters will not function correctly!"
                );
            }
        }

        // Load initial event objects into instance
        for (object, layer_name) in instance.zone.get_event_objects(game_data, false) {
            instance.insert_object(object.entity_id, object, layer_name);
        }

        // Load initial NPCs into instance
        let config = get_config();
        for npc in instance.zone.get_npcs(game_data) {
            instance.insert_npc(ObjectId(fastrand::u32(..)), npc, &config);
        }

        // Determine the starting set of FATEs
        let available_fates = instance.zone.map_ranges.iter().filter_map(|x| x.fate);
        for fate_id in fastrand::choose_multiple(available_fates, MAXIMUM_FATES) {
            instance.fates.push(FateInstance::new(fate_id, game_data));
        }

        instance
    }

    pub fn find_actor(&self, id: ObjectId) -> Option<&NetworkedActor> {
        self.actors.get(&id)
    }

    pub fn find_actor_mut(&mut self, id: ObjectId) -> Option<&mut NetworkedActor> {
        self.actors.get_mut(&id)
    }

    pub fn insert_npc(&mut self, id: ObjectId, spawn: SpawnNpc, config: &Config) {
        // Load drop-ins
        let mut timeline = serde_json::from_str(
            &std::fs::read_to_string(config.filesystem.locate_timeline_file("Default.json"))
                .unwrap(),
        )
        .unwrap();

        let mut search_dirs: Vec<String> = config
            .filesystem
            .additional_resource_paths
            .iter()
            .cloned()
            .map(|mut x| {
                x.push_str("/timelines/");
                x
            })
            .collect();
        search_dirs.push("resources/timelines/".to_string());

        'outer: for search_dir in search_dirs {
            for entry in std::fs::read_dir(search_dir)
                .expect("Didn't find timelines directory?")
                .flatten()
            {
                if !entry
                    .file_name()
                    .to_str()
                    .unwrap_or_default()
                    .ends_with(&format!("_{}.json", spawn.common.base_id))
                {
                    continue;
                }

                if let Ok(contents) = std::fs::read_to_string(entry.path()) {
                    timeline = serde_json::from_str(&contents).unwrap();
                    break 'outer;
                }
            }
        }

        self.actors.insert(
            id,
            NetworkedActor::Npc {
                state: NpcState::natural_state_of(&spawn),
                navmesh_path: VecDeque::default(),
                navmesh_path_lerp: 0.0,
                navmesh_target: None,
                last_position: None,
                spawn,
                timeline,
                timeline_position: 0,
                newly_hated_actor: None,
                currently_invulnerable: false,
                status_effects: StatusEffects::default(),
                last_wander_timestamp: Instant::now() + Duration::from_secs(fastrand::u64(0..60)),
            },
        );
    }

    pub fn generate_actor_id() -> ObjectId {
        // TODO: ensure we don't collide with another actor
        ObjectId(fastrand::u32(..))
    }

    /// Finds all (alive) players and NPCs. Returns their ids, positions and battalions.
    pub fn find_possible_enemies(&self) -> Vec<(ObjectId, Position, u8)> {
        self.actors
            .iter()
            .filter(|(_, y)| {
                matches!(y, NetworkedActor::Player { .. })
                    || matches!(y, NetworkedActor::Npc { .. })
            })
            .filter(|(_, y)| y.get_common_spawn().health_points > 0)
            .map(|(x, y)| {
                (
                    *x,
                    y.get_common_spawn().position,
                    y.get_common_spawn().battalion,
                )
            })
            .collect()
    }

    pub fn insert_empty_actor(&mut self, actor_id: ObjectId) {
        if self.actors.contains_key(&actor_id) {
            return;
        }

        self.actors.insert(
            actor_id,
            NetworkedActor::Player {
                spawn: SpawnPlayer::default(),
                status_effects: StatusEffects::default(),
                teleport_query: TeleportQuery::default(),
                distance_range: DistanceRange::Normal,
                conditions: Conditions::default(),
                executing_gimmick_jump: false,
                inside_instance_exit: false,
                parameters: BaseParameters::default(),
                dueling_opponent_id: ObjectId::default(),
                remove_cooldowns: false,
                last_combo_action: 0,
                combo_sequence: 0,
            },
        );
    }

    pub fn insert_object(&mut self, actor_id: ObjectId, object: SpawnObject, layer_name: String) {
        self.actors
            .insert(actor_id, NetworkedActor::Object { object, layer_name });
    }

    pub fn insert_treasure(&mut self, actor_id: ObjectId, treasure: SpawnTreasure) {
        self.actors
            .insert(actor_id, NetworkedActor::Treasure { treasure });
    }

    /// Inserts a new task into the queue, with a set `duration` and given `data`.
    pub fn insert_task(
        &mut self,
        from_id: ClientId,
        from_actor_id: ObjectId,
        duration: Duration,
        data: QueuedTaskData,
    ) {
        self.queued_task.push(QueuedTask {
            point: Instant::now() + duration,
            from_id,
            from_actor_id,
            data,
        });
    }

    /// Finds all tasks relevant to a given actor.
    pub fn find_tasks(&self, for_actor_id: ObjectId) -> Vec<QueuedTask> {
        self.queued_task
            .iter()
            .filter(|x| x.from_actor_id == for_actor_id)
            .cloned()
            .collect()
    }

    pub fn cancel_task(&mut self, network: Arc<Mutex<NetworkState>>, task: &QueuedTask) {
        // Delete the selected task:
        self.queued_task.retain(|x| x != task);

        // Then actually do the work:
        if let QueuedTaskData::CastAction { request, .. } = &task.data {
            cancel_action(network.clone(), task.from_id, request.action_id)
        }
    }

    // NOTE: this currently does *not* call cancel_action, so be careful if you're porting from cancel_task!
    pub fn retain_tasks(&mut self, f: impl Fn(&QueuedTask) -> bool) {
        // Delete the selected tasks
        self.queued_task.retain(f);
    }

    /// Cancels all queued actions for this actor.
    pub fn cancel_actor_tasks(&mut self, actor_id: ObjectId) {
        // Delete the selected task:
        self.queued_task.retain(|x| x.from_actor_id != actor_id);
    }

    /// Returns the actor ID (if any) of the spawned EObj by it's instance ID in the layout.
    pub fn find_object(&self, layout_id: u32) -> Option<ObjectId> {
        for (id, actor) in &self.actors {
            if let NetworkedActor::Object { object, .. } = actor
                && object.layout_id == layout_id
            {
                return Some(*id);
            }
        }

        None
    }

    /// Returns the actor ID (if any) of the spawned BNpc by it's instance ID in the layout.
    pub fn find_npc(&self, layout_id: u32) -> Option<(ObjectId, Position)> {
        for (id, actor) in &self.actors {
            if let NetworkedActor::Npc { spawn, .. } = actor
                && spawn.common.layout_id == layout_id
            {
                return Some((*id, spawn.common.position));
            }
        }

        None
    }

    /// Returns the actor ID (if any) of the spawned EObj by it's EObj ID.
    pub fn find_object_by_eobj_id(&self, eobj_id: u32) -> Option<ObjectId> {
        for (id, actor) in &self.actors {
            if let NetworkedActor::Object { object, .. } = actor
                && object.base_id == eobj_id
            {
                return Some(*id);
            }
        }

        None
    }

    /// Returns the actor ID (if any) of the spawned EObj by it's EObj ID and layer name.
    pub fn find_object_by_eobj_id_and_layer_name(
        &self,
        eobj_id: u32,
        eq_layer_name: &str,
    ) -> Option<ObjectId> {
        for (id, actor) in &self.actors {
            if let NetworkedActor::Object { object, layer_name } = actor
                && object.base_id == eobj_id
                && layer_name == eq_layer_name
            {
                return Some(*id);
            }
        }

        None
    }

    /// Returns the entrance circle event object (if found).
    pub fn find_entrance_circle(&self) -> Option<ObjectId> {
        // Prefer EObjs in LVD_zone_01 as that's where the circle is usually placed, otherwise other EObjs conflict such as in E8N.
        for base_id in ENTRANCE_CIRCLE_IDS {
            if let Some(id) = self.find_object_by_eobj_id_and_layer_name(base_id, "LVD_zone_01") {
                return Some(id);
            }
        }

        // Fallback to not matching by layer name...
        for base_id in ENTRANCE_CIRCLE_IDS {
            if let Some(id) = self.find_object_by_eobj_id(base_id) {
                return Some(id);
            }
        }

        None
    }

    /// Returns the base ID of the spawned EObj by it's actor ID.
    pub fn find_base_id_by_actor_id(&self, actor_id: ObjectId) -> Option<u32> {
        for (id, actor) in &self.actors {
            if *id == actor_id
                && let NetworkedActor::Object { object, .. } = actor
            {
                return Some(object.base_id);
            }
        }

        None
    }

    /// Returns the actor ID (if any) of the spawned EObj by it's Bind Layout ID.
    pub fn find_object_by_bind_layout_id(&self, bind_layout_id: u32) -> Option<ObjectId> {
        for (id, actor) in &self.actors {
            if let NetworkedActor::Object { object, .. } = actor
                && object.bind_layout_id == bind_layout_id
            {
                return Some(*id);
            }
        }

        None
    }

    pub fn inform_fate_spawn(
        network: &mut NetworkState,
        from_actor_id: ObjectId,
        fate: &FateInstance,
    ) {
        let send_motivation_npc = |network: &mut NetworkState| {
            if let Some((object_id, position)) = fate.motivation_npc {
                network.send_to_by_actor_id(
                    from_actor_id,
                    FromServer::ActorControlSelf(ActorControlCategory::SetupMotivationNpc {
                        fate_id: fate.fate_id,
                        motivation_npc: object_id,
                        unk1: 2175,
                        x: position.0.x,
                        y: position.0.y,
                        z: position.0.z,
                    }),
                    DestinationNetwork::ZoneClients,
                );
            }
        };

        // TODO: maybe only for newly spawned fates?
        network.send_to_by_actor_id(
            from_actor_id,
            FromServer::ActorControlSelf(ActorControlCategory::FateInit {
                fate_id: fate.fate_id,
                fate_state: FateState::Unk1,
            }),
            DestinationNetwork::ZoneClients,
        );

        send_motivation_npc(network);

        network.send_to_by_actor_id(
            from_actor_id,
            FromServer::ActorControlSelf(ActorControlCategory::CreateFateContext {
                fate_id: fate.fate_id,
                is_bonus: 0,
            }),
            DestinationNetwork::ZoneClients,
        );

        // TODO: We need to send this when the fate *begins* running too
        if fate.fate_state == FateState::Running {
            network.send_to_by_actor_id(
                from_actor_id,
                FromServer::PacketSegment(
                    ServerZoneIpcSegment::new(ServerZoneIpcData::UnkFate {
                        fate_id: fate.fate_id,
                        unk1: 0,
                        start_timestamp: fate.start_timestamp,
                        unk3: 0,
                        time_limit: FATE_TIME_LIMIT.as_secs() as u32,
                        unk5: 0,
                    }),
                    from_actor_id,
                ),
                DestinationNetwork::ZoneClients,
            );
        }

        network.send_to_by_actor_id(
            from_actor_id,
            FromServer::ActorControlSelf(ActorControlCategory::FateInit {
                fate_id: fate.fate_id,
                fate_state: fate.fate_state,
            }),
            DestinationNetwork::ZoneClients,
        );

        // Yes, retail does send it twice.
        send_motivation_npc(network);

        network.send_to_by_actor_id(
            from_actor_id,
            FromServer::ActorControlSelf(ActorControlCategory::FateUpdateTargetableStatus {
                fate_id: fate.fate_id,
            }),
            DestinationNetwork::ZoneClients,
        );
    }

    // TODO: should be moved to NetworkState along with above function?? maybe??
    pub fn inform_fate_spawn_globally(&self, network: &mut NetworkState, fate: &FateInstance) {
        for actor in self.actors.keys() {
            Self::inform_fate_spawn(network, *actor, fate);
        }
    }
}
