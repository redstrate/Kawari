use std::collections::HashMap;

use crate::{
    ClientHandle, ClientId, FromServer,
    common::SpawnKind,
    server::{
        ClientState, WorldServer,
        actor::NetworkedActor,
        instance::Instance,
        social::{Party, get_party_id_from_actor_id},
    },
};
use kawari::{
    common::ObjectId,
    ipc::zone::{ActorControlCategory, ServerZoneIpcSegment},
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
    /// Creates a `FromServer` message that will spawn `actor`.
    pub fn spawn_existing_actor_message(
        client_state: &mut ClientState,
        object_id: ObjectId,
        actor: &NetworkedActor,
    ) -> Option<FromServer> {
        let spawn_index = (match actor {
            NetworkedActor::Player { .. } => client_state.actor_allocator.reserve(object_id),
            NetworkedActor::Npc { .. } => client_state.actor_allocator.reserve(object_id),
            NetworkedActor::Object { .. } => client_state.object_allocator.reserve(object_id),
        })?;

        let msg = match actor {
            NetworkedActor::Player { spawn, .. } => {
                let mut spawn = spawn.clone();
                spawn.common.spawn_index = spawn_index;
                FromServer::ActorSpawn(object_id, SpawnKind::Player(spawn))
            }
            NetworkedActor::Npc { spawn, .. } => {
                let mut spawn = spawn.clone();
                spawn.common.spawn_index = spawn_index;
                FromServer::ActorSpawn(object_id, SpawnKind::Npc(spawn))
            }
            NetworkedActor::Object { object } => {
                let mut object = *object;
                object.spawn_index = spawn_index;
                FromServer::ObjectSpawn(object)
            }
        };

        Some(msg)
    }

    /// Inform clients that have spawned this actor, that it should be deleted.
    pub fn remove_actor(&mut self, instance: &mut Instance, actor_id: ObjectId) {
        instance.actors.remove(&actor_id);

        for (id, (handle, state)) in &mut self.clients {
            let id = *id;

            // Don't tell itself
            if handle.actor_id == actor_id {
                continue;
            }

            // Skip any clients not in this instance
            if !instance.actors.contains_key(&handle.actor_id) {
                continue;
            }

            if let Some(spawn_index) = state.object_allocator.free(actor_id) {
                let msg = FromServer::DeleteObject(spawn_index);

                if handle.send(msg).is_err() {
                    self.to_remove.push(id);
                }
            } else if let Some(spawn_index) = state.actor_allocator.free(actor_id) {
                let msg = FromServer::DeleteActor(actor_id, spawn_index);

                if handle.send(msg).is_err() {
                    self.to_remove.push(id);
                }
            }

            // If the actor wasn't spawned for this client, fail silently.
        }
    }

    /// Sends a `message` to every client in this instance but *not* including it.
    pub fn send_to_instance(
        &mut self,
        instance: &Instance,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        self.send_in_range_implementation(
            ObjectId::default(),
            instance,
            message,
            destination,
            false,
            false,
        );
    }

    pub fn send_in_range(
        &mut self,
        actor_id: ObjectId,
        data: &WorldServer,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        let Some(instance) = data.find_actor_instance(actor_id) else {
            return;
        };

        self.send_in_range_instance(actor_id, instance, message, destination);
    }

    /// Sends the `message` to every client in range of `actor_id` but *not* including it.
    pub fn send_in_range_instance(
        &mut self,
        actor_id: ObjectId,
        instance: &Instance,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        self.send_in_range_implementation(actor_id, instance, message, destination, false, true);
    }

    /// Sends the `message` to every client in range of `actor_id` *and* including it.
    pub fn send_in_range_inclusive_instance(
        &mut self,
        actor_id: ObjectId,
        instance: &Instance,
        message: FromServer,
        destination: DestinationNetwork,
    ) {
        self.send_in_range_implementation(actor_id, instance, message, destination, true, true);
    }

    fn send_in_range_implementation(
        &mut self,
        actor_id: ObjectId,
        instance: &Instance,
        message: FromServer,
        destination: DestinationNetwork,
        inclusive: bool,
        only_spawned: bool,
    ) {
        let clients = match destination {
            DestinationNetwork::ZoneClients => &mut self.clients,
            DestinationNetwork::ChatClients => &mut self.chat_clients,
        };

        for (id, (handle, state)) in clients {
            let id = *id;

            if !inclusive {
                // Don't include the actor itself
                if actor_id == handle.actor_id {
                    continue;
                }
            }

            // Skip any clients not in our instance
            if !instance.actors.contains_key(&handle.actor_id) {
                continue;
            }

            if only_spawned {
                // Skip anything that hasn't spawned us
                if !state.has_spawned(actor_id) && actor_id != handle.actor_id {
                    continue;
                }
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

    /// Sends the `message` to `client_id`.
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

    /// Sends the `message` to `actor_id`.
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

    /// Sends the `message` to every member of `party_id`.
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
            if !member.actor_id.is_valid() || member.zone_client_id == ClientId::default() {
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

    /// Send a server message to a specific actor, or their entire party (including the specific actor).
    pub fn send_to_party_or_self(&mut self, from_actor_id: ObjectId, msg: FromServer) {
        if let Some(party_id) = get_party_id_from_actor_id(self, from_actor_id) {
            self.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);
        } else {
            self.send_to_by_actor_id(from_actor_id, msg, DestinationNetwork::ZoneClients);
        }
    }

    /// Sends the `ipc` to `client_id`.
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

    /// Sends the ActorControl `category` to all in-range actors, *excluding* `from_actor_id`.
    pub fn send_ac_in_range(
        &mut self,
        data: &WorldServer,
        from_actor_id: ObjectId,
        category: ActorControlCategory,
    ) {
        let Some(instance) = data.find_actor_instance(from_actor_id) else {
            return;
        };

        self.send_ac_in_range_instance(instance, from_actor_id, category);
    }

    /// Sends the ActorControl `category` to all in-range actors, *excluding* `from_actor_id`.
    pub fn send_ac_in_range_instance(
        &mut self,
        data: &Instance,
        from_actor_id: ObjectId,
        category: ActorControlCategory,
    ) {
        let msg = FromServer::ActorControl(from_actor_id, category);

        self.send_in_range_instance(from_actor_id, data, msg, DestinationNetwork::ZoneClients);
    }

    /// Sends the ActorControl `category` to all in-range actors, *including* `from_actor_id` (but as an ActorControlSelf.)
    pub fn send_ac_in_range_inclusive(
        &mut self,
        data: &WorldServer,
        from_actor_id: ObjectId,
        category: ActorControlCategory,
    ) {
        // First send to the actor itself:
        {
            let msg = FromServer::ActorControlSelf(category.clone());

            self.send_to_by_actor_id(from_actor_id, msg, DestinationNetwork::ZoneClients);
        }

        // Then to the other acotrs in range:
        self.send_ac_in_range(data, from_actor_id, category);
    }

    /// Returns the `ClientState` for `client_id`.
    pub fn get_state_mut(&mut self, client_id: ClientId) -> Option<&mut ClientState> {
        self.clients.get_mut(&client_id).map(|x| &mut x.1)
    }

    /// Returns the `ClientHandle` and `ClientState` for `actor_id`.
    pub fn get_by_actor_mut(
        &mut self,
        actor_id: ObjectId,
    ) -> Option<&mut (ClientHandle, ClientState)> {
        self.clients
            .iter_mut()
            .filter(|x| x.1.0.actor_id == actor_id)
            .last()
            .map(|x| x.1)
    }
}
