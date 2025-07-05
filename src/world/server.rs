use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    common::{CustomizeData, GameData, ObjectId, ObjectTypeId},
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, BattleNpcSubKind,
        ClientTriggerCommand, CommonSpawn, NpcSpawn, ObjectKind,
    },
};

use super::{Actor, ClientHandle, ClientId, FromServer, ToServer};

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
    Npc(NpcSpawn),
}

#[derive(Default, Debug, Clone)]
struct Instance {
    // structure temporary, of course
    actors: HashMap<ObjectId, NetworkedActor>,
}

impl Instance {
    fn find_actor(&self, id: ObjectId) -> Option<&NetworkedActor> {
        self.actors.get(&id)
    }

    fn find_actor_mut(&mut self, id: ObjectId) -> Option<&mut NetworkedActor> {
        self.actors.get_mut(&id)
    }

    fn insert_npc(&mut self, id: ObjectId, spawn: NpcSpawn) {
        self.actors.insert(id, NetworkedActor::Npc(spawn));
    }

    fn generate_actor_id() -> u32 {
        // TODO: ensure we don't collide with another actor
        fastrand::u32(..)
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

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let data = Arc::new(Mutex::new(WorldServer::default()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

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
                data.instances
                    .entry(zone_id)
                    .or_insert_with(Instance::default);

                // Send existing player data, if any
                if let Some(instance) = data.find_instance(zone_id).cloned() {
                    for (id, (handle, state)) in &mut data.clients {
                        let id = *id;

                        if id == from_id {
                            state.zone_id = zone_id;

                            // send existing player data
                            for (id, spawn) in &instance.actors {
                                let npc_spawn = match spawn {
                                    NetworkedActor::Player(npc_spawn) => npc_spawn,
                                    NetworkedActor::Npc(npc_spawn) => npc_spawn,
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
                            NetworkedActor::Npc(npc_spawn) => &mut npc_spawn.common,
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

                        if let ClientTriggerCommand::EventRelatedUnk { .. } = &trigger.trigger
                        {
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
