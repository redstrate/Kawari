use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

use crate::{
    common::ObjectId,
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlTarget, ClientTriggerCommand, CommonSpawn,
    },
};

use super::{Actor, ClientHandle, ClientId, FromServer, ToServer};

#[derive(Default, Debug)]
struct WorldServer {
    clients: HashMap<ClientId, ClientHandle>,
    // structure temporary, of course
    actors: HashMap<ObjectId, CommonSpawn>,
}

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let mut data = WorldServer::default();
    let mut to_remove = Vec::new();

    while let Some(msg) = recv.recv().await {
        match msg {
            ToServer::NewClient(handle) => {
                data.clients.insert(handle.id, handle);
            }
            ToServer::ZoneLoaded(from_id) => {
                for (id, handle) in &mut data.clients {
                    let id = *id;

                    if id == from_id {
                        // send existing player data
                        for (id, common) in &data.actors {
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
            ToServer::Message(from_id, msg) => {
                for (id, handle) in &mut data.clients {
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
            ToServer::ActorSpawned(from_id, actor, common) => {
                data.actors.insert(actor.id, common.clone());

                for (id, handle) in &mut data.clients {
                    let id = *id;

                    if id == from_id {
                        continue;
                    }

                    let msg = FromServer::ActorSpawn(actor, common.clone());

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::ActorDespawned(_from_id, actor_id) => {
                data.actors.remove(&ObjectId(actor_id));

                for (id, handle) in &mut data.clients {
                    let id = *id;

                    let msg = FromServer::ActorDespawn(actor_id);

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::ActorMoved(from_id, actor_id, position, rotation) => {
                if let Some((_, common)) = data
                    .actors
                    .iter_mut()
                    .find(|actor| *actor.0 == ObjectId(actor_id))
                {
                    common.pos = position;
                    common.rotation = rotation;
                }

                for (id, handle) in &mut data.clients {
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
            ToServer::ClientTrigger(from_id, from_actor_id, trigger) => {
                for (id, handle) in &mut data.clients {
                    let id = *id;

                    // there's no reason to tell the actor what it just did
                    if id == from_id {
                        continue;
                    }

                    tracing::info!("{:#?}", trigger);

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
            ToServer::Disconnected(from_id) => {
                to_remove.push(from_id);
            }
            ToServer::FatalError(err) => return Err(err),
        }
    }
    // Remove any clients that errored out
    for id in to_remove {
        data.clients.remove(&id);
    }
    Ok(())
}
