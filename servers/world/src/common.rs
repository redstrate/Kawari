use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::mpsc::Sender;

use crate::{StatusEffects, lua::LuaTask};
use kawari::{
    common::{JumpState, MoveAnimationState, MoveAnimationType, ObjectId, Position},
    ipc::{
        chat::{
            ChatChannelType, PartyMessage, SendPartyMessage, SendTellMessage, TellNotFoundError,
        },
        zone::{
            ActionRequest, ActorControl, ActorControlSelf, ActorControlTarget, ClientTrigger,
            Conditions, Config, InviteReply, InviteType, NpcSpawn, ObjectSpawn, PartyMemberEntry,
            PartyUpdateStatus, PlayerEntry, PlayerSpawn, ServerZoneIpcSegment, SocialListRequest,
            SocialListRequestType,
        },
    },
};

use super::lua::LuaZone;

#[derive(Copy, Clone, Default, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

impl std::fmt::Debug for ClientId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ClientId ({})", self.0)
    }
}

/// A type encapsulating the different spawn types.
/// Note that event object (eobj) spawning is handled elsewhere in connection.rs.
#[derive(Clone, Debug)]
pub enum SpawnKind {
    /// A player's spawn data is contained within.
    Player(PlayerSpawn),
    /// An NPC's spawn data is contained within.
    Npc(NpcSpawn),
}

/// A type encapsulating party update sender and recipient info.
/// This is internal to Kawari, hence it being placed here.
#[derive(Clone, Debug, Default)]
pub struct PartyUpdateTargets {
    pub execute_account_id: u64,
    pub execute_content_id: u64,
    pub execute_name: String,
    pub target_account_id: u64,
    pub target_content_id: u64,
    pub target_name: String,
}

