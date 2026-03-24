use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    ClientId, GameData, Navmesh, StatusEffects,
    server::{
        action::cancel_action,
        actor::{NetworkedActor, NpcState},
        director::DirectorData,
        network::NetworkState,
        zone::Zone,
    },
    zone_connection::{BaseParameters, TeleportQuery},
};
use kawari::{
    common::{DistanceRange, ENTRANCE_CIRCLE_IDS, ObjectId, Position},
    config::{FilesystemConfig, get_config},
    ipc::zone::{ActionRequest, Conditions, NpcSpawn, ObjectSpawn, PlayerSpawn, SpawnTreasure},
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

#[derive(Debug, Clone, PartialEq)]
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
    FishBite {},
    /// Seal a boss wall.
    SealBossWall {
        id: u32,
        place_name: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedTask {
    pub point: Instant,
    pub from_id: ClientId,
    pub from_actor_id: ObjectId,
    pub data: QueuedTaskData,
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
        for object in instance.zone.get_event_objects(game_data, false) {
            instance.insert_object(object.entity_id, object);
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

    pub fn insert_npc(&mut self, id: ObjectId, spawn: NpcSpawn) {
        // Load drop-ins
        let mut timeline = serde_json::from_str(
            &std::fs::read_to_string(FilesystemConfig::locate_timeline_file("Default.json"))
                .unwrap(),
        )
        .unwrap();

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

        'outer: for search_dir in search_dirs {
            for entry in std::fs::read_dir(search_dir)
                .expect("Didn't find timelines directory?")
                .flatten()
            {
                if !entry
                    .file_name()
                    .to_str()
                    .unwrap_or_default()
                    .ends_with(&format!("_{}.json", spawn.common.npc_base))
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
                state: NpcState::Wander,
                current_path: VecDeque::default(),
                current_path_lerp: 0.0,
                current_target: None,
                last_position: None,
                spawn,
                timeline,
                timeline_position: 0,
                newly_hated_actor: None,
            },
        );
    }

    pub fn generate_actor_id() -> ObjectId {
        // TODO: ensure we don't collide with another actor
        ObjectId(fastrand::u32(..))
    }

    pub fn find_all_players(&self) -> Vec<(ObjectId, Position)> {
        self.actors
            .iter()
            .filter(|(_, y)| matches!(y, NetworkedActor::Player { .. }))
            .map(|(x, y)| (*x, y.get_common_spawn().position))
            .collect()
    }

    pub fn insert_empty_actor(&mut self, actor_id: ObjectId) {
        if self.actors.contains_key(&actor_id) {
            return;
        }

        self.actors.insert(
            actor_id,
            NetworkedActor::Player {
                spawn: PlayerSpawn::default(),
                status_effects: StatusEffects::default(),
                teleport_query: TeleportQuery::default(),
                distance_range: DistanceRange::Normal,
                conditions: Conditions::default(),
                executing_gimmick_jump: false,
                inside_instance_exit: false,
                parameters: BaseParameters::default(),
                dueling_opponent_id: ObjectId::default(),
                remove_cooldowns: false,
            },
        );
    }

    pub fn insert_object(&mut self, actor_id: ObjectId, object: ObjectSpawn) {
        self.actors
            .insert(actor_id, NetworkedActor::Object { object });
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
        tracing::info!("Removing task {task:#?} from the schedule!");

        // Delete the selected task:
        self.queued_task.retain(|x| x != task);

        // Then actually do the work:
        if let QueuedTaskData::CastAction { .. } = task.data {
            cancel_action(network.clone(), task.from_id)
        }
    }

    /// Cancels all queued actions for this actor.
    pub fn cancel_actor_tasks(&mut self, actor_id: ObjectId) {
        tracing::info!("Removing tasks for {actor_id} from the schedule!");

        // Delete the selected task:
        self.queued_task.retain(|x| x.from_actor_id != actor_id);
    }

    /// Returns the actor ID (if any) of the spawned EObj by it's instance ID in the layout.
    pub fn find_object(&self, layout_id: u32) -> Option<ObjectId> {
        for (id, actor) in &self.actors {
            if let NetworkedActor::Object { object } = actor
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
            if let NetworkedActor::Object { object } = actor
                && object.base_id == eobj_id
            {
                return Some(*id);
            }
        }

        None
    }

    /// Returns the entrance circle event object (if found).
    pub fn find_entrance_circle(&self) -> Option<ObjectId> {
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
                && let NetworkedActor::Object { object } = actor
            {
                return Some(object.base_id);
            }
        }

        None
    }
}
