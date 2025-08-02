use binrw::{BinRead, BinWrite};
use icarus::TerritoryType::TerritoryTypeSheet;
use physis::{common::Language, lvb::Lvb, resource::Resource};
use std::{
    collections::{HashMap, VecDeque},
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    common::{CustomizeData, GameData, ObjectId, ObjectTypeId, Position},
    config::get_config,
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, BattleNpcSubKind,
        ClientTriggerCommand, CommonSpawn, Conditions, NpcSpawn, ObjectKind, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentData, SegmentType},
};

use super::{Actor, ClientHandle, ClientId, FromServer, Navmesh, ToServer};

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
    Player(NpcSpawn),
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
            NetworkedActor::Player(npc_spawn) => &npc_spawn.common,
            NetworkedActor::Npc { spawn, .. } => &spawn.common,
        }
    }
}

#[derive(Default, Debug, Clone)]
struct Instance {
    // structure temporary, of course
    actors: HashMap<ObjectId, NetworkedActor>,
    navmesh: Navmesh,
}

impl Instance {
    pub fn new(id: u16, game_data: &mut GameData) -> Self {
        let mut instance = Self::default();

        let sheet = TerritoryTypeSheet::read_from(&mut game_data.resource, Language::None).unwrap();
        let Some(row) = sheet.get_row(id as u32) else {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return instance;
        };

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let bg_path = row.Bg().into_string().unwrap();

        let path = format!("bg/{}.lvb", &bg_path);
        tracing::info!("Loading {}", path);
        let lgb_file = game_data.resource.read(&path).unwrap();
        let lgb = Lvb::from_existing(&lgb_file).unwrap();

        let mut navimesh_path = None;
        for layer_set in &lgb.scns[0].unk3.unk2 {
            // FIXME: this is wrong. I think there might be multiple, separate navimeshes in really big zones but I'm not sure yet.
            navimesh_path = Some(layer_set.path_nvm.replace("/server/data/", "").to_string());
        }

        if navimesh_path.is_none() {
            tracing::info!("No navimesh path found, monsters will not function correctly!");
            return instance;
        }

        let config = get_config();
        if config.filesystem.navimesh_path.is_empty() {
            tracing::warn!("Navimesh path is not set! Monsters will not function correctly!");
        } else {
            let mut nvm_path = PathBuf::from(config.filesystem.navimesh_path);
            nvm_path.push(navimesh_path.unwrap());

            if let Ok(nvm_bytes) = std::fs::read(&nvm_path) {
                instance.navmesh = Navmesh::from_existing(&nvm_bytes).unwrap();

                tracing::info!("Successfully loaded navimesh from {nvm_path:?}");
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
struct ClientState {
    zone_id: u16,
}

#[derive(Default, Debug)]
struct WorldServer {
    to_remove: Vec<ClientId>,
    clients: HashMap<ClientId, (ClientHandle, ClientState)>,
    /// Indexed by zone id
    instances: HashMap<u16, Instance>,
}

impl WorldServer {
    /// Finds the instance associated with a zone, or None if it doesn't exist yet.
    fn find_instance(&self, zone_id: u16) -> Option<&Instance> {
        self.instances.get(&zone_id)
    }

    /// Finds the instance associated with a zone, or creates it if it doesn't exist yet
    fn find_instance_mut(&mut self, zone_id: u16) -> &mut Instance {
        self.instances.entry(zone_id).or_default()
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance_mut(&mut self, actor_id: u32) -> Option<&mut Instance> {
        self.instances
            .values_mut()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }

    /// Tell all the clients that a new NPC spawned.
    fn send_npc(&mut self, actor: Actor, spawn: NpcSpawn) {
        // TODO: only send in the relevant instance
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            let msg = FromServer::ActorSpawn(actor, spawn.clone());

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }
}

fn server_logic_tick(data: &mut WorldServer) {
    for instance in data.instances.values_mut() {
        let mut actor_moves = Vec::new();
        let players = instance.find_all_players();

        // const pass
        let instance_copy = instance.clone(); // TODO: refactor out please
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
                        ));
                    }
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

                let target_actor = instance_copy.find_actor(current_target.unwrap());
                let target_pos = target_actor.unwrap().get_common_spawn().pos;
                let distance = Position::distance(spawn.common.pos, target_pos);
                let needs_repath = current_path.is_empty() && distance > 5.0; // TODO: confirm distance this in retail
                if needs_repath && current_target.is_some() {
                    let current_pos = spawn.common.pos;
                    let target_actor = instance_copy.find_actor(current_target.unwrap());
                    let target_pos = target_actor.unwrap().get_common_spawn().pos;
                    *current_path = instance
                        .navmesh
                        .calculate_path(
                            [current_pos.x, current_pos.y, current_pos.z],
                            [target_pos.x, target_pos.y, target_pos.z],
                        )
                        .into();
                }

                // update common spawn
                for msg in &actor_moves {
                    if let FromServer::ActorMove(msg_id, pos, rotation) = msg {
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
            for (handle, _) in data.clients.values_mut() {
                if handle.send(msg.clone()).is_err() {
                    //to_remove.push(id);
                }
            }
        }
    }
}

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let data = Arc::new(Mutex::new(WorldServer::default()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

    {
        let data = data.clone();
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            interval.tick().await;
            loop {
                interval.tick().await;
                let mut data = data.lock().unwrap();
                server_logic_tick(&mut data);
            }
        });
    }

    while let Some(msg) = recv.recv().await {
        let mut to_remove = Vec::new();

        match msg {
            ToServer::NewClient(handle) => {
                let mut data = data.lock().unwrap();

                data.clients
                    .insert(handle.id, (handle, ClientState::default()));
            }
            ToServer::ZoneLoaded(from_id, zone_id, common_spawn) => {
                let mut data = data.lock().unwrap();

                // create a new instance if necessary
                data.instances.entry(zone_id).or_insert_with(|| {
                    let mut game_data = game_data.lock().unwrap();
                    Instance::new(zone_id, &mut game_data)
                });

                // Send existing player data, if any
                if let Some(instance) = data.find_instance(zone_id).cloned() {
                    for (id, (handle, state)) in &mut data.clients {
                        let id = *id;

                        if id == from_id {
                            state.zone_id = zone_id;

                            // send existing player data
                            for (id, spawn) in &instance.actors {
                                let npc_spawn = match spawn {
                                    NetworkedActor::Player(spawn) => spawn,
                                    NetworkedActor::Npc { spawn, .. } => spawn,
                                };

                                // Note that we currently only support spawning via the NPC packet, hence why we don't need to differentiate here
                                let msg = FromServer::ActorSpawn(
                                    Actor {
                                        id: *id,
                                        hp: 100,
                                        spawn_index: 0,
                                    },
                                    npc_spawn.clone(),
                                );

                                handle.send(msg).unwrap();
                            }

                            break;
                        }
                    }
                }

                let (client, _) = data.clients.get(&from_id).unwrap().clone();

                // add the connection's actor to the table
                {
                    let instance = data.find_instance_mut(zone_id);
                    instance.actors.insert(
                        ObjectId(client.actor_id),
                        NetworkedActor::Player(NpcSpawn {
                            common: common_spawn.clone(),
                            ..Default::default()
                        }),
                    );
                }

                // Then tell any clients in the zone that we spawned
                for (id, (handle, state)) in &mut data.clients {
                    let id = *id;

                    // don't bother telling the client who told us
                    if id == from_id {
                        continue;
                    }

                    // skip any clients not in our zone
                    if state.zone_id != zone_id {
                        continue;
                    }

                    let msg = FromServer::ActorSpawn(
                        Actor {
                            id: ObjectId(client.actor_id),
                            hp: 0,
                            spawn_index: 0,
                        },
                        NpcSpawn {
                            common: common_spawn.clone(),
                            ..Default::default()
                        },
                    );

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::LeftZone(from_id, actor_id, zone_id) => {
                let mut data = data.lock().unwrap();

                // when the actor leaves the zone, remove them from the instance
                let current_instance = data.find_actor_instance_mut(actor_id).unwrap();
                current_instance.actors.remove(&ObjectId(actor_id));

                // Then tell any clients in the zone that we left
                for (id, (handle, state)) in &mut data.clients {
                    let id = *id;

                    // don't bother telling the client who told us
                    if id == from_id {
                        continue;
                    }

                    // skip any clients not in our zone
                    if state.zone_id != zone_id {
                        continue;
                    }

                    let msg = FromServer::ActorDespawn(actor_id);

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::Message(from_id, msg) => {
                let mut data = data.lock().unwrap();

                for (id, (handle, _)) in &mut data.clients {
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
            ToServer::ActorMoved(from_id, actor_id, position, rotation) => {
                let mut data = data.lock().unwrap();

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

                    for (id, (handle, _)) in &mut data.clients {
                        let id = *id;

                        if id == from_id {
                            continue;
                        }

                        let msg = FromServer::ActorMove(actor_id, position, rotation);

                        if handle.send(msg).is_err() {
                            to_remove.push(id);
                        }
                    }
                }
            }
            ToServer::ClientTrigger(from_id, from_actor_id, trigger) => {
                let mut data = data.lock().unwrap();

                for (id, (handle, _)) in &mut data.clients {
                    let id = *id;

                    tracing::info!("{:#?}", trigger);

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
                        continue;
                    }

                    match &trigger.trigger {
                        ClientTriggerCommand::SetTarget { actor_id } => {
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::SetTarget {
                                        actor_id: *actor_id,
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
                        ClientTriggerCommand::Emote { emote } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Emote { emote: *emote },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ToggleWeapon { shown } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::ToggleWeapon { shown: *shown },
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
                                object_type: 0,
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

                data.send_npc(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    spawn,
                );
            }
            ToServer::DebugNewEnemy(_from_id, from_actor_id) => {
                let mut data = data.lock().unwrap();

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
                            hp_curr: 91,
                            hp_max: 91,
                            mp_curr: 100,
                            mp_max: 100,
                            spawn_index: 0,   // not needed at this level
                            bnpc_base: 13498, // TODO: changing this prevents it from spawning...
                            bnpc_name: 405,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            level: 1,
                            battalion: 4,
                            model_chara: 297,
                            pos: player.common.pos,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                data.send_npc(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    spawn,
                );
            }
            ToServer::DebugSpawnClone(_from_id, from_actor_id) => {
                let mut data = data.lock().unwrap();

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

                data.send_npc(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    spawn,
                );
            }
            ToServer::ActionRequest(from_id, _from_actor_id, request) => {
                let mut game_data = game_data.lock().unwrap();
                let cast_time = game_data.get_casttime(request.action_key).unwrap();

                let send_execution = |from_id: ClientId, data: Arc<Mutex<WorldServer>>| {
                    let mut data = data.lock().unwrap();

                    tracing::info!("Now finishing delayed cast!");

                    for (id, (handle, _)) in &mut data.clients {
                        let id = *id;

                        if id == from_id {
                            let msg = FromServer::ActionComplete(request);

                            if handle.send(msg).is_err() {
                                data.to_remove.push(id);
                            }
                            break;
                        }
                    }
                };

                if cast_time == 0 {
                    // If instantaneous, send right back
                    send_execution(from_id, data.clone());
                } else {
                    // Otherwise, delay
                    // NOTE: I know this won't scale, but it's a fine hack for now

                    tracing::info!(
                        "Delaying spell cast for {} milliseconds",
                        cast_time as u64 * 100
                    );

                    // we have to shadow these variables to tell rust not to move them into the async closure
                    let data = data.clone();
                    tokio::task::spawn(async move {
                        let mut interval =
                            tokio::time::interval(Duration::from_millis(cast_time as u64 * 100));
                        interval.tick().await;
                        interval.tick().await;
                        send_execution(from_id, data);
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

                    player.common.display_flags = config.display_flag;
                }

                let mut data = data.lock().unwrap();
                for (id, (handle, _)) in &mut data.clients {
                    let id = *id;

                    let msg = FromServer::UpdateConfig(from_actor_id, config.clone());

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::Equip(_from_id, from_actor_id, main_weapon_id, model_ids) => {
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
                    player.common.models = model_ids;
                }

                // Inform all clients about their new equipped model ids
                let mut data = data.lock().unwrap();
                for (id, (handle, _)) in &mut data.clients {
                    let id = *id;

                    let msg = FromServer::ActorEquip(from_actor_id, main_weapon_id, model_ids);

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
                                      data: Arc<Mutex<WorldServer>>,
                                      entry: &PathBuf| {
                    let mut data = data.lock().unwrap();

                    for (id, (handle, _)) in &mut data.clients {
                        let id = *id;

                        if id == from_id {
                            // only care about Ipc packets
                            let filename = entry.file_name().unwrap().to_str().unwrap();
                            if !filename.contains("Ipc") {
                                continue;
                            }

                            // only care about packets from the server
                            if !filename.contains("(to client)") {
                                continue;
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

                            let ipc_header_bytes =
                                std::fs::read(format!("{path}/ipc_header.bin")).unwrap();
                            let opcode =
                                u16::from_le_bytes(ipc_header_bytes[2..4].try_into().unwrap());

                            let mut ipc_data = std::fs::read(format!("{path}/data.bin")).unwrap();
                            let ipc_len = ipc_data.len() as u32 + 32;
                            let mut cursor = Cursor::new(&mut ipc_data);
                            if let Ok(parsed) =
                                ServerZoneIpcSegment::read_le_args(&mut cursor, (&ipc_len,))
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
                                data: SegmentData::Ipc {
                                    data: ServerZoneIpcSegment {
                                        op_code: ServerZoneIpcType::Unknown(opcode),
                                        data: ServerZoneIpcData::Unknown { unk: ipc_data },
                                        ..Default::default()
                                    },
                                },
                            });

                            if handle.send(msg).is_err() {
                                data.to_remove.push(id);
                            }
                            break;
                        }
                    }
                };

                let data = data.clone();
                tokio::task::spawn(async move {
                    for entry in &entries {
                        let mut interval = tokio::time::interval(Duration::from_millis(100));
                        interval.tick().await;
                        interval.tick().await;
                        send_execution(from_id, data.clone(), entry);
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
                     data: Arc<Mutex<WorldServer>>,
                     effect_id: u16,
                     effect_param: u16,
                     effect_source_actor_id: ObjectId| {
                        let mut data = data.lock().unwrap();

                        tracing::info!("Now losing effect {}!", effect_id);

                        for (id, (handle, _)) in &mut data.clients {
                            let id = *id;

                            if id == from_id {
                                let msg = FromServer::LoseEffect(
                                    effect_id,
                                    effect_param,
                                    effect_source_actor_id,
                                );

                                if handle.send(msg).is_err() {
                                    data.to_remove.push(id);
                                }
                                break;
                            }
                        }
                    };

                // Eventually tell the player they lost this effect
                // NOTE: I know this won't scale, but it's a fine hack for now

                tracing::info!("Effect {effect_id} lasts for {effect_duration} seconds");

                // we have to shadow these variables to tell rust not to move them into the async closure
                let data = data.clone();
                tokio::task::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(
                        (effect_duration * 1000.0) as u64,
                    ));
                    interval.tick().await;
                    interval.tick().await;
                    send_lost_effect(
                        from_id,
                        data,
                        effect_id,
                        effect_param,
                        effect_source_actor_id,
                    );
                });
            }
            ToServer::Disconnected(from_id) => {
                let mut data = data.lock().unwrap();

                data.to_remove.push(from_id);
            }
            ToServer::FatalError(err) => return Err(err),
        }

        // Remove any clients that errored out
        {
            let mut data = data.lock().unwrap();
            data.to_remove.append(&mut to_remove);

            for remove_id in data.to_remove.clone() {
                // remove any actors they had
                let mut actor_id = None;
                for (id, (handle, _)) in &mut data.clients {
                    if *id == remove_id {
                        actor_id = Some(handle.actor_id);
                    }
                }

                if let Some(actor_id) = actor_id {
                    // remove them from the instance
                    let current_instance = data.find_actor_instance_mut(actor_id).unwrap();
                    current_instance.actors.remove(&ObjectId(actor_id));
                }

                data.clients.remove(&remove_id);
            }
        }
    }
    Ok(())
}
