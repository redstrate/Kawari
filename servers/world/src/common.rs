use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::sync::mpsc::Sender;

use crate::{StatusEffects, lua::LuaTask, server::Party, zone_connection::BaseParameters};
use kawari::{
    common::{
        CharacterMode, JumpState, LogMessageType, MoveAnimationState, MoveAnimationType, ObjectId,
        ObjectTypeId, Position,
    },
    ipc::{
        chat::{CWLinkshellMessage, ChatChannelType, PartyMessage, TellMessage},
        zone::{
            ActionRequest, ActorControlCategory, CWLSLeaveReason, CWLSPermissionRank,
            ClientTrigger, Conditions, Config, CrossworldLinkshellInvite, InviteReply, InviteType,
            OnlineStatus, PartyMemberEntry, PartyMemberPositions, PartyUpdateStatus,
            ReadyCheckReply, ServerZoneIpcSegment, SpawnNpc, SpawnObject, SpawnPlayer,
            SpawnTreasure, StrategyBoard, StrategyBoardUpdate, WaymarkPlacementMode,
            WaymarkPosition, WaymarkPreset,
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
    Player(SpawnPlayer),
    /// An NPC's spawn data is contained within.
    Npc(SpawnNpc),
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
    ActorControl(ObjectId, ActorControlCategory),
    /// We need to update an actor's target
    ActorControlTarget(ObjectId, ObjectTypeId, ActorControlCategory),
    /// We need to update the player actor
    ActorControlSelf(ActorControlCategory),
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
    ChangeZone(
        u16,
        u16,
        u16,
        Position,
        f32,
        LuaZone,
        bool,
        Option<ServerZoneIpcSegment>,
    ),
    /// The returned position and rotation from ToServer::MoveToPopRange.
    NewPosition(Position, f32, bool),
    /// We need to inform the recipent about the direct message they're receiving.
    TellMessageReceived(ObjectId, TellMessage),
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
    PartyMessageReceived(PartyMessage),
    /// Members of this party need to be informed of an update.
    PartyUpdate(
        PartyUpdateTargets,
        PartyUpdateStatus,
        Option<(u64, u32, ObjectId, Vec<PartyMemberEntry>)>,
    ),
    /// Inform the client of the result of a social invite they have sent.
    InviteCharacterResult(u64, LogMessageType, u16, InviteType, String),
    /// Inform the client they were in a party, and request that they inform us of their return.
    RejoinPartyAfterDisconnect(u64),
    /// Send an arbitrary IPC segment to the client.
    PacketSegment(ServerZoneIpcSegment, ObjectId),
    /// Set of Lua tasks queued up from the server.
    NewTasks(Vec<LuaTask>),
    /// New copy of the status effects list, for use in Lua scripting.
    NewStatusEffects(StatusEffects),
    /// An event object was spawned.
    SpawnObject(SpawnObject),
    /// Inform the client about the location they discovered, so their map can be revealed.
    LocationDiscovered(u32, u32),
    /// Inform the client that a member of their party has shared a strategy board.
    StrategyBoardShared(u64, StrategyBoard),
    /// Inform the sending client that another client received their strategy board.
    StrategyBoardSharedAck(u64),
    /// Inform the client that the board sharer has made a real-time update.
    StrategyBoardRealtimeUpdate(StrategyBoardUpdate),
    /// Inform the client that the board sharer has ended their real-time sharing session.
    StrategyBoardRealtimeFinished(),
    /// Inform the client that a waymark was placed or updated.
    WaymarkUpdated(u8, WaymarkPlacementMode, WaymarkPosition, i32),
    /// Inform the client that a waymark preset was applied.
    WaymarkPreset(WaymarkPreset, i32),
    /// Inform the client that they entered an instance exit range.
    EnteredInstanceEntranceRange(u32),
    /// Inform the client to increment the rested EXP by 10 seconds.
    IncrementRestedExp(),
    /// Inform the client about a countdown that was started in their party.
    Countdown(u64, u64, String, ObjectId, u16),
    /// Inform the client that a sign/marker was applied to a target by someone in their party.
    TargetSignToggled(u32, ObjectId, ObjectTypeId),
    /// Request the client to begin preparing to leave this content.
    LeaveContent(),
    /// Request the client to finish their current event.
    FinishEvent(),
    /// When a fish bites.
    FishBite(),
    /// Inform the client that another player has dismounted.
    ActorDismounted(ObjectId),
    /// Inform the client of the whereabouts of their party members.
    PartyMemberPositionsUpdate(PartyMemberPositions),
    /// Inform the client that they've received a friend request.
    FriendInvite(u64, u64, String),
    /// Use this connections's database to commit the party list for the server.
    CommitParties(HashMap<u64, Party>),
    /// Treasure was spawned.
    TreasureSpawn(SpawnTreasure),
    /// A chat message from one of the client's cwlses has been received.
    CWLSMessageReceived(CWLinkshellMessage),
    /// Inform the zone and chat connections about their linkshell channels.
    SetLinkshellChatChannels(Vec<u32>, Vec<u32>, bool),
    /// Inform the client that one of their linkshells has been disbanded.
    LinkshellDisbanded(u64, String),
    /// Inform the client that a member left one of their linkshells.
    LinkshellLeft(ObjectId, u64, u64, String, CWLSLeaveReason, u64),
    /// Inform the client that an owner of one of their linkshells has renamed it.
    LinkshellRenamed(u64, String, u64, String),
    /// Inform the client that a member of one of their linkshells had their rank changed.
    LinkshellRankChanged(u64, u64, u64, CWLSPermissionRank, String),
    /// Inform the client that they have received a new cross-world linkshell invitation.
    LinkshellInviteReceived(CrossworldLinkshellInvite),
    /// Inform the client that someone has joined their linkshell.
    LinkshellInviteAccepted(u64, u64, String, String),
    /// Inform the zone connection about their current mount's model id. Helps persist mounts across zone transitions and through log-outs.
    SetCurrentMount(u16),
    /// Inform the chat connection that it needs to refresh its non-party ChatChannels due to some event necessitating it.
    MustRefreshChatChannels(),
    /// Inform the client that a friend removal has taken place.
    FriendRemoved(u64, String),
}

#[derive(Debug, Clone)]
pub struct ClientHandle {
    pub id: ClientId,
    pub channel: Sender<FromServer>,
    pub actor_id: ObjectId,
    pub content_id: u64,
    pub account_id: u64,
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
    Message(ObjectId, MessageInfo),
    /// The connection's player moved.
    ActorMoved(
        ObjectId,
        Position,
        f32,
        MoveAnimationType,
        MoveAnimationState,
        JumpState,
        Option<u64>,
    ),
    /// The connection has recieved a client trigger.
    ClientTrigger(ClientId, ObjectId, ClientTrigger),
    /// The connection loaded into a zone.
    // TODO: the connection should not be in charge and telling the global server what zone they just loaded in! but this will work for now
    ZoneLoaded(ClientId, ObjectId, SpawnPlayer),
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
    WarpAetheryte(ClientId, ObjectId, u32, bool),
    /// Ready to spawn the player (this happens during initrequest)
    ReadySpawnPlayer(ClientId, ObjectId, u16, Position, f32),
    /// Ready to send the ZoneIn ACS
    ZoneIn(ClientId, ObjectId, bool),
    /// We need to summon a player's minion, and tell other clients
    ActorSummonsMinion(ObjectId, u32),
    /// We need to despawn a player's minion, and tell other clients
    ActorDespawnsMinion(ObjectId),
    /// Move the player's actor to the specified pop range.
    MoveToPopRange(ClientId, ObjectId, u32, bool),
    /// The connection sent a direct message to another client. This needs the sender's actor id too for purposes of `send_ipc_from`.
    TellMessageSent(ObjectId, ObjectId, TellMessage),
    /// The client invited another player to join their party.
    InvitePlayerToParty(ObjectId, u64, String),
    /// The client replied to another player's invite.
    InvitationResponse(ClientId, u64, u64, String, u64, InviteType, InviteReply),
    /// The party leader is adding a member to their party.
    AddPartyMember(u64, ObjectId, u64),
    /// The client sent a message to their party.
    PartyMessageSent(PartyMessage),
    /// The client is designating another player in the party as leader.
    PartyChangeLeader(u64, u64, u64, String, u64, String),
    /// The client is removing another player from the party.
    PartyMemberKick(u64, u64, u64, String, u64, String),
    /// The client changed areas.
    PartyMemberChangedAreas(u64, u64, u64, ObjectId, String, i32),
    /// The client left their party.
    PartyMemberLeft(u64, u64, u64, ObjectId, String),
    /// The client disbands their party.
    PartyDisband(u64, u64, u64, String),
    /// The chat connection acknowledges the shutdown notice, and now we need to remove it from our internal state.
    ChatDisconnected(ClientId),
    /// The client went offline and we need to inform other party members.
    PartyMemberOffline(u64, u64, u64, ObjectId, String),
    /// The client returned online and we need to inform other party members.
    PartyMemberReturned(ObjectId, i32),
    /// The client is requesting to join the following content.
    JoinContent(ClientId, ObjectId, u16),
    /// The c lient is requesting to leave their current content, the connection is in charge of keeping track of the old position.
    LeaveContent(ClientId, ObjectId, u16, Position, f32),
    /// Update the global server state of the client's conditions.
    UpdateConditions(ObjectId, Conditions),
    /// (Temporary) Signal to the server to commence the duty.
    CommenceDuty(ObjectId),
    /// (Temporary) Signal to the server to kill this actor.
    Kill(ClientId, ObjectId),
    /// Inform the server to update our HP to this value.
    SetHP(ClientId, ObjectId, u32),
    /// Inform the server to update our MP to this value.
    SetMP(ClientId, ObjectId, u16),
    /// The client discovered a new location in this zone.
    NewLocationDiscovered(ClientId, u32, Position, u16),
    /// The client is sharing a strategy board with their party.
    ShareStrategyBoard(ObjectId, u64, u64, u64, StrategyBoard),
    /// The client received a strategy board from another member in their party.
    StrategyBoardReceived(u64, u64, u64),
    /// The client is making edits to their strategy board via real-time sharing.
    StrategyBoardRealtimeUpdate(ObjectId, u64, u64, StrategyBoardUpdate),
    /// The client finished their real-time sharing session.
    StrategyBoardRealtimeFinished(u64),
    /// The client applied a waymark preset for their party.
    ApplyWaymarkPreset(ObjectId, u64, WaymarkPreset, i32),
    /// Inform the server of our new basic stat values.
    SetNewStatValues(ObjectId, u8, u8, BaseParameters),
    /// The client started a countdown in their party.
    StartCountdown(u64, ObjectId, u64, u64, String, ObjectId, u16),
    /// The client yields from a GimmickAccessor.
    GimmickAccessor(ObjectId, ObjectId, Vec<i32>),
    /// The client begins fishing.
    Fish(ClientId, ObjectId),
    /// Warp to a specified pop range in a new territory.
    WarpPopRange(ClientId, ObjectId, u16, u32),
    /// Simulate mounting.
    DebugMount(ClientId, ObjectId, u16),
    /// Request the global server state to reload its Lua state.
    ReloadScripts,
    /// The client dismounted.
    Dismounted(ObjectId, Option<u64>),
    /// Inform the server of this actor's new online status.
    SetOnlineStatus(ObjectId, OnlineStatus),
    /// The client is requesting to ride pillion with a party member's mount.
    RidePillionRequest(ObjectId, Option<u64>, ObjectId, u32),
    /// Inform the server of this actor's new CharacterMode.
    SetCharacterMode(ObjectId, CharacterMode, u8),
    /// Broadcasts an actor control to other players.
    BroadcastActorControl(ObjectId, ActorControlCategory),
    /// The client invited another player to be friends.
    InvitePlayerToFriendList(ObjectId, u64, String),
    /// The client initiated a ready check for their party.
    ReadyCheckInitiated(Option<u64>, ObjectId, u64, u64, String),
    /// The client responded to an on-going ready check in their party.
    ReadyCheckResponse(Option<u64>, ObjectId, u64, u64, String, ReadyCheckReply),
    /// Removes action cooldowns for this player.
    RemoveCooldowns(ObjectId),
    /// The client's zone connection wishes to inform its chat connection about any linkshells the player belongs to.
    SetLinkshells(ObjectId, Vec<u64>),
    /// The client sent a message to a cross-world linkshell.
    CWLSMessageSent(CWLinkshellMessage),
    /// The client disbanded their linkshell, and online members need to be informed.
    DisbandLinkshell(u64, String),
    /// The client left a linkshell, and online members need to be informed.
    LeaveLinkshell(ObjectId, u64, u64, String, CWLSLeaveReason, u64),
    /// The client renamed their linkshell.
    RenameLinkshell(u64, String, u64, String),
    /// The client changed the rank of a member in their linkshell.
    SetLinkshellRank(u64, u64, u64, CWLSPermissionRank, String),
    /// The client invited another character to join their linkshell.
    SendLinkshellInvite(ObjectId, CrossworldLinkshellInvite),
    /// The client accepted an invite to a linkshell.
    AcceptedLinkshellInvite(ObjectId, u64, u64, String, String),
    /// The client removes a player from their friend list.
    FriendRemoved(ObjectId, u64, String, ObjectId, u64, String),
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
