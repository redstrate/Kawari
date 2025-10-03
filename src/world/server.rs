use binrw::{BinRead, BinWrite};
use std::{
    collections::{HashMap, VecDeque},
    env::consts::EXE_SUFFIX,
    io::Cursor,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    common::{
        CustomizeData, GameData, JumpState, MoveAnimationState, MoveAnimationType, ObjectId,
        ObjectTypeId, ObjectTypeKind, Position,
    },
    config::get_config,
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, BattleNpcSubKind,
        ClientTriggerCommand, CommonSpawn, Conditions, NpcSpawn, ObjectKind, PlayerSpawn,
        ServerZoneIpcData, ServerZoneIpcSegment,
    },
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentData, SegmentType},
};

use super::{Actor, ClientHandle, ClientId, FromServer, Navmesh, ToServer, Zone, lua::LuaZone};

use crate::world::common::SpawnKind;

/// Used for the debug NPC.
pub const CUSTOMIZE_DATA: CustomizeData = CustomizeData {
    race: 4,
    gender: 1,
    age: 1,
    height: 50,
    subrace: 7,
    face: 3,
    hair: 5,
    enable_highlights: 0,
    skin_tone: 10,
    right_eye_color: 75,
    hair_tone: 50,
    highlights: 0,
    facial_features: 1,
    facial_feature_color: 19,
    eyebrows: 1,
    left_eye_color: 75,
    eyes: 1,
    nose: 0,
    jaw: 1,
    mouth: 1,
    lips_tone_fur_pattern: 169,
    race_feature_size: 100,
    race_feature_type: 1,
    bust: 100,
    face_paint: 0,
    face_paint_color: 167,
};

#[derive(Debug, Clone)]
enum NetworkedActor {
    Player(PlayerSpawn),
    Npc {
        current_path: VecDeque<[f32; 3]>,
        current_path_lerp: f32,
        current_target: Option<ObjectId>,
        last_position: Option<Position>,
        spawn: NpcSpawn,
    },
}

impl NetworkedActor {
    pub fn get_common_spawn(&self) -> &CommonSpawn {
        match &self {
            NetworkedActor::Player(player_spawn) => &player_spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
        }
    }
}

#[derive(Default, Debug)]
enum NavmeshGenerationStep {
    /// No generation is currently happening.
    #[default]
    None,
    /// We need to generate a navmesh at this path.
    Needed(String),
    /// The process to write the navmesh has started, and we need to wait until the file exists.
    Started(String),
}

#[derive(Default, Debug)]
struct Instance {
    // structure temporary, of course
    actors: HashMap<ObjectId, NetworkedActor>,
    navmesh: Navmesh,
    zone: Zone,
    weather_id: u16,
    // If Some, then this is the path of the navmesh we need to generate.
    generate_navmesh: NavmeshGenerationStep,
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

    fn find_actor(&self, id: ObjectId) -> Option<&NetworkedActor> {
        self.actors.get(&id)
    }

    fn find_actor_mut(&mut self, id: ObjectId) -> Option<&mut NetworkedActor> {
        self.actors.get_mut(&id)
    }

    fn insert_npc(&mut self, id: ObjectId, spawn: NpcSpawn) {
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

    fn generate_actor_id() -> u32 {
        // TODO: ensure we don't collide with another actor
        fastrand::u32(..)
    }

    fn find_all_players(&self) -> Vec<ObjectId> {
        self.actors
            .iter()
            .filter(|(_, y)| matches!(y, NetworkedActor::Player(_)))
            .map(|(x, _)| *x)
            .collect()
    }
}

#[derive(Default, Debug, Clone)]
struct ClientState {}

#[derive(Default, Debug)]
struct WorldServer {
    /// Indexed by zone id
    instances: HashMap<u16, Instance>,
}

impl WorldServer {
    /// Ensures an instance exists, and creates one if not found.
    fn ensure_exists(&mut self, zone_id: u16, game_data: &mut GameData) {
        // create a new instance if necessary
        self.instances
            .entry(zone_id)
            .or_insert_with(|| Instance::new(zone_id, game_data));
    }

    /// Finds the instance associated with a zone, or None if it doesn't exist yet.
    fn find_instance(&self, zone_id: u16) -> Option<&Instance> {
        self.instances.get(&zone_id)
    }

