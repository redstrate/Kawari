use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

use crate::{
    common::{GameData, ObjectId},
    config::get_config,
    ipc::zone::NpcSpawn,
    world::{
        Navmesh,
        server::{actor::NetworkedActor, zone::Zone},
    },
};

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

// TODO: structure is temporary, of course
#[derive(Default, Debug)]
pub struct Instance {
    pub actors: HashMap<ObjectId, NetworkedActor>,
    pub navmesh: Navmesh,
    pub zone: Zone,
    pub weather_id: u16,
    /// If Some, then this is the path of the navmesh we need to generate.
    pub generate_navmesh: NavmeshGenerationStep,
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

    pub fn generate_actor_id() -> u32 {
        // TODO: ensure we don't collide with another actor
        fastrand::u32(..)
    }

    pub fn find_all_players(&self) -> Vec<ObjectId> {
        self.actors
            .iter()
            .filter(|(_, y)| matches!(y, NetworkedActor::Player(_)))
            .map(|(x, _)| *x)
            .collect()
    }
}
