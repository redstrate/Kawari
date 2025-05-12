use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

use crate::{
    common::{ObjectId, Position},
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, BattleNpcSubKind,
        ClientTriggerCommand, CommonSpawn, NpcSpawn, ObjectKind,
    },
};

use super::{Actor, ClientHandle, ClientId, FromServer, ToServer};

#[derive(Default, Debug, Clone)]
struct Instance {
    // structure temporary, of course
    actors: HashMap<ObjectId, CommonSpawn>,
}

#[derive(Default, Debug, Clone)]
struct ClientState {
    zone_id: u16,
}

#[derive(Default, Debug)]
struct WorldServer {
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
        if self.instances.contains_key(&zone_id) {
            self.instances.get_mut(&zone_id).unwrap()
        } else {
            self.instances.insert(zone_id, Instance::default());
            self.instances.get_mut(&zone_id).unwrap()
        }
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance_mut(&mut self, actor_id: u32) -> Option<&mut Instance> {
        for (_, instance) in &mut self.instances {
            if instance.actors.contains_key(&ObjectId(actor_id)) {
                return Some(instance);
            }
        }
        None
    }
}

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let mut data = WorldServer::default();
    let mut to_remove = Vec::new();

    while let Some(msg) = recv.recv().await {
        match msg {
            ToServer::NewClient(handle) => {
                data.clients
                    .insert(handle.id, (handle, ClientState::default()));
            }
            ToServer::ZoneLoaded(from_id, zone_id) => {
                // create a new instance if nessecary
                if !data.instances.contains_key(&zone_id) {
                    data.instances.insert(zone_id, Instance::default());
                }

                // Send existing player data, if any
                if let Some(instance) = data.find_instance(zone_id).cloned() {
                    for (id, (handle, state)) in &mut data.clients {
                        let id = *id;

                        if id == from_id {
                            state.zone_id = zone_id;

                            // send existing player data
                            for (id, common) in &instance.actors {
                                let msg = FromServer::ActorSpawn(
                                    Actor {
                                        id: *id,
                                        hp: 100,
                                        spawn_index: 0,
                                    },
                                    common.clone(),
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
                    instance
                        .actors
                        .insert(ObjectId(client.actor_id), client.common.clone());
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
                        client.common.clone(),
                    );

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::LeftZone(from_id, actor_id, zone_id) => {
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
                if let Some(instance) = data.find_actor_instance_mut(actor_id) {
                    if let Some((_, common)) = instance
                        .actors
                        .iter_mut()
                        .find(|actor| *actor.0 == ObjectId(actor_id))
                    {
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
                for (id, (handle, _)) in &mut data.clients {
                    let id = *id;

                    tracing::info!("{:#?}", trigger);

                    // handle player-to-server actions
                    if id == from_id {
                        match &trigger.trigger {
                            ClientTriggerCommand::TeleportQuery { aetheryte_id } => {
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
                            _ => {}
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
                        _ => tracing::warn!("Server doesn't know what to do with {:#?}", trigger),
                    }
                }
            }
            ToServer::DebugNewNpc(_from_id) => {
                for (id, (handle, _)) in &mut data.clients {
                    let id = *id;

                    let msg = FromServer::SpawnNPC(NpcSpawn {
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
                            pos: Position::default(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::Disconnected(from_id) => {
                to_remove.push(from_id);
            }
            ToServer::FatalError(err) => return Err(err),
        }

        // Remove any clients that errored out
        for remove_id in &to_remove {
            // remove any actors they had
            let mut actor_id = None;
            for (id, (handle, _)) in &mut data.clients {
                if *id == *remove_id {
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
    Ok(())
}