    /// Finds the instance associated with a zone, or creates it if it doesn't exist yet
    fn find_instance_mut(&mut self, zone_id: u16) -> &mut Instance {
        self.instances.entry(zone_id).or_default()
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance(&self, actor_id: u32) -> Option<&Instance> {
        self.instances
            .values()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance_mut(&mut self, actor_id: u32) -> Option<&mut Instance> {
        self.instances
            .values_mut()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }
}

#[derive(Default, Debug)]
struct NetworkState {
    to_remove: Vec<ClientId>,
    clients: HashMap<ClientId, (ClientHandle, ClientState)>,
}

impl NetworkState {
    /// Tell all the clients that a new actor spawned.
    fn send_actor(&mut self, actor: Actor, spawn: SpawnKind) {
        // TODO: only send in the relevant instance
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            let msg = FromServer::ActorSpawn(actor, spawn.clone());

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }

    /// Inform all clients in an instance that the actor has left.
    fn inform_remove_actor(&mut self, instance: &Instance, from_id: ClientId, actor_id: u32) {
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            // Don't bother telling the client who told us
            if id == from_id {
                continue;
            }

            // Skip any clients not in this instance
            if !instance.actors.contains_key(&ObjectId(handle.actor_id)) {
                continue;
            }

            let msg = FromServer::ActorDespawn(actor_id);

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }

    fn send_to(&mut self, client_id: ClientId, message: FromServer) {
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            if id == client_id {
                if handle.send(message).is_err() {
                    self.to_remove.push(id);
                }
                break;
            }
        }
    }
}

fn set_player_minion(
    data: &mut WorldServer,
    network: &mut NetworkState,
    to_remove: &mut Vec<ClientId>,
    minion_id: u32,
    from_id: ClientId,
    from_actor_id: u32,
) {
    for (id, (handle, _)) in &mut network.clients {
        let id = *id;

        // Update our common spawn to reflect the new minion
        if id == from_id {
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                break;
            };

            let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                break;
            };

            let NetworkedActor::Player(player) = actor else {
                break;
            };

            player.common.active_minion = minion_id as u16;

            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::MinionSpawnControl { minion_id },
            });

            if handle.send(msg).is_err() {
                to_remove.push(id);
            }

            // Skip sending the regular ActorControl to ourselves
            continue;
        }

        let msg = FromServer::ActorControl(
            from_actor_id,
            ActorControl {
                category: ActorControlCategory::MinionSpawnControl { minion_id },
            },
        );

        if handle.send(msg).is_err() {
            to_remove.push(id);
        }
    }
}

