use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use crate::{
    ClientId, FromServer, GameData, Navmesh, StatusEffects,
    server::{
        action::{cancel_action, clear_action_cooldowns},
        actor::{NetworkedActor, NpcState},
        combat_state::PlayerCombatState,
        director::DirectorData,
        network::{DestinationNetwork, NetworkState},
        zone::Zone,
    },
    zone_connection::{BaseParameters, TeleportQuery},
};
use kawari::{
    common::{DistanceRange, ENTRANCE_CIRCLE_IDS, ObjectId, Position, Timeline},
    config::{FilesystemConfig, get_config},
    ipc::zone::{
        ActionRequest, ActorControlCategory, Conditions, ServerZoneIpcSegment, SpawnNpc,
        SpawnObject, SpawnPlayer, SpawnTreasure,
    },
};
use parking_lot::Mutex;

static DEFAULT_TIMELINE: OnceLock<Timeline> = OnceLock::new();
static BASE_TIMELINES: OnceLock<HashMap<u32, Timeline>> = OnceLock::new();

fn default_timeline() -> Timeline {
    DEFAULT_TIMELINE
        .get_or_init(|| {
            serde_json::from_str(
                &std::fs::read_to_string(FilesystemConfig::locate_timeline_file("Default.json"))
                    .unwrap(),
            )
            .unwrap()
        })
        .clone()
}

fn base_timelines() -> &'static HashMap<u32, Timeline> {
    BASE_TIMELINES.get_or_init(|| {
        let mut timelines = HashMap::new();
        let mut search_dirs: Vec<String> = get_config()
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

        for search_dir in search_dirs {
            let Ok(entries) = std::fs::read_dir(search_dir) else {
                continue;
            };

            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let Some(file_name) = file_name.to_str() else {
                    continue;
                };
                let Some(base_id) = file_name
                    .strip_suffix(".json")
                    .and_then(|name| name.rsplit_once('_'))
                    .and_then(|(_, base_id)| base_id.parse::<u32>().ok())
                else {
                    continue;
                };
                let Ok(contents) = std::fs::read_to_string(entry.path()) else {
                    continue;
                };
                let Ok(timeline) = serde_json::from_str(&contents) else {
                    continue;
                };

                timelines.entry(base_id).or_insert(timeline);
            }
        }

        timelines
    })
}

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
    CastEnemyAction {
        request: ActionRequest,
    },
    LoseStatusEffect {
        effect_id: u16,
        effect_param: u16,
        effect_source_actor_id: ObjectId,
    },
    /// Fade out a dead actor.
    DeadFadeOut {
        actor_id: ObjectId,
    },
    /// Despawn a dead actor.
    DeadDespawn {
        actor_id: ObjectId,
    },
    /// Complete an EventAction
    CastEventAction {
        target: ObjectId,
    },
    /// Make a fish bite.
    FishBite,
    /// Seal a boss wall.
    SealBossWall {
        id: u32,
        place_name: u32,
    },
    /// Generically send a packet segment, only used for `do_change_zone`. Don't abuse this as a generic task, you almost certainly want to create a new variant.
    PacketSegment {
        segment: ServerZoneIpcSegment,
    },
    /// Used by directors since its tough to fit this into the director logic.
    WarpToPopRange {
        id: u32,
    },
    /// Finalize a pet summon after the actor has been visible to the client for a short moment.
    RevealPet {
        actor_id: ObjectId,
    },
    /// Resolve a short-lived Summoner elemental primal's finisher. If the owner has no attackable
    /// target yet, this task retries within the 8s summon window, then reverts to carbuncle without
    /// damage.
    SummonerPrimalFinisher {
        owner_id: ObjectId,
        pet_id: ObjectId,
        preferred_target_id: ObjectId,
        action_id: u32,
        potency: u32,
        expires_at: Instant,
    },
    /// Resolve one 3s tick of a demi-summon's automatic attack.
    SummonerDemiAutoAttack {
        owner_id: ObjectId,
    },
    /// Revert an elemental primal back to carbuncle after its finisher animation has had time to
    /// play on the client.
    SummonerPrimalRevert {
        owner_id: ObjectId,
        pet_id: ObjectId,
    },
    /// Resolve one tick of Summoner Slipstream's lingering ground AoE.
    SummonerSlipstreamTick {
        owner_id: ObjectId,
        center: Position,
        radius: f32,
        potency: u32,
        ticks_remaining: u8,
    },
    /// Spawn Summoner Slipstream's lingering ground VFX.
    SummonerSlipstreamGroundVfx {
        owner_id: ObjectId,
        center: Position,
    },
    /// Remove Summoner Slipstream's lingering ground VFX actor.
    SummonerSlipstreamGroundVfxCleanup {
        object_id: ObjectId,
    },
    /// Reset a player's action combo status.
    ResetCombo,
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
        for npc in instance.zone.get_npcs(game_data) {
            instance.insert_npc(ObjectId(fastrand::u32(..)), npc);
        }

        instance
    }

    pub fn find_actor(&self, id: ObjectId) -> Option<&NetworkedActor> {
        self.actors.get(&id)
    }

    pub fn find_actor_mut(&mut self, id: ObjectId) -> Option<&mut NetworkedActor> {
        self.actors.get_mut(&id)
    }

    pub fn insert_npc(&mut self, id: ObjectId, spawn: SpawnNpc) {
        let timeline = base_timelines()
            .get(&spawn.common.base_id)
            .cloned()
            .unwrap_or_else(default_timeline);

        let spawn_position = spawn.common.position.0;
        self.actors.insert(
            id,
            NetworkedActor::Npc {
                state: NpcState::natural_state_of(&spawn),
                navmesh_path: VecDeque::default(),
                navmesh_path_lerp: 0.0,
                navmesh_target: None,
                last_position: None,
                spawn_position,
                spawn,
                timeline,
                timeline_position: 0,
                hate_list: HashMap::new(),
                currently_invulnerable: false,
                ai_paused: false,
                targetable: true,
                visible: true,
                cast_locked: false,
                status_effects: StatusEffects::default(),
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
                combat_state: PlayerCombatState::default(),
                last_combo_action: 0,
                combo_sequence: 0,
                hated_by: HashMap::new(),
                last_enmity_sent: Vec::new(),
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

    pub(super) fn cancel_task(
        &mut self,
        network: Arc<Mutex<NetworkState>>,
        game_data: &mut GameData,
        task: &QueuedTask,
    ) {
        // Delete the selected task:
        self.queued_task.retain(|x| x != task);

        // Then actually do the work:
        if let QueuedTaskData::CastAction { request, .. } = &task.data {
            let cleared_cooldown_groups =
                if let Some(actor) = self.find_actor_mut(task.from_actor_id) {
                    clear_action_cooldowns(actor, game_data, request.action_id)
                } else {
                    Vec::new()
                };
            if !cleared_cooldown_groups.is_empty() {
                let mut network = network.lock();
                for cooldown_group in cleared_cooldown_groups {
                    network.send_to(
                        task.from_id,
                        FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                            cooldown_group,
                            elapsed_centisec: 0,
                            total_centisec: 0,
                        }),
                        DestinationNetwork::ZoneClients,
                    );
                }
            }
            cancel_action(
                network.clone(),
                task.from_id,
                None,
                Some(request.action_type),
                Some(request.action_id),
                None,
            )
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
}
