use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::mpsc::Sender;

use crate::{
    common::Position,
    ipc::zone::{
        ActorControl, ActorControlSelf, ActorControlTarget, ClientTrigger, CommonSpawn, NpcSpawn,
    },
};

use super::Actor;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

pub enum FromServer {
    /// A chat message.
    Message(String),
    /// An actor has been spawned.
    ActorSpawn(Actor, CommonSpawn),
    /// An actor moved to a new position.
    ActorMove(u32, Position, f32),
    // An actor has despawned.
    ActorDespawn(u32),
    /// We need to update an actor
    ActorControl(u32, ActorControl),
    /// We need to update an actor's target
    ActorControlTarget(u32, ActorControlTarget),
    /// We need to update the player actor
    ActorControlSelf(ActorControlSelf),
    /// Spawn an NPC
    SpawnNPC(NpcSpawn),
}

#[derive(Debug, Clone)]
pub struct ClientHandle {
    pub id: ClientId,
    pub ip: SocketAddr,
    pub channel: Sender<FromServer>,
    pub actor_id: u32,
    pub common: CommonSpawn,
}

impl ClientHandle {
    /// Send a message to this client actor. Will emit an error if sending does
    /// not succeed immediately, as this means that forwarding messages to the
    /// tcp connection cannot keep up.
    pub fn send(&mut self, msg: FromServer) -> Result<(), std::io::Error> {
        if self.channel.try_send(msg).is_err() {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Can't keep up or dead",
            ))
        } else {
            Ok(())
        }
    }

    /// Kill the actor.
    pub fn kill(self) {
        // run the destructor
        drop(self);
    }
}

pub enum ToServer {
    /// A new connection has started.
    NewClient(ClientHandle),
    /// The connection sent a message.
    Message(ClientId, String),
    /// The connection's player moved.
    ActorMoved(ClientId, u32, Position, f32),
    /// The connection has recieved a client trigger.
    ClientTrigger(ClientId, u32, ClientTrigger),
    /// The connection loaded into a zone.
    // TODO: the connection should not be in charge and telling the global server what zone they just loaded in! but this will work for now
    ZoneLoaded(ClientId, u16),
    /// The connection left a zone.
    LeftZone(ClientId, u32, u16),
    /// The connection disconnected.
    Disconnected(ClientId),
    /// A fatal error occured.
    FatalError(std::io::Error),
    DebugNewNpc(ClientId),
}

#[derive(Clone, Debug)]
pub struct ServerHandle {
    pub chan: Sender<ToServer>,
    pub next_id: Arc<AtomicUsize>,
}

impl ServerHandle {
    pub async fn send(&mut self, msg: ToServer) {
        if self.chan.send(msg).await.is_err() {
            panic!("Main loop has shut down.");
        }
    }
    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}