/// A type encapsulating various information about a zone chat mesage to be sent.
#[derive(Clone, Debug, Default)]
pub struct MessageInfo {
    /// The sender's actor id.
    pub sender_actor_id: ObjectId,
    /// The sender's account id. Likely used by the client to know to ignore the message if this player is blocked.
    pub sender_account_id: u64,
    /// The sender's home world id. Used for purposes of displaying their home world in the chat window.
    pub sender_world_id: u16,
    /// The sender's name.
    pub sender_name: String,
    /// The sender's position in the zone, used for creating a radius around which the message is heard (not yet implemented on Kawari).
    pub sender_position: Position,
    /// The channel the message is intended for (say, shout, yell, custom emote (/em)).
    pub channel: ChatChannelType,
    /// The chat message itself.
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum FromServer {
    /// A chat message.
    Message(MessageInfo),
    /// An actor has been spawned.
    ActorSpawn(ObjectId, SpawnKind),
    /// An actor moved to a new position.
    ActorMove(
        ObjectId,
        Position,
        f32,
        MoveAnimationType,
        MoveAnimationState,
        JumpState,
    ),
    // An actor should be despawned.
    DeleteActor(ObjectId, u8),
    // An object should be despawned.
    DeleteObject(u8),
    /// We need to update an actor
    ActorControl(ObjectId, ActorControl),
    /// We need to update an actor's target
    ActorControlTarget(ObjectId, ActorControlTarget),
    /// We need to update the player actor
    ActorControlSelf(ActorControlSelf),
    /// Update an actor's equip display flags.
    UpdateConfig(ObjectId, Config),
    /// Update an actor's model IDs.
    ActorEquip(ObjectId, u64, u64, [u32; 10]),
    /// We need to summon a player's minion, and tell other clients
    ActorSummonsMinion(u32),
    /// We need to despawn a player's minion, and tell other clients
    ActorDespawnsMinion(),
    /// The player should lose this effect.
    LoseEffect(u16, u16, ObjectId),
    // TODO: temporary
    Conditions(Conditions),
    /// To inform the connection of the zone they're loading into.
    ChangeZone(u16, u16, u16, Position, f32, LuaZone, bool),
    /// The returned position and rotation from ToServer::MoveToPopRange.
    NewPosition(Position, f32, bool),
    /// We need to inform the recipent about the direct message they're receiving.
    TellMessageSent(MessageInfo),
    /// We need to inform the sender that the recipient was not found or is offline.
    TellRecipientNotFound(TellNotFoundError),
    /// We need to tell our chat connection that our zone connection has disconnected.
    ChatDisconnected(),
    /// Inform the chat connection that its zone connection has joined a party.
    SetPartyChatChannel(u32),
    /// Inform the client that they've received a party invite.
    PartyInvite(u64, u64, String),
    /// Inform the client about the results of an invite sent to another player.
    InvitationResult(u64, u64, String, InviteType, InviteReply),
    /// The client who received the invite also needs to be informed.
    InvitationReplyResult(u64, String, InviteType, InviteReply),
    /// A chat message from the client's party has been received.
    PartyMessageSent(PartyMessage),
    /// The client who requested a social list update needs to be informed.
    SocialListResponse(SocialListRequestType, u8, Vec<PlayerEntry>),
    /// Members of this party need to be informed of an update.
    PartyUpdate(
        PartyUpdateTargets,
        PartyUpdateStatus,
        Option<(u64, u32, ObjectId, Vec<PartyMemberEntry>)>,
    ),
    /// The character the client invited is already in a party.
    // TODO: This is actually incorrect behaviour, we need to research more how the client "knows" another player is already in a party since the server doesn't seem to intervene.
    CharacterAlreadyInParty(),
    /// Inform the client they were in a party, and request that they inform us of their return.
    RejoinPartyAfterDisconnect(u64),
    /// Send an arbitrary IPC segment to the client.
    PacketSegment(ServerZoneIpcSegment, ObjectId),
    /// Set of Lua tasks queued up from the server.
    NewTasks(Vec<LuaTask>),
    /// New copy of the status effects list, for use in Lua scripting.
    NewStatusEffects(StatusEffects),
    /// An event object was spawned.
    ObjectSpawn(ObjectSpawn),
}

#[derive(Debug, Clone)]
pub struct ClientHandle {
    pub id: ClientId,
    pub ip: SocketAddr,
    pub channel: Sender<FromServer>,
    pub actor_id: ObjectId,
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

#[derive(Debug)]
pub enum ToServer {
    /// A new zone connection has started.
    NewClient(ClientHandle),
    /// A new chat connection has started.
    NewChatClient(ClientHandle),
    /// The connection sent a message.
    Message(ClientId, MessageInfo),
    /// The connection's player moved.
    ActorMoved(
        ClientId,
        ObjectId,
        Position,
        f32,
        MoveAnimationType,
        MoveAnimationState,
        JumpState,
    ),
    /// The connection has recieved a client trigger.
    ClientTrigger(ClientId, ObjectId, ClientTrigger),
    /// The connection loaded into a zone.
    // TODO: the connection should not be in charge and telling the global server what zone they just loaded in! but this will work for now
    ZoneLoaded(ClientId, ObjectId, PlayerSpawn),
    /// The connection wants to enter a new zone.
    // TODO: temporary as this is only used for commands and those aren't run on global server state yet
    ChangeZone(ClientId, ObjectId, u16, Option<Position>, Option<f32>),
    /// The player walks through a zone change line.
    EnterZoneJump(ClientId, ObjectId, u32),
    /// The connection disconnected.
    Disconnected(ClientId, ObjectId),
    /// A fatal error occured.
    FatalError(std::io::Error),
    /// Spawn an enemy debug NPC.
    DebugNewEnemy(ClientId, ObjectId, u32),
    /// Spawn a debug clone.
    DebugSpawnClone(ClientId, ObjectId),
    /// Request to perform an action
    ActionRequest(ClientId, ObjectId, ActionRequest),
    /// We want to update our own equip display flags.
    Config(ClientId, ObjectId, Config),
    /// Tell the server what models IDs we have equipped.
    Equip(ClientId, ObjectId, u64, u64, [u32; 10]),
    /// The player gains an effect.
    GainEffect(ClientId, ObjectId, u16, u16, f32, ObjectId),
    /// The player loses an effect.
    LoseEffect(ClientId, ObjectId, u16, u16, ObjectId),
    /// Warp with the specified id.
    Warp(ClientId, ObjectId, u32),
    /// Warp with the specified aetheryte id.
    WarpAetheryte(ClientId, ObjectId, u32),
    /// Ready to spawn the player (this happens during initrequest)
    ReadySpawnPlayer(ClientId, ObjectId, u16, Position, f32),
    /// Ready to send the ZoneIn ACS
    ZoneIn(ClientId, ObjectId, bool),
    /// We need to summon a player's minion, and tell other clients
    ActorSummonsMinion(ClientId, ObjectId, u32),
    /// We need to despawn a player's minion, and tell other clients
    ActorDespawnsMinion(ClientId, ObjectId),
    /// Move the player's actor to the specified pop range.
    MoveToPopRange(ClientId, ObjectId, u32, bool),
    /// The connection sent a direct message to another client.
    TellMessageSent(ClientId, ObjectId, SendTellMessage),
    /// The client invited another player to join their party.
    InvitePlayerToParty(ObjectId, u64, String),
    /// The client replied to another player's invite.
    InvitationResponse(ClientId, u64, u64, String, u64, InviteType, InviteReply),
    /// The party leader is adding a member to their party.
    AddPartyMember(u64, ObjectId, u64),
    /// The client sent a message to their party.
    PartyMessageSent(ObjectId, SendPartyMessage),
    /// The client is requesting a social list update.
    RequestSocialList(ClientId, ObjectId, u64, SocialListRequest),
    /// The client is designating another player in the party as leader.
    PartyChangeLeader(u64, u64, u64, String, u64, String),
    /// The client is removing another player from the party.
    PartyMemberKick(u64, u64, u64, String, u64, String),
    /// The client changed areas.
    PartyMemberChangedAreas(u64, u64, u64, String),
    /// The client left their party.
    PartyMemberLeft(u64, u64, u64, ObjectId, String),
    /// The client disbands their party.
    PartyDisband(u64, u64, u64, String),
    /// The chat connection acknowledges the shutdown notice, and now we need to remove it from our internal state.
    ChatDisconnected(ClientId),
    /// The client went offline and we need to inform other party members.
    PartyMemberOffline(u64, u64, u64, ObjectId, String),
    /// The client returned online and we need to inform other party members.
    PartyMemberReturned(ObjectId),
    /// The client is requesting to join the following content.
    JoinContent(ClientId, ObjectId, u16),
    /// Update the global server state of the client's conditions.
    UpdateConditions(ObjectId, Conditions),
    /// (Temporary) Signal to the server to commence the duty.
    CommenceDuty(ClientId, ObjectId),
    /// (Temporary) Signal to the server to kill this actor.
    Kill(ClientId, ObjectId),
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
