use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    ClientId, Navmesh, StatusEffects,
    server::{action::cancel_action, actor::NetworkedActor, network::NetworkState, zone::Zone},
    zone_connection::TeleportQuery,
};
use kawari::{
    common::{DistanceRange, GameData, ObjectId},
    config::get_config,
    ipc::zone::{ActionRequest, Conditions, NpcSpawn, ObjectSpawn, PlayerSpawn},
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
    LoseStatusEffect {
        effect_id: u16,
        effect_param: u16,
        effect_source_actor_id: ObjectId,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedTask {
    pub point: Instant,
    pub from_id: ClientId,
    pub from_actor_id: ObjectId,
    pub data: QueuedTaskData,
}

// TODO: structure is temporary, of course
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
        for object in instance.zone.get_event_objects(game_data) {
            instance.insert_object(object.entity_id, object);
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
        self.actors.insert(
            id,
            NetworkedActor::Npc {
                current_path: VecDeque::default(),
                current_path_lerp: 0.0,
                current_target: None,
                last_position: None,
                spawn,
            },
        );
    }

    pub fn generate_actor_id() -> ObjectId {
        // TODO: ensure we don't collide with another actor
        ObjectId(fastrand::u32(..))
    }

    pub fn find_all_players(&self) -> Vec<ObjectId> {
        self.actors
            .iter()
            .filter(|(_, y)| matches!(y, NetworkedActor::Player { .. }))
            .map(|(x, _)| *x)
            .collect()
    }

    pub fn insert_empty_actor(&mut self, actor_id: ObjectId) {
        self.actors.insert(
            actor_id,
            NetworkedActor::Player {
                spawn: PlayerSpawn::default(),
                status_effects: StatusEffects::default(),
                teleport_query: TeleportQuery::default(),
                distance_range: DistanceRange::Normal,
                conditions: Conditions::default(),
                executing_gimmick_jump: false,
            },
        );
    }

    pub fn insert_object(&mut self, actor_id: ObjectId, object: ObjectSpawn) {
        self.actors
            .insert(actor_id, NetworkedActor::Object { object });
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
        match task.data {
            QueuedTaskData::CastAction { .. } => cancel_action(network.clone(), task.from_id),
            QueuedTaskData::LoseStatusEffect { .. } => {} // Nothing needs to happen for status effects
        }
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
}