fn server_logic_tick(data: &mut WorldServer, network: &mut NetworkState) {
    for instance in data.instances.values_mut() {
        let mut actor_moves = Vec::new();
        let players = instance.find_all_players();

        let mut target_actor_pos = HashMap::new();

        // const pass
        for (id, actor) in &instance.actors {
            if let NetworkedActor::Npc {
                current_path,
                current_path_lerp,
                current_target,
                spawn,
                last_position,
            } = actor
            {
                if current_target.is_some() {
                    let needs_repath = current_path.is_empty();
                    if !needs_repath {
                        // follow current path
                        let next_position = Position {
                            x: current_path[0][0],
                            y: current_path[0][1],
                            z: current_path[0][2],
                        };
                        let current_position = last_position.unwrap_or(spawn.common.pos);

                        let dir_x = current_position.x - next_position.x;
                        let dir_z = current_position.z - next_position.z;
                        let rotation = f32::atan2(-dir_z, dir_x).to_degrees();

                        actor_moves.push(FromServer::ActorMove(
                            id.0,
                            Position::lerp(current_position, next_position, *current_path_lerp),
                            rotation,
                            MoveAnimationType::RUNNING,
                            MoveAnimationState::None,
                            JumpState::NoneOrFalling,
                        ));
                    }

                    target_actor_pos.insert(
                        current_target.unwrap(),
                        instance
                            .find_actor(current_target.unwrap())
                            .unwrap()
                            .get_common_spawn()
                            .pos,
                    );
                }
            }
        }

        // mut pass
        for (id, actor) in &mut instance.actors {
            if let NetworkedActor::Npc {
                current_path,
                current_path_lerp,
                current_target,
                spawn,
                last_position,
            } = actor
            {
                // switch to the next node if we passed this one
                if *current_path_lerp >= 1.0 {
                    *current_path_lerp = 0.0;
                    if !current_path.is_empty() {
                        *last_position = Some(Position {
                            x: current_path[0][0],
                            y: current_path[0][1],
                            z: current_path[0][2],
                        });
                        current_path.pop_front();
                    }
                }

                if current_target.is_none() {
                    // find a player
                    if !players.is_empty() {
                        *current_target = Some(players[0]);
                    }
                } else if !current_path.is_empty() {
                    let next_position = Position {
                        x: current_path[0][0],
                        y: current_path[0][1],
                        z: current_path[0][2],
                    };
                    let current_position = last_position.unwrap_or(spawn.common.pos);
                    let distance = Position::distance(current_position, next_position);

                    // TODO: this doesn't work like it should
                    *current_path_lerp += (10.0 / distance).clamp(0.0, 1.0);
                }

                if target_actor_pos.contains_key(&current_target.unwrap()) {
                    let target_pos = target_actor_pos[&current_target.unwrap()];
                    let distance = Position::distance(spawn.common.pos, target_pos);
                    let needs_repath = current_path.is_empty() && distance > 5.0; // TODO: confirm distance this in retail
                    if needs_repath && current_target.is_some() {
                        let current_pos = spawn.common.pos;
                        *current_path = instance
                            .navmesh
                            .calculate_path(
                                [current_pos.x, current_pos.y, current_pos.z],
                                [target_pos.x, target_pos.y, target_pos.z],
                            )
                            .into();
                    }
                }

                // update common spawn
                for msg in &actor_moves {
                    if let FromServer::ActorMove(
                        msg_id,
                        pos,
                        rotation,
                        MoveAnimationType::RUNNING,
                        MoveAnimationState::None,
                        JumpState::NoneOrFalling,
                    ) = msg
                    {
                        if id.0 == *msg_id {
                            spawn.common.pos = *pos;
                            spawn.common.rotation = *rotation;
                        }
                    }
                }
            }
        }

        // inform clients of the NPCs new positions
        for msg in actor_moves {
            for (handle, _) in network.clients.values_mut() {
                if handle.send(msg.clone()).is_err() {
                    //to_remove.push(id);
                }
            }
        }

        // generate navmesh if necessary
        match &instance.generate_navmesh {
            NavmeshGenerationStep::None => {}
            NavmeshGenerationStep::Needed(nvm_path) => {
                tracing::info!(
                    "Missing navmesh {nvm_path:?}, we are going to generate it in the background now..."
                );

                let mut dir = std::env::current_exe().unwrap();
                dir.pop();
                dir.push(format!("kawari-navimesh{EXE_SUFFIX}"));

                // start navimesh generator
                Command::new(dir)
                    .arg(instance.zone.id.to_string())
                    .arg(nvm_path)
                    .spawn()
                    .unwrap();

                instance.generate_navmesh = NavmeshGenerationStep::Started(nvm_path.clone());
            }
            NavmeshGenerationStep::Started(nvm_path) => {
                if let Ok(nvm_bytes) = std::fs::read(nvm_path) {
                    if let Some(navmesh) = Navmesh::from_existing(&nvm_bytes) {
                        instance.navmesh = navmesh;

                        tracing::info!("Successfully loaded navimesh from {nvm_path:?}");
                    } else {
                        tracing::warn!(
                            "Failed to read {nvm_path:?}, monsters will not function correctly!"
                        );
                    }
                    instance.generate_navmesh = NavmeshGenerationStep::None;
                }
            }
        }
    }
}

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let data = Arc::new(Mutex::new(WorldServer::default()));
    let network = Arc::new(Mutex::new(NetworkState::default()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

    {
        let data = data.clone();
        let network = network.clone();
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            interval.tick().await;
            loop {
                interval.tick().await;
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                server_logic_tick(&mut data, &mut network);
            }
        });
    }

    while let Some(msg) = recv.recv().await {
        let mut to_remove = Vec::new();

        match msg {
            ToServer::NewClient(handle) => {
                tracing::info!(
                    "New client {:?} is connecting with actor id {}",
                    handle.id,
                    handle.actor_id
                );

                let mut network = network.lock().unwrap();

                network
                    .clients
                    .insert(handle.id, (handle, ClientState::default()));
            }
            ToServer::ReadySpawnPlayer(from_id, zone_id, position, rotation) => {
                let mut network = network.lock().unwrap();
                let mut data = data.lock().unwrap();

                // create a new instance if necessary
                let mut game_data = game_data.lock().unwrap();
                data.ensure_exists(zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(zone_id);

                // tell the client to load into the zone
                let msg = FromServer::ChangeZone(
                    zone_id,
                    target_instance.weather_id,
                    position,
                    rotation,
                    LuaZone::from_zone(&target_instance.zone, target_instance.weather_id),
                    true, // since this is initial login
                );

                network.send_to(from_id, msg);
            }
            ToServer::ZoneLoaded(from_id, zone_id, player_spawn) => {
                tracing::info!(
                    "Client {from_id:?} has now loaded, sending them existing player data."
                );

                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                // Send existing player data to the connection, if any
                if let Some(instance) = data.find_instance(zone_id) {
                    // send existing player data
                    for (id, spawn) in &instance.actors {
                        let kind = match spawn {
                            NetworkedActor::Player(spawn) => SpawnKind::Player(spawn.clone()),
                            NetworkedActor::Npc { spawn, .. } => {
                                // TODO: Do we actually care about NPCs here if we're only sending *player* data?
                                SpawnKind::Npc(spawn.clone())
                            }
                        };

                        let msg = FromServer::ActorSpawn(
                            Actor {
                                id: *id,
                                hp: 100,
                                spawn_index: 0,
                            },
                            kind,
                        );

                        network.send_to(from_id, msg);
                    }
                }

                let (client, _) = network.clients.get(&from_id).unwrap().clone();

                if let Some(instance) = data.find_instance(zone_id) {
                    // Then tell any clients in the zone that we spawned
                    for (id, (handle, _)) in &mut network.clients {
                        let id = *id;

                        // don't bother telling the client who told us
                        if id == from_id {
                            continue;
                        }

                        // skip any clients not in our zone
                        if !instance.actors.contains_key(&ObjectId(handle.actor_id)) {
                            continue;
                        }

                        let msg = FromServer::ActorSpawn(
                            Actor {
                                id: ObjectId(client.actor_id),
                                hp: 0,
                                spawn_index: 0,
                            },
                            SpawnKind::Player(player_spawn.clone()),
                        );

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    }
                }

                // add the connection's actor to the table
                {
                    let instance = data.find_instance_mut(zone_id);
                    instance.actors.insert(
                        ObjectId(client.actor_id),
                        NetworkedActor::Player(player_spawn),
                    );
                }
            }
            ToServer::ChangeZone(from_id, actor_id, zone_id) => {
                tracing::info!("Client {from_id:?} is requesting to go to {zone_id}!");

                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                // create a new instance if necessary
                let mut game_data = game_data.lock().unwrap();
                data.ensure_exists(zone_id, &mut game_data);

                // inform the players in this zone that this actor left
                if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                    current_instance.actors.remove(&ObjectId(actor_id));
                    network.inform_remove_actor(current_instance, from_id, actor_id);
                }

                // then find or create a new instance with the zone id
                data.ensure_exists(zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(zone_id);

                // tell the client to load into the zone
                let msg = FromServer::ChangeZone(
                    zone_id,
                    target_instance.weather_id,
                    Position::default(),
                    0.0,
                    LuaZone::from_zone(&target_instance.zone, target_instance.weather_id),
                    false,
                );
                network.send_to(from_id, msg);
            }
            ToServer::EnterZoneJump(from_id, actor_id, exitbox_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                // first, find the zone jump in the current zone
                let destination_zone_id;
                let destination_instance_id;
                if let Some(current_instance) = data.find_actor_instance(actor_id) {
                    let Some((_, new_exit_box)) = current_instance.zone.find_exit_box(exitbox_id)
                    else {
                        tracing::warn!("Couldn't find exit box {exitbox_id}?!");
                        break;
                    };
                    destination_zone_id = new_exit_box.territory_type;
                    destination_instance_id = new_exit_box.destination_instance_id;
                } else {
                    tracing::warn!(
                        "Actor isn't in the instance it was expected in. This is a bug!"
                    );
                    break;
                }

                // inform the players in this zone that this actor left
                if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                    current_instance.actors.remove(&ObjectId(actor_id));
                    network.inform_remove_actor(current_instance, from_id, actor_id);
                }

                // then find or create a new instance with the zone id
                let mut game_data = game_data.lock().unwrap();
                data.ensure_exists(destination_zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(destination_zone_id);

                // TODO: this same code is 99% the same for zone jumps, aetherytes and warps. it should be consolidated!
                let exit_position;
                if let Some((destination_object, _)) =
                    target_instance.zone.find_pop_range(destination_instance_id)
                {
                    exit_position = Position {
                        x: destination_object.transform.translation[0],
                        y: destination_object.transform.translation[1],
                        z: destination_object.transform.translation[2],
                    };
                } else {
                    exit_position = Position::default();
                }

                // now that we have all of the data needed, inform the connection of where they need to be
                let msg = FromServer::ChangeZone(
                    destination_zone_id,
                    target_instance.weather_id,
                    exit_position,
                    0.0, // TODO: exit rotation
                    LuaZone::from_zone(&target_instance.zone, target_instance.weather_id),
                    false,
                );
                network.send_to(from_id, msg);
            }
            ToServer::Warp(from_id, actor_id, warp_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                let mut game_data = game_data.lock().unwrap();

                // first, find the warp and it's destination
                let (destination_instance_id, destination_zone_id) = game_data
                    .get_warp(warp_id)
                    .expect("Failed to find the warp!");

                // inform the players in this zone that this actor left
                if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                    current_instance.actors.remove(&ObjectId(actor_id));
                    network.inform_remove_actor(current_instance, from_id, actor_id);
                }

                // then find or create a new instance with the zone id
                data.ensure_exists(destination_zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(destination_zone_id);

                let exit_position;
                if let Some((destination_object, _)) =
                    target_instance.zone.find_pop_range(destination_instance_id)
                {
                    exit_position = Position {
                        x: destination_object.transform.translation[0],
                        y: destination_object.transform.translation[1],
                        z: destination_object.transform.translation[2],
                    };
                } else {
                    exit_position = Position::default();
                }

                // now that we have all of the data needed, inform the connection of where they need to be
                let msg = FromServer::ChangeZone(
                    destination_zone_id,
                    target_instance.weather_id,
                    exit_position,
                    0.0, // TODO: exit rotation
                    LuaZone::from_zone(&target_instance.zone, target_instance.weather_id),
                    false,
                );
                network.send_to(from_id, msg);
            }
            ToServer::WarpAetheryte(from_id, actor_id, aetheryte_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();
                let mut game_data = game_data.lock().unwrap();

                // first, find the warp and it's destination
                let (destination_instance_id, destination_zone_id) = game_data
                    .get_aetheryte(aetheryte_id)
                    .expect("Failed to find the aetheryte!");

                // inform the players in this zone that this actor left
                if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                    current_instance.actors.remove(&ObjectId(actor_id));
                    network.inform_remove_actor(current_instance, from_id, actor_id);
                }

                // then find or create a new instance with the zone id
                data.ensure_exists(destination_zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(destination_zone_id);

                let exit_position;
                if let Some((destination_object, _)) =
                    target_instance.zone.find_pop_range(destination_instance_id)
                {
                    exit_position = Position {
                        x: destination_object.transform.translation[0],
                        y: destination_object.transform.translation[1],
                        z: destination_object.transform.translation[2],
                    };
                } else {
                    exit_position = Position::default();
                }

                // now that we have all of the data needed, inform the connection of where they need to be
                let msg = FromServer::ChangeZone(
                    destination_zone_id,
                    target_instance.weather_id,
                    exit_position,
                    0.0, // TODO: exit rotation
                    LuaZone::from_zone(&target_instance.zone, target_instance.weather_id),
                    false,
                );
                network.send_to(from_id, msg);
            }
            ToServer::Message(from_id, msg) => {
                let mut network = network.lock().unwrap();

                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    if id == from_id {
                        continue;
                    }

                    let msg = FromServer::Message(msg.clone());

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::ActorMoved(
                from_id,
                actor_id,
                position,
                rotation,
                anim_type,
                anim_state,
                jump_state,
            ) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                if let Some(instance) = data.find_actor_instance_mut(actor_id) {
                    if let Some((_, spawn)) = instance
                        .actors
                        .iter_mut()
                        .find(|actor| *actor.0 == ObjectId(actor_id))
                    {
                        let common = match spawn {
                            NetworkedActor::Player(npc_spawn) => &mut npc_spawn.common,
                            NetworkedActor::Npc { spawn, .. } => &mut spawn.common,
                        };
                        common.pos = position;
                        common.rotation = rotation;
                    }

                    for (id, (handle, _)) in &mut network.clients {
                        let id = *id;

                        if id == from_id {
                            continue;
                        }

                        let msg = FromServer::ActorMove(
                            actor_id, position, rotation, anim_type, anim_state, jump_state,
                        );

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    }
                }
            }
            ToServer::ClientTrigger(from_id, from_actor_id, trigger) => {
                let mut network = network.lock().unwrap();

                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    tracing::info!("{:#X?}", trigger);

                    // handle player-to-server actions
                    if id == from_id {
                        if let ClientTriggerCommand::TeleportQuery { aetheryte_id } =
                            &trigger.trigger
                        {
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::TeleportStart {
                                    insufficient_gil: 0,
                                    aetheryte_id: *aetheryte_id,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::EventRelatedUnk { .. } = &trigger.trigger {
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::EventRelatedUnk1 { unk1: 1 },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::EventRelatedUnk2 { unk1: 0 },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::WalkInTriggerFinished { .. } = &trigger.trigger
                        {
                            // This is where we finally release the client after the walk-in trigger.
                            let msg = FromServer::Conditions(Conditions::default());

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::WalkInTriggerRelatedUnk1 {
                                    unk1: 0,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            // Yes, this is actually sent every time the trigger event finishes...
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::CompanionUnlock {
                                    unk1: 0,
                                    unk2: 1,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::WalkInTriggerRelatedUnk2 {
                                    unk1: 0,
                                    unk2: 0,
                                    unk3: 0,
                                    unk4: 7,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::SummonMinion { minion_id } = &trigger.trigger {
                            let msg = FromServer::ActorSummonsMinion(*minion_id);

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::DespawnMinion { .. } = &trigger.trigger {
                            let msg = FromServer::ActorDespawnsMinion();

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        continue;
                    }

                    match &trigger.trigger {
                        ClientTriggerCommand::SetTarget {
                            actor_id,
                            actor_type,
                        } => {
                            // For whatever reason these don't match what the server has to send back, so they cannot be directly reused.
                            let actor_type = match *actor_type {
                                0 => ObjectTypeKind::None,
                                1 => ObjectTypeKind::EObjOrNpc,
                                2 => ObjectTypeKind::Minion,
                                _ => {
                                    // TODO: Are there other types?
                                    tracing::warn!(
                                        "SetTarget: Unknown actor target type {}! Defaulting to None!",
                                        *actor_type
                                    );
                                    ObjectTypeKind::None
                                }
                            };

                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::SetTarget {
                                        target: ObjectTypeId {
                                            object_id: *actor_id,
                                            object_type: actor_type,
                                        },
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ChangePose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Pose {
                                        unk1: *unk1,
                                        pose: *pose,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ReapplyPose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Pose {
                                        unk1: *unk1,
                                        pose: *pose,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::Emote(emote_info) => {
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::Emote(*emote_info),
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ToggleWeapon { shown, unk_flag } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::ToggleWeapon {
                                        shown: *shown,
                                        unk_flag: *unk_flag,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ManuallyRemoveEffect {
                            effect_id,
                            source_actor_id,
                            ..
                        } => {
                            let msg =
                                FromServer::LoseEffect(*effect_id as u16, 0, *source_actor_id);

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        _ => tracing::warn!("Server doesn't know what to do with {:#?}", trigger),
                    }
                }
            }
            ToServer::DebugNewNpc(_from_id, from_actor_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: CommonSpawn {
                            hp_curr: 100,
                            hp_max: 100,
                            mp_curr: 100,
                            mp_max: 100,
                            look: CUSTOMIZE_DATA,
                            bnpc_base: 13498,
                            bnpc_name: 10261,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            target_id: ObjectTypeId {
                                object_id: ObjectId(from_actor_id),
                                object_type: ObjectTypeKind::None,
                            }, // target the player
                            level: 1,
                            models: [
                                0,  // head
                                89, // body
                                89, // hands
                                89, // legs
                                89, // feet
                                0,  // ears
                                0,  // neck
                                0,  // wrists
                                0,  // left finger
                                0,  // right finger
                            ],
                            pos: player.common.pos,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::DebugNewEnemy(_from_id, from_actor_id, id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    let model_chara;
                    {
                        let mut game_data = game_data.lock().unwrap();
                        model_chara = game_data.find_bnpc(id).unwrap();
                    }

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: CommonSpawn {
                            hp_curr: 91,
                            hp_max: 91,
                            mp_curr: 100,
                            mp_max: 100,
                            spawn_index: 0, // not needed at this level
                            bnpc_base: id,
                            bnpc_name: 405,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            level: 1,
                            battalion: 4,
                            model_chara,
                            pos: player.common.pos,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::DebugSpawnClone(_from_id, from_actor_id) => {
                let mut data = data.lock().unwrap();
                let mut network = network.lock().unwrap();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: player.common.clone(),
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::ActionRequest(from_id, _from_actor_id, request) => {
                let mut game_data = game_data.lock().unwrap();
                let cast_time = game_data.get_casttime(request.action_key).unwrap();

                let send_execution = |from_id: ClientId, network: Arc<Mutex<NetworkState>>| {
                    let mut network = network.lock().unwrap();

                    tracing::info!("Now finishing delayed cast!");

                    let msg = FromServer::ActionComplete(request);
                    network.send_to(from_id, msg);
                };

                if cast_time == 0 {
                    // If instantaneous, send right back
                    send_execution(from_id, network.clone());
                } else {
                    // Otherwise, delay
                    // NOTE: I know this won't scale, but it's a fine hack for now

                    tracing::info!(
                        "Delaying spell cast for {} milliseconds",
                        cast_time as u64 * 100
                    );

                    // we have to shadow these variables to tell rust not to move them into the async closure
                    let network = network.clone();
                    tokio::task::spawn(async move {
                        let mut interval =
                            tokio::time::interval(Duration::from_millis(cast_time as u64 * 100));
                        interval.tick().await;
                        interval.tick().await;
                        send_execution(from_id, network);
                    });
                }
            }
            ToServer::Config(_from_id, from_actor_id, config) => {
                // update their stored state so it's correctly sent on new spawns
                {
                    let mut data = data.lock().unwrap();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    player.common.display_flags = config.display_flag.into();
                }

                let mut network = network.lock().unwrap();
                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    let msg = FromServer::UpdateConfig(from_actor_id, config.clone());

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::Equip(_from_id, from_actor_id, main_weapon_id, sub_weapon_id, model_ids) => {
                // update their stored state so it's correctly sent on new spawns
                {
                    let mut data = data.lock().unwrap();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    player.common.main_weapon_model = main_weapon_id;
                    player.common.sec_weapon_model = sub_weapon_id;
                    player.common.models = model_ids;
                }

                // Inform all clients about their new equipped model ids
                let mut network = network.lock().unwrap();
                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    let msg = FromServer::ActorEquip(
                        from_actor_id,
                        main_weapon_id,
                        sub_weapon_id,
                        model_ids,
                    );

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::BeginReplay(from_id, path) => {
                let mut entries = std::fs::read_dir(path)
                    .unwrap()
                    .map(|res| res.map(|e| e.path()))
                    .collect::<Result<Vec<_>, std::io::Error>>()
                    .unwrap();

                entries.sort_by(|a, b| {
                    let a_seq = a
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split_once('-')
                        .unwrap()
                        .0
                        .parse::<i32>()
                        .unwrap();
                    let b_seq = b
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split_once('-')
                        .unwrap()
                        .0
                        .parse::<i32>()
                        .unwrap();

                    a_seq.cmp(&b_seq)
                });

                let send_execution = |from_id: ClientId,
                                      network: Arc<Mutex<NetworkState>>,
                                      entry: &PathBuf| {
                    let mut network = network.lock().unwrap();

                    // only care about Ipc packets
                    let filename = entry.file_name().unwrap().to_str().unwrap();
                    if !filename.contains("Ipc") {
                        return;
                    }

                    // only care about packets from the server
                    if !filename.contains("(to client)") {
                        return;
                    }

                    let path = entry.to_str().unwrap();

                    tracing::info!("- Replaying {path}");

                    let source_actor_bytes =
                        std::fs::read(format!("{path}/source_actor.bin")).unwrap();
                    let target_actor_bytes =
                        std::fs::read(format!("{path}/target_actor.bin")).unwrap();
                    let source_actor =
                        u32::from_le_bytes(source_actor_bytes[0..4].try_into().unwrap());
                    let target_actor =
                        u32::from_le_bytes(target_actor_bytes[0..4].try_into().unwrap());

                    let ipc_header_bytes = std::fs::read(format!("{path}/ipc_header.bin")).unwrap();
                    let opcode = u16::from_le_bytes(ipc_header_bytes[2..4].try_into().unwrap());

                    let mut ipc_data = std::fs::read(format!("{path}/data.bin")).unwrap();
                    let ipc_len = ipc_data.len() as u32 + 32;
                    let mut cursor = Cursor::new(&mut ipc_data);
                    if let Ok(parsed) = ServerZoneIpcSegment::read_le_args(&mut cursor, (&ipc_len,))
                    {
                        if let ServerZoneIpcData::InitZone(mut init_zone) = parsed.data {
                            tracing::info!("- Fixing up InitZone");

                            // stop it from trying to initialize obsfucation
                            init_zone.obsfucation_mode = 0;
                            init_zone.seed1 = 0;
                            init_zone.seed2 = 0;
                            init_zone.seed3 = 0;

                            let mut cursor = Cursor::new(Vec::new());
                            init_zone.write_le(&mut cursor).unwrap();
                            ipc_data = cursor.into_inner().to_vec();
                        }
                    }

                    let msg = FromServer::ReplayPacket(PacketSegment {
                        source_actor,
                        target_actor,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ServerZoneIpcSegment {
                            op_code: ServerZoneIpcType::Unknown(opcode),
                            data: ServerZoneIpcData::Unknown { unk: ipc_data },
                            ..Default::default()
                        }),
                    });
                    network.send_to(from_id, msg);
                };

                let network = network.clone();
                tokio::task::spawn(async move {
                    for entry in &entries {
                        let mut interval = tokio::time::interval(Duration::from_millis(100));
                        interval.tick().await;
                        interval.tick().await;
                        send_execution(from_id, network.clone(), entry);
                    }
                });
            }
            ToServer::GainEffect(
                from_id,
                _from_actor_id,
                effect_id,
                effect_duration,
                effect_param,
                effect_source_actor_id,
            ) => {
                let send_lost_effect =
                    |from_id: ClientId,
                     network: Arc<Mutex<NetworkState>>,
                     effect_id: u16,
                     effect_param: u16,
                     effect_source_actor_id: ObjectId| {
                        let mut network = network.lock().unwrap();

                        tracing::info!("Now losing effect {}!", effect_id);

                        let msg =
                            FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id);
                        network.send_to(from_id, msg);
                    };

                // Eventually tell the player they lost this effect
                // NOTE: I know this won't scale, but it's a fine hack for now

                tracing::info!("Effect {effect_id} lasts for {effect_duration} seconds");

                // we have to shadow these variables to tell rust not to move them into the async closure
                let network = network.clone();
                tokio::task::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(
                        (effect_duration * 1000.0) as u64,
                    ));
                    interval.tick().await;
                    interval.tick().await;
                    send_lost_effect(
                        from_id,
                        network,
                        effect_id,
                        effect_param,
                        effect_source_actor_id,
                    );
                });
            }
            ToServer::ZoneIn(from_id, from_actor_id, is_teleport) => {
                // Inform all clients to play the zone in animation
                let mut network = network.lock().unwrap();
                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    if id == from_id {
                        let msg = FromServer::ActorControlSelf(ActorControlSelf {
                            category: ActorControlCategory::ZoneIn {
                                warp_finish_anim: 1,
                                raise_anim: 0,
                                unk1: if is_teleport { 110 } else { 0 },
                            },
                        });

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    } else {
                        // FIXME: do you see teleport animations from other players?
                        let msg = FromServer::ActorControl(
                            from_actor_id,
                            ActorControl {
                                category: ActorControlCategory::ZoneIn {
                                    warp_finish_anim: 1,
                                    raise_anim: 0,
                                    unk1: 0,
                                },
                            },
                        );

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    }
                }
            }
            ToServer::Disconnected(from_id) => {
                let mut network = network.lock().unwrap();

                network.to_remove.push(from_id);
            }
            ToServer::ActorSummonsMinion(from_id, from_actor_id, minion_id) => {
                let mut network = network.lock().unwrap();
                let mut data = data.lock().unwrap();

                set_player_minion(
                    &mut data,
                    &mut network,
                    &mut to_remove,
                    minion_id,
                    from_id,
                    from_actor_id,
                );
            }
            ToServer::ActorDespawnsMinion(from_id, from_actor_id) => {
                let mut network = network.lock().unwrap();
                let mut data = data.lock().unwrap();

                set_player_minion(
                    &mut data,
                    &mut network,
                    &mut to_remove,
                    0,
                    from_id,
                    from_actor_id,
                );
            }
            ToServer::FatalError(err) => return Err(err),
        }

        // Remove any clients that errored out
        {
            let mut network = network.lock().unwrap();
            let mut data = data.lock().unwrap();

            network.to_remove.append(&mut to_remove);

            for remove_id in network.to_remove.clone() {
                // remove any actors they had
                let mut actor_id = None;
                for (id, (handle, _)) in &mut network.clients {
                    if *id == remove_id {
                        actor_id = Some(handle.actor_id);
                    }
                }

                if let Some(actor_id) = actor_id {
                    // remove them from the instance
                    if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                        current_instance.actors.remove(&ObjectId(actor_id));
                        network.inform_remove_actor(current_instance, remove_id, actor_id);
                    }
                }

                network.clients.remove(&remove_id);
            }
        }
    }
    Ok(())
}
