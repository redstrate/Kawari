use std::collections::HashMap;

use crate::{
    common::{INVALID_OBJECT_ID, ObjectId},
    ipc::zone::ServerZoneIpcSegment,
    world::{
        ClientHandle, ClientId, FromServer,
        common::SpawnKind,
        server::{ClientState, instance::Instance, social::Party},
    },
};

#[derive(Default, Debug)]
pub struct NetworkState {
    pub to_remove: Vec<ClientId>,
    pub to_remove_chat: Vec<ClientId>,
    pub clients: HashMap<ClientId, (ClientHandle, ClientState)>,
    pub chat_clients: HashMap<ClientId, (ClientHandle, ClientState)>,
    pub parties: HashMap<u64, Party>,
}

#[derive(Debug, PartialEq)]
pub enum DestinationNetwork {
    ZoneClients,
    ChatClients,
}

impl NetworkState {
    /// Tell all the clients that a new actor spawned.
    pub fn send_actor(&mut self, actor_id: ObjectId, spawn: SpawnKind) {
        // TODO: only send in the relevant instance
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            let msg = FromServer::ActorSpawn(actor_id, spawn.clone());

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }

    /// Inform all clients in an instance that the actor has left.
    pub fn inform_remove_actor(
        &mut self,
        instance: &Instance,
        from_id: ClientId,
        actor_id: ObjectId,
    ) {
        for (id, (handle, _)) in &mut self.clients {
            let id = *id;

            // Don't bother telling the client who told us
            if id == from_id {
                continue;
            }

            // Skip any clients not in this instance
            if !instance.actors.contains_key(&handle.actor_id) {
                continue;
            }

            let msg = FromServer::ActorDespawn(actor_id);

            if handle.send(msg).is_err() {
                self.to_remove.push(id);
            }
        }
    }

    pub fn send_to_all(
        &mut self,
        id_to_skip: Option<ClientId>,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, _)) in clients {
            let id = *id;
            if let Some(id_to_skip) = id_to_skip
                && id == id_to_skip
            {
                continue;
            }

            if handle.send(message.clone()).is_err() {
                if destination == DestinationNetwork::ZoneClients {
                    self.to_remove.push(id);
                } else {
                    self.to_remove_chat.push(id);
                }
            }
        }
    }

    pub fn send_to(
        &mut self,
        client_id: ClientId,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, _)) in clients {
            let id = *id;

            if id == client_id {
                if handle.send(message).is_err() {
                    if destination == DestinationNetwork::ZoneClients {
                        self.to_remove.push(id);
                    } else {
                        self.to_remove_chat.push(id);
                    }
                }
                break;
            }
        }
    }

    pub fn send_to_by_actor_id(
        &mut self,
        actor_id: ObjectId,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, _)) in clients {
            let id = *id;

            if handle.actor_id == actor_id {
                if handle.send(message).is_err() {
                    if destination == DestinationNetwork::ZoneClients {
                        self.to_remove.push(id);
                    } else {
                        self.to_remove_chat.push(id);
                    }
                }
                break;
            }
        }
    }

    pub fn send_to_party(
        &mut self,
        party_id: u64,
        from_actor_id: Option<ObjectId>,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let Some(party) = self.parties.get(&party_id) else {
            return;
        };

        for member in &party.members {
            // Skip offline or otherwise non-existent members
            if member.actor_id == INVALID_OBJECT_ID || member.zone_client_id == ClientId::default()
            {
                continue;
            }

            // Skip a desired party member if needed.
            if let Some(from_actor_id) = from_actor_id
                && from_actor_id == member.actor_id
            {
                continue;
            }

            match destination {
                DestinationNetwork::ZoneClients => {
                    let handle = &mut self.clients.get_mut(&member.zone_client_id).unwrap().0;
                    if handle.send(message.clone()).is_err() {
                        self.to_remove.push(member.zone_client_id);
                    }
                }
                DestinationNetwork::ChatClients => {
                    let handle = &mut self.chat_clients.get_mut(&member.chat_client_id).unwrap().0;
                    if handle.send(message.clone()).is_err() {
                        self.to_remove_chat.push(member.chat_client_id);
                    }
                }
            }
        }
    }

    pub fn send_ipc_to(
        &mut self,
        client_id: ClientId,
        ipc: ServerZoneIpcSegment,
        from_actor_id: ObjectId,
    ) {
        let clients = &mut self.clients;
        let message = FromServer::PacketSegment(ipc, from_actor_id);

        for (id, (handle, _)) in clients {
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
