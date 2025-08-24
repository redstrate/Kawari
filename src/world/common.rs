use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::mpsc::Sender;

use crate::{
    common::{ObjectId, Position},
    ipc::zone::{
        ActionRequest, ActorControl, ActorControlSelf, ActorControlTarget, ClientTrigger,
        CommonSpawn, Conditions, Config, NpcSpawn, ServerZoneIpcSegment,
    },
    packet::PacketSegment,
};

use super::Actor;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

#[derive(Clone)]
pub enum FromServer {
    /// A chat message.
    Message(String),
    /// An actor has been spawned.
    ActorSpawn(Actor, NpcSpawn),
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
    /// Action has completed and needs to be executed
    ActionComplete(ActionRequest),
    /// Action has been cancelled
    ActionCancelled(),
    /// Update an actor's equip display flags.
    UpdateConfig(u32, Config),
    /// Update an actor's model IDs.
    ActorEquip(u32, u64, [u32; 10]),
    /// Informs the connection to replay packet data to the client.
    ReplayPacket(PacketSegment<ServerZoneIpcSegment>),
    /// The player should lose this effect.
    LoseEffect(u16, u16, ObjectId),
    // TODO: temporary
    Conditions(Conditions),
    /// To inform the connection of the zone they're loading into.
    ChangeZone(u16, u16, Position, f32),
}

#[derive(Debug, Clone)]
pub struct ClientHandle {
    pub id: ClientId,
    pub ip: SocketAddr,
    pub channel: Sender<FromServer>,
    pub actor_id: u32,
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
    ZoneLoaded(ClientId, u16, CommonSpawn),
    /// The connection wants to enter a new zone.
    // TODO: temporary as this is only used for commands and those aren't run on global server state yet
    ChangeZone(ClientId, u32, u16),
    /// The player walks through a zone change line.
    EnterZoneJump(ClientId, u32, u32),
    /// The connection disconnected.
    Disconnected(ClientId),
    /// A fatal error occured.
    FatalError(std::io::Error),
    /// Spawn a friendly debug NPC.
    DebugNewNpc(ClientId, u32),
    /// Spawn an enemy debug NPC.
    DebugNewEnemy(ClientId, u32, u32),
    /// Spawn a debug clone.
    DebugSpawnClone(ClientId, u32),
    /// Request to perform an action
    ActionRequest(ClientId, u32, ActionRequest),
    /// We want to update our own equip display flags.
    Config(ClientId, u32, Config),
    /// Tell the server what models IDs we have equipped.
    Equip(ClientId, u32, u64, [u32; 10]),
    /// Begins a packet replay.
    BeginReplay(ClientId, String),
    /// The player gains an effect.
    GainEffect(ClientId, u32, u16, f32, u16, ObjectId),
    /// Warp with the specified id.
    Warp(ClientId, u32, u32),
    /// Warp with the specified aetheryte id.
    WarpAetheryte(ClientId, u32, u32),
    /// Ready to spawn the player (this happens during initrequest)
    ReadySpawnPlayer(ClientId, u16, Position, f32),
    /// Ready to send the ZoneIn ACS
    ZoneIn(ClientId, u32, bool),
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
