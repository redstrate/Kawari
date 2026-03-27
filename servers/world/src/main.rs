use std::sync::Arc;
use std::time::{Instant, SystemTime};

use axum::Router;
use axum::routing::get;
use kawari::common::{
    ContainerType, DirectorEvent, DirectorTrigger, DutyOption, HandlerId, HandlerType,
    ItemOperationKind, ObjectId, ObjectTypeId, ObjectTypeKind, PlayerStateFlags1,
    PlayerStateFlags2, PlayerStateFlags3, Position, calculate_max_level,
};
use kawari::config::{FilesystemConfig, get_config};
use kawari_world::inventory::{EquipSlot, Item, Storage, get_next_free_slot};

use kawari::ipc::chat::{ChatChannel, ClientChatIpcData};

use kawari::ipc::zone::{
    ActorControlCategory, Conditions, ContentFinderUserAction, CrossRealmListing,
    CrossRealmListings, CrossworldLinkshellEx, EventType, InviteType, MapEffects, MarketBoardItem,
    OnlineStatus, OnlineStatusMask, PlayerSetup, SceneFlags, SearchInfo, SocialListRequestType,
    TrustContent, TrustInformation,
};

use kawari::ipc::zone::{
    Blacklist, BlacklistedCharacter, ClientTriggerCommand, ClientZoneIpcData, ReadyCheckReply,
    ServerZoneIpcData, ServerZoneIpcSegment,
};

use kawari::common::{CharacterMode, NETWORK_TIMEOUT, RECEIVE_BUFFER_SIZE};
use kawari::constants::{AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, CLASSJOB_ARRAY_SIZE};
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{ConnectionState, ConnectionType, SegmentData, parse_packet_header};
use kawari_world::lua::{KawariLua, KawariLuaState, LuaPlayer};
use kawari_world::{
    ChatConnection, ChatHandler, CustomIpcConnection, Event, EventHandler, GameData,
    ObsfucationData, TeleportReason, ZoneConnection,
};
use kawari_world::{
    ClientHandle, ClientId, FromServer, MessageInfo, PlayerData, ServerHandle, ToServer,
    WorldDatabase, server_main_loop,
};

use mlua::Function;
use parking_lot::Mutex;
use tokio::io::AsyncReadExt;
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender, channel, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use kawari::common::INVENTORY_ACTION_ACK_GENERAL;

fn spawn_main_loop(
    game_data: Arc<Mutex<GameData>>,
    database: Arc<Mutex<WorldDatabase>>,
) -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(64);

    let handle = ServerHandle {
        chan: send,
        next_id: Default::default(),
    };

    let join = tokio::spawn(async move {
        let game_data_new;
        {
            // We let it clone our GameData so it doesn't take 2x the time to load.
            let game_data_mutex = game_data.lock();
            game_data_new = game_data_mutex.clone();
        }
        let parties;
        let linkshells;
        {
            let mut database = database.lock();
            parties = database.get_parties();
            linkshells = database.find_all_linkshells();
        }
        let res = server_main_loop(game_data_new, parties, linkshells, recv).await;
        match res {
            Ok(()) => {}
            Err(err) => {
                tracing::error!("{}", err);
            }
        }
    });

    (handle, join)
}

/// A task that will process the first few packets from each connection.
fn spawn_initial_setup(
    id: ClientId,
    socket: TcpStream,
    lua: Arc<Mutex<KawariLua>>,
    database: Arc<Mutex<WorldDatabase>>,
    gamedata: Arc<Mutex<GameData>>,
    handle: ServerHandle,
) {
    let _kill = tokio::spawn(initial_setup(id, socket, lua, database, gamedata, handle));
}

/// The initial setup loop, which figures out what the remote connection wants and branches off to provide a chat connection, zone connection, or custom IPC connection.
async fn initial_setup(
    id: ClientId,
    mut socket: TcpStream,
    lua: Arc<Mutex<KawariLua>>,
    database: Arc<Mutex<WorldDatabase>>,
    game_data: Arc<Mutex<GameData>>,
    handle: ServerHandle,
) {
    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];

    match socket.read(&mut buf).await {
        Ok(n) => {
            let header = parse_packet_header(&buf[..n]);
            if header.connection_type == ConnectionType::KawariIpc {
                let mut connection = CustomIpcConnection {
                    socket,
                    state: ConnectionState::None,
                    database: database.clone(),
                    gamedata: game_data.clone(),
                };
                // Handle the first batch of segments before handing off control to the loop proper.
                let segments = connection.parse_packet(&buf[..n]);
                for segment in segments {
                    match &segment.data {
                        SegmentData::KawariIpc(data) => connection.handle_custom_ipc(data).await,
                        _ => panic!(
                            "initial_setup: The KawariIpc connection type only supports KawariIpc segments! Was a mistake made somewhere? Received: {segment:#?}"
                        ),
                    }
                }
            } else if header.connection_type == ConnectionType::Zone {
                let state = ConnectionState::Zone {
                    clientbound_oodle: OodleNetwork::new(),
                    serverbound_oodle: OodleNetwork::new(),
                    scrambler_keys: None,
                };
                let mut connection = ZoneConnection {
                    config: get_config().world,
                    socket,
                    state,
                    player_data: PlayerData::default(),
                    id,
                    handle: handle.clone(),
                    database: database.clone(),
                    lua: lua.clone(),
                    gamedata: game_data.clone(),
                    last_keep_alive: Instant::now(),
                    gracefully_logged_out: false,
                    obsfucation_data: ObsfucationData::default(),
                    queued_content: None,
                    conditions: Conditions::default(),
                    queued_tasks: Vec::new(),
                    old_zone_id: 0,
                    old_position: Position::default(),
                    old_rotation: 0.0,
                    content_handler_id: HandlerId::default(),
                    teleport_reason: TeleportReason::NotSpecified,
                    active_minion: 0,
                    party_id: 0,
                    rejoining_party: false,
                    login_time: None,
                    content_settings: None,
                    current_instance_id: None,
                    glamour_information: None,
                    event_handler_id: None,
                    recipe: None,
                    is_party_leader: false,
                    synced_level: None,
                    search_results: Vec::new(),
                    search_index: 0,
                    friend_results: Vec::new(),
                    friend_index: 0,
                    cwls_results: Vec::new(),
                    cwls_index: 0,
                    cwls_memberships: None,
                };

                // Handle setup before passing off control to the zone connection.
                let segments = connection.parse_packet(&buf[..n]);
                for segment in segments {
                    match &segment.data {
                        SegmentData::Setup { actor_id } => {
                            // for some reason they send a string representation
                            let actor_id = actor_id.parse::<u32>().unwrap();

                            // initialize player data if it doesn't exist
                            if !connection.player_data.character.actor_id.is_valid() {
                                let player_data;
                                {
                                    let mut game_data = connection.gamedata.lock();
                                    let mut database = connection.database.lock();
                                    player_data = database
                                        .find_player_data(ObjectId(actor_id), &mut game_data);
                                }
                                connection.player_data = player_data;
                            }

                            // collect actor data
                            connection.initialize(actor_id).await;
                        }
                        _ => panic!(
                            "initial_setup: The zone connection type must start with a Setup segment! What happened? Received: {segment:#?}"
                        ),
                    }
                }
                spawn_client(connection);
            } else if header.connection_type == ConnectionType::Chat {
                let state = ConnectionState::Zone {
                    clientbound_oodle: OodleNetwork::new(),
                    serverbound_oodle: OodleNetwork::new(),
                    scrambler_keys: None,
                };

                let mut connection = ChatConnection {
                    config: get_config().world,
                    id,
                    actor_id: ObjectId::default(),
                    state,
                    last_keep_alive: Instant::now(),
                    socket,
                    handle,
                    party_chatchannel: ChatChannel::default(),
                    cwls_chatchannels: [ChatChannel::default(); CrossworldLinkshellEx::COUNT],
                    local_ls_chatchannels: [ChatChannel::default(); CrossworldLinkshellEx::COUNT],
                };

                // Handle setup before passing off control to the chat connection.
                let segments = connection.parse_packet(&buf[..n]);
                for segment in segments {
                    match &segment.data {
                        SegmentData::Setup { actor_id } => {
                            // for some reason they send a string representation
                            let actor_id = actor_id.parse::<u32>().unwrap();
                            connection.actor_id = ObjectId(actor_id);
                            connection.initialize().await;
                        }
                        _ => panic!(
                            "initial_setup: The chat connection type must start with a Setup segment! What happened? Received: {segment:#?}"
                        ),
                    }
                }
                spawn_chat_connection(connection);
            } else {
                tracing::error!("Connection type is None! How did this happen?");
            }
        }
        Err(err) => {
            tracing::error!("Error while setting up connection: {err:?}");
        }
    }
}

// TODO: Is there a sensible way we can reuse the other ClientData type so we don't need 2?
struct ClientChatData {
    /// Socket for data recieved from the global server
    recv: Receiver<FromServer>,
    connection: ChatConnection,
}

/// Spawn a new chat connection for an incoming client.
fn spawn_chat_connection(connection: ChatConnection) {
    let (send, recv) = channel(64);

    let id = &connection.id.clone();
    let actor_id = &connection.actor_id.clone();

    let data = ClientChatData { recv, connection };

    // Spawn a new client task
    let (my_send, my_recv) = oneshot::channel();
    let _kill = tokio::spawn(start_chat_connection(my_recv, data));

    // Send client information to said task
    let handle = ClientHandle {
        id: *id,
        channel: send,
        actor_id: *actor_id, // We have the actor id by this point, since Setup is done earlier
        account_id: 0,       // TODO: fill as we need it
        content_id: 0,
    };
    let _ = my_send.send(handle);
}

/// The task that kickstarts the client chat connection loop.
async fn start_chat_connection(my_handle: oneshot::Receiver<ClientHandle>, data: ClientChatData) {
    // Recieve client information from global
    let my_handle = match my_handle.await {
        Ok(my_handle) => my_handle,
        Err(_) => return,
    };

    let connection = data.connection;
    let recv = data.recv;

    // communication channel between client_loop and client_server_loop
    let (internal_send, internal_recv) = unbounded_channel();

    let _ = join!(
        tokio::spawn(client_chat_loop(connection, internal_recv, my_handle)),
        // TODO: It should be okay to reuse the zone connection's client_server_loop for ToServer & FromServer stuff since it gives us our own comms channel, right?
        tokio::spawn(client_server_loop(recv, internal_send))
    );
}

/// The client's chat connection loop, which allows them to communicate with other players in channels that span across the entire world.
async fn client_chat_loop(
    mut connection: ChatConnection,
    mut internal_recv: UnboundedReceiver<FromServer>,
    client_handle: ClientHandle,
) {
    // tell the server we exist, now that we confirmed we are a legitimate connection
    connection
        .handle
        .send(ToServer::NewChatClient(client_handle.clone()))
        .await;

    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    loop {
        tokio::select! {
            biased; // client data should always be prioritized

            n = connection.socket.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        // if the last response was over >5 seconds, the client is probably gone
                        if n == 0 {
                            let now = Instant::now();
                            if now.duration_since(connection.last_keep_alive) > NETWORK_TIMEOUT {
                                tracing::info!("ChatConnection {:#?} was killed because of timeout", client_handle.id);
                                break;
                            }
                        } else {
                            connection.last_keep_alive = Instant::now();
                            let segments = connection.parse_packet(&buf);
                            for segment in segments {
                                match &segment.data {
                                    SegmentData::None() => {}
                                    SegmentData::Setup { .. } => {
                                        // Handled before our connection was spawned!
                                    }
                                    SegmentData::Ipc(data) => {
                                        match &data.data {
                                            ClientChatIpcData::SendTellMessage(data) => {
                                                connection.handle.send(ToServer::TellMessageSent(connection.id, connection.actor_id, data.clone())).await;
                                            }
                                            ClientChatIpcData::SendPartyMessage(data) => {
                                                if data.chatchannel == connection.party_chatchannel {
                                                    connection.handle.send(ToServer::PartyMessageSent(connection.actor_id, data.clone())).await;
                                                } else {
                                                    tracing::error!("The client tried to send a party message to an invalid ChatChannel: {:#?}, while ours is {:#?}", data.chatchannel, connection.party_chatchannel);
                                                }
                                            }
                                            ClientChatIpcData::GetChannelList { unk } => {
                                                tracing::info!("GetChannelList: {:#?} from {}", unk, connection.actor_id);
                                            }
                                            ClientChatIpcData::SendCWLinkshellMessage(data) => {
                                                if connection.cwls_chatchannels.contains(&data.chatchannel) {
                                                    connection.handle.send(ToServer::CWLSMessageSent(connection.actor_id, data.clone())).await;
                                                } else {
                                                    tracing::error!("The client tried to send a party message to an invalid ChatChannel: {:#?}, while ours are {:#?}", data.chatchannel, connection.cwls_chatchannels);
                                                }
                                            }
                                            ClientChatIpcData::SendAllianceMessage(_data) => {
                                                tracing::info!("Chatting in alliances is unimplemented");
                                            }
                                            ClientChatIpcData::Unknown { unk } => {
                                                tracing::warn!("Unknown Chat packet {:?} recieved ({} bytes), this should be handled!", data.header.op_code, unk.len());
                                            }
                                        }
                                    }
                                    SegmentData::KeepAliveRequest { id, timestamp } => connection.send_keep_alive(*id, *timestamp).await,
                                    SegmentData::KeepAliveResponse { .. } => {
                                        // these should be safe to ignore
                                    }
                                    _ => panic!("ChatConnection: The server is receiving a response or an unknown packet: {segment:#?}"),
                                }
                            }
                        }
                    }
                    Err(_) => {
                        tracing::info!("ChatConnection {:#?} was killed because of a network error!", client_handle.id);
                        break;
                    },
                }
            }

            msg = internal_recv.recv() => match msg {
                Some(msg) => match msg {
                    FromServer::TellMessageSent(message_info) => connection.tell_message_received(message_info).await,
                    FromServer::TellRecipientNotFound(error_info) => connection.tell_recipient_not_found(error_info).await,
                    FromServer::ChatDisconnected() => {
                        tracing::info!("ChatConnection {:#?} received shutdown, disconnecting!", connection.id);
                        break;
                    }
                    FromServer::SetPartyChatChannel(channel_id) => connection.set_party_chatchannel(channel_id).await,
                    FromServer::PartyMessageSent(message_info) => connection.party_message_received(message_info).await,
                    FromServer::SetLinkshellChatChannels(cwls, local, _) => connection.set_linkshell_chatchannels(cwls, local).await,
                    FromServer::CWLSMessageSent(message_info) => connection.cwls_message_received(message_info).await,
                    FromServer::LinkshellDisbanded(_, channel_id) => connection.linkshell_disbanded(channel_id).await,
                    FromServer::LinkshellLeft(from_actor_id, _, _, _, _, channel_id) => connection.linkshell_left(from_actor_id, channel_id).await,
                    _ => tracing::error!("ChatConnection {:#?} received a FromServer message we don't care about: {:#?}, ensure you're using the right client network or that you've implemented a handler for it if we actually care about it!", client_handle.id, msg),
                },
                None => break,
            }
        }
    }

    connection
        .handle
        .send(ToServer::ChatDisconnected(connection.id))
        .await;
}

struct ClientZoneData {
    /// Socket for data recieved from the global server
    recv: Receiver<FromServer>,
    connection: ZoneConnection,
}

/// Spawn a new client actor.
fn spawn_client(connection: ZoneConnection) {
    let (send, recv) = channel(64);

    let id = &connection.id.clone();
    let actor_id = &connection.player_data.character.actor_id.clone();
    let content_id = &connection.player_data.character.content_id.clone();
    let account_id = &connection.player_data.character.service_account_id.clone();

    let data = ClientZoneData { recv, connection };

    // Spawn a new client task
    let (my_send, my_recv) = oneshot::channel();
    let _kill = tokio::spawn(start_client(my_recv, data));

    // Send client information to said task
    let handle = ClientHandle {
        id: *id,
        channel: send,
        actor_id: *actor_id, // We have the actor id by this point, since Setup is done earlier
        content_id: *content_id as u64,
        account_id: *account_id as u64,
    };
    let _ = my_send.send(handle);
}

async fn start_client(my_handle: oneshot::Receiver<ClientHandle>, data: ClientZoneData) {
    // Recieve client information from global
    let my_handle = match my_handle.await {
        Ok(my_handle) => my_handle,
        Err(_) => return,
    };

    let connection = data.connection;
    let recv = data.recv;

    // communication channel between client_loop and client_server_loop
    let (internal_send, internal_recv) = unbounded_channel();

    let _ = join!(
        tokio::spawn(client_loop(connection, internal_recv, my_handle)),
        tokio::spawn(client_server_loop(recv, internal_send))
    );
}

async fn client_server_loop(
    mut data: Receiver<FromServer>,
    internal_send: UnboundedSender<FromServer>,
) {
    while let Some(msg) = data.recv().await {
        let Ok(_) = internal_send.send(msg) else {
            // In most cases this should be fine, as it's simply the zone and chat connection loops ending
            // If needed we can log messages here for troubleshooting, but previously this was just an unwrap which assumed it'd never fail
            break;
        };
    }
}

/// Process packets from the client. Returns false if we want to kill the connection.
async fn process_packet(
    connection: &mut ZoneConnection,
    lua_player: &mut LuaPlayer,
    events: &mut Vec<(Box<dyn EventHandler>, Event)>,
    client_handle: ClientHandle,
    n: usize,
    buf: &[u8],
) -> bool {
    let config = get_config();

    // if the last response was over >5 seconds, the client is probably gone
    if n == 0 {
        let now = Instant::now();
        if now.duration_since(connection.last_keep_alive) > NETWORK_TIMEOUT {
            tracing::info!(
                "ZoneConnection {:#?} was killed because of timeout",
                client_handle.id
            );
            return false;
        }
    } else {
        connection.last_keep_alive = Instant::now();

        let segments = connection.parse_packet(&buf[..n]);
        for segment in &segments {
            match &segment.data {
                SegmentData::None() => {}
                SegmentData::Setup { .. } => {
                    // Handled before our connection was spawned!
                }
                SegmentData::Ipc(data) => {
                    match &data.data {
                        ClientZoneIpcData::InitRequest { .. } => {
                            tracing::info!("Client is now requesting zone information. Sending!");

                            // IPC Init(?)
                            {
                                let ipc =
                                    ServerZoneIpcSegment::new(ServerZoneIpcData::InitResponse {
                                        actor_id: connection.player_data.character.actor_id,
                                    });
                                connection.send_ipc_self(ipc).await;
                            }

                            let service_account_id;
                            {
                                let mut database = connection.database.lock();
                                service_account_id = database.find_service_account(
                                    connection.player_data.character.content_id as u64,
                                );
                            }

                            let Ok(mut login_reply) = ureq::get(format!(
                                "{}/_private/max_ex?service={}",
                                config.login.server_name, service_account_id,
                            ))
                            .call() else {
                                tracing::warn!(
                                    "Failed to find service account {service_account_id}, just going to stop talking to this connection..."
                                );
                                return false;
                            };

                            let expansion = login_reply
                                .body_mut()
                                .read_to_string()
                                .unwrap()
                                .parse()
                                .unwrap();
                            // Send inventory
                            connection.send_inventory().await;

                            connection.ensure_valid_zone().await;

                            // set equip display flags
                            connection
                                .actor_control_self(ActorControlCategory::SetEquipDisplayFlags {
                                    display_flag: connection.player_data.volatile.display_flags,
                                })
                                .await;

                            // Store when we logged in, for various purposes.
                            connection.login_time = Some(SystemTime::now());

                            // Mark the player as online for total player counts, player searches, etc.
                            {
                                connection.player_data.volatile.is_online = true;
                                let mut database = connection.database.lock();
                                database.commit_volatile(&connection.player_data);
                            }

                            // Stats
                            connection.send_stats().await;

                            // As seen in retail, they pad it with the first value
                            let mut padded_exp = connection.player_data.classjob.exp.0.clone();
                            padded_exp.resize(CLASSJOB_ARRAY_SIZE, padded_exp[0]);

                            // Ditto for levels
                            let mut padded_levels: Vec<u16> =
                                connection.player_data.classjob.levels.0.to_vec();
                            padded_levels.resize(CLASSJOB_ARRAY_SIZE, padded_levels[0]);

                            let chara_make;
                            let city_state;
                            {
                                let mut database = connection.database.lock();
                                chara_make = database.get_chara_make(
                                    connection.player_data.character.content_id as u64,
                                );
                                city_state = database.get_city_state(
                                    connection.player_data.character.content_id as u64,
                                );
                            }

                            // Player Setup
                            {
                                let mut player_state_flags1 = PlayerStateFlags1::NONE;
                                if connection.player_data.mentor.is_novice == 0 {
                                    player_state_flags1.set(PlayerStateFlags1::NOT_NOVICE, true);
                                }
                                if connection.player_data.mentor.is_battle == 1 {
                                    player_state_flags1.set(PlayerStateFlags1::BATTLE_MENTOR, true);
                                }

                                let mut player_state_flags2 = PlayerStateFlags2::NONE;
                                if connection.player_data.mentor.is_returner == 1 {
                                    player_state_flags2.set(PlayerStateFlags2::RETURNER, true);
                                }

                                let mut player_state_flags3 = PlayerStateFlags3::NONE;
                                if connection.player_data.mentor.is_trade == 1 {
                                    player_state_flags3.set(PlayerStateFlags3::TRADE_MENTOR, true);
                                }

                                let ipc = ServerZoneIpcSegment::new(
                                    ServerZoneIpcData::PlayerSetup(PlayerSetup {
                                        content_id: connection.player_data.character.content_id
                                            as u64,
                                        player_state_flags1,
                                        player_state_flags2,
                                        player_state_flags3,
                                        exp: padded_exp,
                                        max_level: calculate_max_level(expansion),
                                        expansion,
                                        name: connection.player_data.character.name.clone(),
                                        actor_id: connection.player_data.character.actor_id,
                                        race: chara_make.customize.race,
                                        gender: chara_make.customize.gender,
                                        tribe: chara_make.customize.subrace,
                                        city_state,
                                        nameday_month: chara_make.birth_month as u8,
                                        nameday_day: chara_make.birth_day as u8,
                                        deity: chara_make.guardian as u8,
                                        current_class: connection.player_data.classjob.current_class
                                            as u8,
                                        first_class: connection.player_data.classjob.first_class
                                            as u8,
                                        levels: padded_levels,
                                        unlocks: connection.player_data.unlock.unlocks.data.clone(),
                                        aetherytes: connection
                                            .player_data
                                            .aetheryte
                                            .unlocked
                                            .data
                                            .clone(),
                                        minions: connection.player_data.unlock.minions.data.clone(),
                                        mounts: connection.player_data.unlock.mounts.data.clone(),
                                        homepoint: connection.player_data.aetheryte.homepoint
                                            as u16,
                                        favourite_aetheryte_count: 1,
                                        favorite_aetheryte_ids: [8, 0, 0, 0],
                                        seen_active_help: connection
                                            .player_data
                                            .unlock
                                            .seen_active_help
                                            .data
                                            .clone(),
                                        aether_currents_mask: connection
                                            .player_data
                                            .aether_current
                                            .unlocked
                                            .data
                                            .clone(),
                                        orchestrion_roll_mask: connection
                                            .player_data
                                            .unlock
                                            .orchestrion_rolls
                                            .data
                                            .clone(),
                                        buddy_equip_mask: connection
                                            .player_data
                                            .companion
                                            .unlocked_equip
                                            .data
                                            .clone(),
                                        cutscene_seen_mask: connection
                                            .player_data
                                            .unlock
                                            .cutscene_seen
                                            .data
                                            .clone(),
                                        ornament_mask: connection
                                            .player_data
                                            .unlock
                                            .ornaments
                                            .data
                                            .clone(),
                                        caught_fish_mask: connection
                                            .player_data
                                            .unlock
                                            .caught_fish
                                            .data
                                            .clone(),
                                        caught_spearfish_mask: connection
                                            .player_data
                                            .unlock
                                            .caught_spearfish
                                            .data
                                            .clone(),
                                        adventure_mask: connection
                                            .player_data
                                            .unlock
                                            .adventures
                                            .data
                                            .clone(),
                                        triple_triad_cards: connection
                                            .player_data
                                            .unlock
                                            .triple_triad_cards
                                            .data
                                            .clone(),
                                        glasses_styles_mask: connection
                                            .player_data
                                            .unlock
                                            .glasses_styles
                                            .data
                                            .clone(),
                                        chocobo_taxi_stands_mask: connection
                                            .player_data
                                            .unlock
                                            .chocobo_taxi_stands
                                            .data
                                            .clone(),
                                        aether_current_comp_flg_set_bitmask1: connection
                                            .player_data
                                            .aether_current
                                            .comp_flg_set
                                            .data[0],
                                        aether_current_comp_flg_set_bitmask2: connection
                                            .player_data
                                            .aether_current
                                            .comp_flg_set
                                            .data[1..AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE]
                                            .to_vec(),

                                        // content
                                        unlocked_special_content: connection
                                            .player_data
                                            .content
                                            .unlocked_special_content
                                            .data
                                            .clone(),
                                        unlocked_dungeons: connection
                                            .player_data
                                            .content
                                            .unlocked_dungeons
                                            .data
                                            .clone(),
                                        unlocked_raids: connection
                                            .player_data
                                            .content
                                            .unlocked_raids
                                            .data
                                            .clone(),
                                        unlocked_guildhests: connection
                                            .player_data
                                            .content
                                            .unlocked_guildhests
                                            .data
                                            .clone(),
                                        unlocked_trials: connection
                                            .player_data
                                            .content
                                            .unlocked_trials
                                            .data
                                            .clone(),
                                        unlocked_crystalline_conflict: connection
                                            .player_data
                                            .content
                                            .unlocked_crystalline_conflicts
                                            .data
                                            .clone(),
                                        unlocked_frontline: connection
                                            .player_data
                                            .content
                                            .unlocked_frontlines
                                            .data
                                            .clone(),
                                        cleared_raids: connection
                                            .player_data
                                            .content
                                            .cleared_raids
                                            .data
                                            .clone(),
                                        cleared_dungeons: connection
                                            .player_data
                                            .content
                                            .cleared_dungeons
                                            .data
                                            .clone(),
                                        cleared_guildhests: connection
                                            .player_data
                                            .content
                                            .cleared_guildhests
                                            .data
                                            .clone(),
                                        cleared_trials: connection
                                            .player_data
                                            .content
                                            .cleared_trials
                                            .data
                                            .clone(),
                                        cleared_crystalline_conflict: connection
                                            .player_data
                                            .content
                                            .cleared_crystalline_conflicts
                                            .data
                                            .clone(),
                                        cleared_frontline: connection
                                            .player_data
                                            .content
                                            .cleared_frontlines
                                            .data
                                            .clone(),
                                        cleared_masked_carnivale: connection
                                            .player_data
                                            .content
                                            .cleared_masked_carnivale
                                            .data
                                            .clone(),
                                        unlocked_misc_content: connection
                                            .player_data
                                            .content
                                            .unlocked_misc_content
                                            .data
                                            .clone(),
                                        cleared_misc_content: connection
                                            .player_data
                                            .content
                                            .cleared_misc_content
                                            .data
                                            .clone(),
                                        can_do_triple_triad_matches: true,
                                        ..Default::default()
                                    }),
                                );
                                connection.send_ipc_self(ipc).await;
                            }

                            let level;
                            {
                                let mut game_data = connection.gamedata.lock();

                                level = connection
                                    .player_data
                                    .inventory
                                    .equipped
                                    .calculate_item_level(&mut game_data)
                                    as u32;
                            }

                            connection
                                .actor_control_self(ActorControlCategory::SetItemLevel { level })
                                .await;

                            connection
                                .handle
                                .send(ToServer::ReadySpawnPlayer(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    connection.player_data.volatile.zone_id as u16,
                                    connection.player_data.volatile.position,
                                    connection.player_data.volatile.rotation as f32,
                                ))
                                .await;

                            //connection.remind_pending_invites().await;
                            connection.init_linkshells().await;

                            // Send login message
                            connection.send_notice(&config.world.login_message).await;

                            let online_player_count;
                            {
                                let mut db = connection.database.lock();
                                online_player_count = db.get_online_player_count();
                            }

                            connection
                                .send_notice(&format!(
                                    "There are currently {} players online.",
                                    online_player_count
                                ))
                                .await;
                        }
                        ClientZoneIpcData::FinishLoading { .. } => {
                            let spawn = connection.respawn_player(true).await;

                            // tell the server we loaded into the zone, so it can start sending us actors
                            connection
                                .handle
                                .send(ToServer::ZoneLoaded(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    spawn.clone(),
                                ))
                                .await;

                            // If we're in a party, we need to tell the other members we changed areas or reconnected.
                            if connection.is_in_party() {
                                if !connection.rejoining_party {
                                    connection
                                        .handle
                                        .send(ToServer::PartyMemberChangedAreas(
                                            connection.party_id,
                                            connection.player_data.character.service_account_id
                                                as u64,
                                            connection.player_data.character.content_id as u64,
                                            connection.player_data.character.actor_id,
                                            connection.player_data.character.name.clone(),
                                            connection.player_data.volatile.zone_id,
                                        ))
                                        .await;
                                } else {
                                    connection
                                        .handle
                                        .send(ToServer::PartyMemberReturned(
                                            connection.player_data.character.actor_id,
                                            connection.player_data.volatile.zone_id,
                                        ))
                                        .await;
                                    connection.rejoining_party = false;
                                }
                            }

                            connection.send_stats().await;
                        }
                        ClientZoneIpcData::ClientTrigger(trigger) => {
                            match trigger.trigger {
                                ClientTriggerCommand::RequestTitleList {} => {
                                    let ipc =
                                        ServerZoneIpcSegment::new(ServerZoneIpcData::TitleList {
                                            unlock_bitmask: connection
                                                .player_data
                                                .unlock
                                                .titles
                                                .data
                                                .clone(),
                                        });
                                    connection.send_ipc_self(ipc).await;
                                }
                                ClientTriggerCommand::FinishZoning {} => {
                                    connection
                                        .handle
                                        .send(ToServer::ZoneIn(
                                            connection.id,
                                            connection.player_data.character.actor_id,
                                            connection.teleport_reason == TeleportReason::Aetheryte,
                                        ))
                                        .await;

                                    // Reset so it doesn't get stuck to Aetheryte:
                                    connection.teleport_reason = TeleportReason::NotSpecified;

                                    // Initialize map effects to their default state.
                                    // TODO: find a better place to do this?
                                    if let Some(instance_id) = connection.current_instance_id {
                                        let map_effects;
                                        {
                                            let mut game_data = connection.gamedata.lock();
                                            map_effects =
                                                game_data.get_map_effects(instance_id as u32)
                                        }

                                        if let Some(map_effects) = map_effects {
                                            let mut states = Vec::new();
                                            for (i, layout_id) in map_effects.iter().enumerate() {
                                                // A layout ID of zero means the effect should be skipped.
                                                if *layout_id != 0 {
                                                    states.resize(i + 1, 0);
                                                    states[i] = 4; // 4 means to play it, I guess?
                                                }
                                            }

                                            let ipc = MapEffects {
                                                handler_id: connection.content_handler_id,
                                                unk_flag: 5,
                                                states,
                                                ..Default::default()
                                            }
                                            .package()
                                            .unwrap();
                                            connection.send_ipc_self(ipc).await;
                                        }
                                    }
                                }
                                ClientTriggerCommand::BeginContentsReplay {} => {
                                    connection
                                        .actor_control_self(
                                            ActorControlCategory::BeginContentsReplay { unk1: 1 },
                                        )
                                        .await;
                                }
                                ClientTriggerCommand::EndContentsReplay {} => {
                                    connection
                                        .actor_control_self(
                                            ActorControlCategory::EndContentsReplay { unk1: 1 },
                                        )
                                        .await;

                                    connection.respawn_player(false).await;
                                }
                                ClientTriggerCommand::Dismount { sequence } => {
                                    // TODO: Move all this to FromServer::ActorDismounted so all of the logic can be consolidated
                                    connection.conditions = Conditions::default();
                                    connection.send_conditions().await;

                                    // SetMode isn't important, no, but it's included for accuracy.
                                    connection
                                        .set_character_mode(CharacterMode::Normal, 0)
                                        .await;

                                    // Retail indeed does send an AC, not an ACS for this.
                                    connection
                                        .actor_control(
                                            connection.player_data.character.actor_id,
                                            ActorControlCategory::PlayDismountAnimation {
                                                unk1: 47494,
                                                unk2: 32711,
                                                unk3: 1510381914,
                                            },
                                        )
                                        .await;

                                    // TODO: This should only be sent when the player is actually riding pillion, but for now, it doesn't seem to hurt sending it unconditionally.
                                    connection
                                        .actor_control(
                                            connection.player_data.character.actor_id,
                                            ActorControlCategory::RidePillion {
                                                target_actor_id: ObjectId::default(),
                                                target_seat_index: 0,
                                            },
                                        )
                                        .await;

                                    connection
                                        .actor_control_self(ActorControlCategory::Dismount {
                                            sequence,
                                        })
                                        .await;

                                    // Then these are also sent!
                                    connection
                                        .actor_control_self(ActorControlCategory::SetPetEntityId {
                                            unk1: 0,
                                        })
                                        .await;

                                    connection
                                        .actor_control_self(ActorControlCategory::CompanionUnlock {
                                            unk1: 0,
                                            unk2: 0,
                                        })
                                        .await;

                                    connection
                                        .actor_control_self(
                                            ActorControlCategory::SetPetParameters {
                                                pet_id: 0,
                                                unk2: 0,
                                                unk3: 0,
                                                unk4: 7,
                                            },
                                        )
                                        .await;

                                    // TODO: Remove this `let party_id` in an upcoming party refactor, this is temporary
                                    let party_id = if connection.party_id != 0 {
                                        Some(connection.party_id)
                                    } else {
                                        None
                                    };
                                    connection
                                        .handle
                                        .send(ToServer::Dismounted(
                                            connection.player_data.character.actor_id,
                                            party_id,
                                        ))
                                        .await;
                                }
                                ClientTriggerCommand::ShownActiveHelp { id } => {
                                    // Save this so it isn't shown again on next login
                                    connection.player_data.unlock.seen_active_help.set(id);
                                }
                                ClientTriggerCommand::SeenCutscene { id } => {
                                    connection.player_data.unlock.cutscene_seen.set(id);
                                }
                                ClientTriggerCommand::DirectorTrigger {
                                    handler_id,
                                    trigger,
                                    arg,
                                } => {
                                    match trigger {
                                        DirectorTrigger::Sync => {
                                            // Always send a sync response for now
                                            connection
                                                .actor_control_self(
                                                    ActorControlCategory::DirectorEvent {
                                                        handler_id,
                                                        event: DirectorEvent::SyncResponse,
                                                        arg: 1,
                                                        unk1: 0,
                                                    },
                                                )
                                                .await;
                                        }
                                        DirectorTrigger::SummonStrikingDummy => {
                                            connection
                                                .handle
                                                .send(ToServer::DebugNewEnemy(
                                                    connection.id,
                                                    connection.player_data.character.actor_id,
                                                    11744, // TODO: this doesn't seem to be right?!
                                                ))
                                                .await;
                                        }
                                        _ => tracing::info!(
                                            "DirectorTrigger: {handler_id} {trigger:?} {arg}"
                                        ),
                                    }
                                }
                                ClientTriggerCommand::OpenGoldSaucerGeneralTab {} => {
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::GoldSaucerInformation { unk: [0; 40] },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                ClientTriggerCommand::OpenTrustWindow {} => {
                                    // We have to send at least one valid trust to the client, otherwise the window never shows.
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::TrustInformation(TrustInformation {
                                            available_content: vec![TrustContent {
                                                trust_content_id: 1, // Holminster Switch
                                                last_selected_characters: [0xFF; 16],
                                            }],
                                            levels: [0; 34],
                                            exp: [0; 34],
                                        }),
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                ClientTriggerCommand::OpenDutySupportWindow {} => {
                                    // We have to send at least one available duty to the client, otherwise it crashes.
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::DutySupportInformation {
                                            available_content: vec![1],
                                        },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                ClientTriggerCommand::OpenPortraitsWindow {} => {
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::PortraitsInformation { unk: [0; 56] },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                ClientTriggerCommand::SetTitle { title_id } => {
                                    connection.player_data.volatile.title = title_id as i32;

                                    // Inform the server, so it sends out the AC.
                                    connection
                                        .handle
                                        .send(ToServer::ClientTrigger(
                                            connection.id,
                                            connection.player_data.character.actor_id,
                                            trigger.clone(),
                                        ))
                                        .await;
                                }
                                ClientTriggerCommand::AbandonContent { .. } => {
                                    // Remove ourselves from this instance.
                                    connection
                                        .handle
                                        .send(ToServer::LeaveContent(
                                            connection.id,
                                            connection.player_data.character.actor_id,
                                            connection.old_zone_id,
                                            connection.old_position,
                                            connection.old_rotation,
                                        ))
                                        .await;
                                }
                                ClientTriggerCommand::PrepareCastGlamour { .. } => {
                                    // The actual glamoruing happens later when the action is complete.
                                    connection.glamour_information = Some(trigger.trigger.clone());
                                }
                                ClientTriggerCommand::PrepareRemoveGlamour { .. } => {
                                    // Ditto.
                                    connection.glamour_information = Some(trigger.trigger.clone());
                                }
                                ClientTriggerCommand::BeginOrEndFishing { end } => {
                                    let handler_id = HandlerId::new(HandlerType::Fishing, 1).0;
                                    if !end {
                                        connection
                                            .start_event(
                                                ObjectTypeId {
                                                    object_id: connection
                                                        .player_data
                                                        .character
                                                        .actor_id,
                                                    object_type: ObjectTypeKind::None,
                                                },
                                                handler_id,
                                                EventType::Fishing,
                                                0,
                                                events,
                                            )
                                            .await;

                                        let event = &events.last().unwrap().1;

                                        // TODO: wrong scene flags
                                        connection
                                            .event_scene(
                                                event,
                                                1,
                                                SceneFlags::NO_DEFAULT_CAMERA,
                                                vec![274, 277, 0],
                                            )
                                            .await;

                                        let ipc = ServerZoneIpcSegment::new(
                                            ServerZoneIpcData::LogMessage {
                                                handler_id: HandlerId(handler_id),
                                                message_type: 1110,
                                                params_count: 1,
                                                item_id: 28,
                                                item_quantity: 0,
                                            },
                                        );
                                        connection.send_ipc_self(ipc).await;

                                        connection
                                            .handle
                                            .send(ToServer::Fish(
                                                connection.id,
                                                connection.player_data.character.actor_id,
                                            ))
                                            .await;
                                    } else {
                                        let event = &events.last().unwrap().1;

                                        connection
                                            .event_scene(
                                                event,
                                                3,
                                                SceneFlags::NO_DEFAULT_CAMERA,
                                                vec![273],
                                            )
                                            .await;
                                    }
                                }
                                ClientTriggerCommand::RequestGatheringPoint { id } => {
                                    let base_id;
                                    let level;
                                    let count;
                                    {
                                        let mut gamedata = connection.gamedata.lock();
                                        (base_id, level, count) = gamedata.get_gathering_point(id);
                                    }

                                    connection
                                        .actor_control_self(
                                            ActorControlCategory::SetupGatheringPoint {
                                                id,
                                                base_id: base_id as u32,
                                                level: level as u32,
                                                count: count as u32,
                                            },
                                        )
                                        .await;
                                }
                                ClientTriggerCommand::BeginCraft { end, id } => {
                                    let handler_id = HandlerId::new(HandlerType::Craft, 1).0;
                                    if !end {
                                        connection
                                            .start_event(
                                                ObjectTypeId {
                                                    object_id: connection
                                                        .player_data
                                                        .character
                                                        .actor_id,
                                                    object_type: ObjectTypeKind::None,
                                                },
                                                handler_id,
                                                EventType::Craft,
                                                0,
                                                events,
                                            )
                                            .await;

                                        let event = &events.last().unwrap().1;

                                        let recipe;
                                        {
                                            let mut gamedata = connection.gamedata.lock();
                                            recipe = gamedata.get_recipe(id);
                                        }

                                        // TODO: wrong scene flags
                                        connection
                                            .event_scene(
                                                event,
                                                0,
                                                SceneFlags::NO_DEFAULT_CAMERA,
                                                vec![1, 0, recipe.item_id as u32, 0],
                                            )
                                            .await;

                                        connection.recipe = Some(recipe);
                                    }
                                }
                                ClientTriggerCommand::RidePillionRequest {
                                    target_actor_id,
                                    target_seat_index,
                                } => {
                                    // TODO: Remove this `let party_id` in an upcoming party refactor, this is temporary
                                    let party_id = if connection.party_id != 0 {
                                        Some(connection.party_id)
                                    } else {
                                        None
                                    };

                                    connection
                                        .handle
                                        .send(ToServer::RidePillionRequest(
                                            connection.player_data.character.actor_id,
                                            party_id,
                                            target_actor_id,
                                            target_seat_index,
                                        ))
                                        .await;
                                }
                                ClientTriggerCommand::ExamineCharacter { .. } => {
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::ExamineCharacterInformation {
                                            unk1: [0; 640],
                                            name: "test".to_string(),
                                            unk2: [0; 272],
                                        },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                ClientTriggerCommand::ToggleNoviceStatus { .. } => {
                                    if connection.player_data.search_info.online_status
                                        != OnlineStatus::NewAdventurer
                                    {
                                        connection.player_data.search_info.online_status =
                                            OnlineStatus::NewAdventurer;
                                    } else {
                                        connection.player_data.search_info.online_status =
                                            OnlineStatus::Online;
                                    }
                                    {
                                        let mut database = connection.database.lock();
                                        database.commit_search_info(&connection.player_data);
                                    }
                                    connection.update_online_status().await;
                                }
                                ClientTriggerCommand::OpenUnk1 { .. } => {
                                    for i in 0..10 {
                                        let ipc = ServerZoneIpcSegment::new(
                                            ServerZoneIpcData::RetainerInfo {
                                                sequence: 1,
                                                unk2: 0,
                                                retainer_id: 1 + i as u64,
                                                index: i,
                                                item_count: 174,
                                                gil: 144492,
                                                unk55: 0,
                                                unk56: 2,
                                                classjob_id: 26,
                                                level: 33,
                                                unk7: 0,
                                                unk8: 0,
                                                unk9: 0,
                                                unk10: 1,
                                                unk11: 0,
                                                name: "Test Retainer".to_string(),
                                            },
                                        );
                                        connection.send_ipc_self(ipc).await;
                                    }

                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::RetainerInfoEnd {
                                            sequence: 1,
                                            unk1: 66826,
                                            unk2: 0,
                                            unk3: 4294967295,
                                            unk4: 4294967295,
                                            unk5: 65535,
                                        },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                _ => {
                                    // inform the server of our trigger, it will handle sending it to other clients
                                    connection
                                        .handle
                                        .send(ToServer::ClientTrigger(
                                            connection.id,
                                            connection.player_data.character.actor_id,
                                            trigger.clone(),
                                        ))
                                        .await;
                                }
                            }
                        }
                        ClientZoneIpcData::UnkSocialEvent { .. } => {
                            connection
                                .send_ipc_self(ServerZoneIpcSegment::new(
                                    ServerZoneIpcData::UnkSocialResponse {
                                        unk: Default::default(),
                                    },
                                ))
                                .await;
                        }
                        ClientZoneIpcData::SocialListRequest(request) => {
                            if request.request_type == SocialListRequestType::Friends {
                                // We have to refresh manually here as the friend list doesn't have a convenient search & result opcode pair like searching does.
                                connection.refresh_friend_list().await;
                            }
                            let entries = if request.request_type == SocialListRequestType::Party {
                                Some(connection.party_member_entries())
                            } else {
                                None
                            };

                            connection
                                .send_social_list(
                                    request.request_type,
                                    request.sequence,
                                    entries,
                                    None,
                                )
                                .await;
                        }
                        ClientZoneIpcData::UpdatePositionHandler {
                            position,
                            rotation,
                            anim_type,
                            anim_state,
                            jump_state,
                        } => {
                            connection.player_data.volatile.rotation = *rotation as f64;
                            connection.player_data.volatile.position = *position;

                            let party_id = if connection.party_id != 0 {
                                Some(connection.party_id)
                            } else {
                                None
                            };

                            connection
                                .handle
                                .send(ToServer::ActorMoved(
                                    connection.player_data.character.actor_id,
                                    *position,
                                    *rotation,
                                    *anim_type,
                                    *anim_state,
                                    *jump_state,
                                    party_id,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::LogOut { .. } => {
                            connection.gracefully_logged_out = true;
                            connection.begin_log_out().await;
                        }
                        ClientZoneIpcData::Disconnected { .. } => {
                            tracing::info!("Client disconnected!");

                            // We no longer send ToServer::Disconnected here because the end of the function already does it unconditionally
                            return false;
                        }
                        ClientZoneIpcData::SendChatMessage(chat_message) => {
                            let info = MessageInfo {
                                sender_actor_id: connection.player_data.character.actor_id,
                                sender_account_id: connection
                                    .player_data
                                    .character
                                    .service_account_id
                                    as u64,
                                sender_world_id: config.world.world_id,
                                sender_position: connection.player_data.volatile.position,
                                sender_name: connection.player_data.character.name.clone(),
                                channel: chat_message.channel,
                                message: chat_message.message.clone(),
                            };

                            connection
                                .handle
                                .send(ToServer::Message(
                                    connection.player_data.character.actor_id,
                                    info,
                                ))
                                .await;

                            let mut handled = false;
                            let command_trigger: char = '!';
                            if chat_message.message.starts_with(command_trigger) {
                                let parts: Vec<&str> = chat_message.message.split(' ').collect();
                                let command_name = &parts[0][1..];

                                {
                                    let lua = connection.lua.lock();
                                    let state = lua.0.app_data_ref::<KawariLuaState>().unwrap();

                                    // If a Lua command exists, try using that first
                                    if let Some(command_script) =
                                        state.command_scripts.get(command_name)
                                    {
                                        handled = true;

                                        let file_name =
                                            FilesystemConfig::locate_script_file(command_script);

                                        let mut run_script = || -> mlua::Result<()> {
                                            lua.0.scope(|scope| {
                                                    let connection_data = scope
                                                    .create_userdata_ref_mut(lua_player)?;

                                                    /* TODO: Instead of panicking we ought to send a message to the player
                                                     * and the console log, and abandon execution. */
                                                    lua.0.load(
                                                        std::fs::read(&file_name).unwrap_or_else(|_| panic!("Failed to load script file {}!", &file_name)),
                                                    )
                                                    .set_name("@".to_string() + &file_name)
                                                    .exec()?;

                                                    let required_rank = lua.0.globals().get("required_rank");
                                                    if let Err(error) = required_rank {
                                                        tracing::info!("Script is missing required_rank! Unable to run command, sending error to user. Additional information: {}", error);
                                                        let func: Function =
                                                        lua.0.globals().get("onCommandRequiredRankMissingError")?;
                                                        func.call::<()>((error.to_string(), connection_data))?;
                                                        return Ok(());
                                                    }

                                                    /* Reset state for future commands. Without this it'll stay set to the last value
                                                     * and allow other commands that omit required_rank to run, which is undesirable. */
                                                    lua.0.globals().set("required_rank", mlua::Value::Nil)?;

                                                    if connection.player_data.character.gm_rank as u8 >= required_rank? {
                                                        let mut func_args = Vec::new();
                                                        if parts.len() > 1 {
                                                            func_args = (parts[1..]).to_vec();
                                                            tracing::info!("Args passed to Lua command {}: {:?}", command_name, func_args);
                                                        } else {
                                                            tracing::info!("No additional args passed to Lua command {}.", command_name);
                                                        }
                                                        let func: Function =
                                                        lua.0.globals().get("onCommand")?;
                                                        func.call::<()>((func_args, connection_data))?;

                                                        /* `command_sender` is an optional variable scripts can define to identify themselves in print messages.
                                                         * It's okay if this global isn't set. We also don't care what its value is, just that it exists.
                                                         * This is reset -after- running the command intentionally. Resetting beforehand will never display the command's identifier.
                                                         */
                                                        let command_sender: Result<mlua::prelude::LuaValue, mlua::prelude::LuaError> = lua.0.globals().get("command_sender");
                                                        if command_sender.is_ok() {
                                                            lua.0.globals().set("command_sender", mlua::Value::Nil)?;
                                                        }
                                                        Ok(())
                                                    } else {
                                                        tracing::info!("User with account_id {} tried to invoke GM command {} with insufficient privileges!",
                                                                       connection.player_data.character.service_account_id, command_name);
                                                        let func: Function =
                                                        lua.0.globals().get("onCommandRequiredRankInsufficientError")?;
                                                        func.call::<()>(connection_data)?;
                                                        Ok(())
                                                    }
                                                })
                                        };

                                        if let Err(err) = run_script() {
                                            tracing::warn!("Lua error in {file_name}: {:?}", err);
                                        }
                                    }
                                }

                                // Fallback to Rust implemented commands
                                if !handled {
                                    handled = ChatHandler::handle_chat_message(
                                        connection,
                                        chat_message,
                                        events,
                                    )
                                    .await;
                                }

                                // If it's truly not existent:
                                if !handled {
                                    tracing::info!("Unknown command {command_name}");

                                    let lua = connection.lua.lock();

                                    let mut call_func = || {
                                        lua.0.scope(|scope| {
                                            let connection_data =
                                                scope.create_userdata_ref_mut(lua_player)?;
                                            let func: Function =
                                                lua.0.globals().get("onUnknownCommandError")?;
                                            func.call::<()>((command_name, connection_data))?;
                                            Ok(())
                                        })
                                    };

                                    if let Err(err) = call_func() {
                                        tracing::warn!("Lua error in Global.lua: {:?}", err);
                                    }
                                }
                            }
                        }
                        ClientZoneIpcData::GMCommand {
                            command,
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            ..
                        } => {
                            connection
                                .run_gm_command(*command, *arg0, *arg1, *arg2, *arg3, lua_player)
                                .await;
                        }
                        ClientZoneIpcData::GMCommandName {
                            command,
                            arg0,
                            arg1,
                            arg2,
                            arg3,
                            ..
                        } => {
                            connection
                                .run_gm_command(*command, *arg0, *arg1, *arg2, *arg3, lua_player)
                                .await;
                        }
                        ClientZoneIpcData::GMCommandName2 { data, .. } => {
                            tracing::info!("GM Command Test: {data}");
                        }
                        ClientZoneIpcData::ZoneJump {
                            exit_box, position, ..
                        } => {
                            tracing::info!(
                                "Character entered {exit_box} with a position of {position:#?}!"
                            );

                            connection
                                .handle
                                .send(ToServer::EnterZoneJump(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    *exit_box,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::ActionRequest(request) => {
                            connection
                                .handle
                                .send(ToServer::ActionRequest(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    request.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::PingSync { timestamp, .. } => {
                            // this is *usually* sent in response, but not always
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PingSyncReply {
                                timestamp: *timestamp,      // copied from here
                                transmission_interval: 333, // always this for some reason
                            });
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::ItemOperation(action) => {
                            tracing::info!("Client is modifying inventory! {action:#?}");
                            connection
                                .send_inventory_ack(
                                    action.context_id,
                                    INVENTORY_ACTION_ACK_GENERAL as u16,
                                )
                                .await;

                            connection.player_data.inventory.process_action(action);

                            if action.operation_type == ItemOperationKind::Discard {
                                tracing::info!("Client is discarding from their inventory!");

                                let ipc = ServerZoneIpcSegment::new(
                                    ServerZoneIpcData::InventoryTransaction {
                                        sequence: connection.player_data.item_sequence,
                                        operation_type: action.operation_type,
                                        src_actor_id: connection.player_data.character.actor_id,
                                        src_storage_id: action.src_storage_id,
                                        src_container_index: action.src_container_index,
                                        src_stack: action.src_stack,
                                        src_catalog_id: action.src_catalog_id,
                                        dummy_container: ContainerType::DiscardingItemSentinel,
                                        dst_storage_id: ContainerType::DiscardingItemSentinel,
                                        dst_container_index: u16::MAX,
                                        dst_actor_id: ObjectId::default(),
                                        dst_stack: 0,
                                        dst_catalog_id: 0,
                                    },
                                );
                                connection.send_ipc_self(ipc).await;
                                connection
                                    .send_inventory_transaction_finish(0x90, 0x200)
                                    .await;
                            }

                            connection.player_data.item_sequence += 1;

                            // This is annoying, basically if we change weapons the client expects the *server* to also equip the relevant job crystal if available.
                            // The client DOES not send us an ItemOperation for this, but in turn we don't have to inform them about the update.
                            if (action.src_storage_id == ContainerType::Equipped
                                && action.src_container_index == EquipSlot::MainHand as u16)
                                || (action.dst_storage_id == ContainerType::Equipped
                                    && action.dst_container_index == EquipSlot::MainHand as u16)
                            {
                                tracing::info!("Changing class based on weapon...");

                                // We need to update our current class based on the weapon...
                                connection.change_class_based_on_weapon().await;

                                tracing::info!(
                                    "- Result: {}",
                                    connection.player_data.classjob.current_class
                                );

                                let id;
                                {
                                    let mut gamedata = connection.gamedata.lock();
                                    id = gamedata.get_soul_crystal_item_id(
                                        connection.player_data.classjob.current_class as u16,
                                    );
                                }
                                if let Some(id) = id
                                    && connection.player_data.inventory.has_soul_crystal(id)
                                {
                                    tracing::info!(
                                        "*We* are equipping soul crystal {id} based on client behavior..."
                                    );
                                    connection.player_data.inventory.equip_soul_crystal(id);

                                    // Then re-check the soul crystal...
                                    connection.change_class_based_on_soul_crystal().await;
                                } else {
                                    tracing::info!(
                                        "*We* are unequipping the soul crystal based on client behavior..."
                                    );
                                    connection
                                        .player_data
                                        .inventory
                                        .unequip_equipment(EquipSlot::SoulCrystal as u16);
                                }
                            }

                            // If the soul crystal is changed, ensure we update accordingly.
                            if (action.src_storage_id == ContainerType::Equipped
                                && action.src_container_index == EquipSlot::SoulCrystal as u16)
                                || (action.dst_storage_id == ContainerType::Equipped
                                    && action.dst_container_index == EquipSlot::SoulCrystal as u16)
                            {
                                let soul_crystal =
                                    connection.player_data.inventory.equipped.soul_crystal;
                                if soul_crystal.quantity > 0 {
                                    tracing::info!(
                                        "We are re-checking class based on the soul crystal..."
                                    );
                                    connection.change_class_based_on_soul_crystal().await;
                                } else {
                                    tracing::info!(
                                        "We are re-checking class based on the weapon..."
                                    );
                                    connection.change_class_based_on_weapon().await;
                                }
                            }

                            // This is also something done client-side.
                            connection.remove_incompatible_armor(action).await;

                            // If the client modified their equipped items, we have to process that
                            if action.src_storage_id == ContainerType::Equipped
                                || action.dst_storage_id == ContainerType::Equipped
                            {
                                connection.inform_equip().await;
                                connection.send_stats().await; // Because stats changed based on equipped items
                            }
                        }
                        ClientZoneIpcData::StartTalkEvent {
                            actor_id,
                            handler_id,
                        } => {
                            if connection
                                .start_event(*actor_id, handler_id.0, EventType::Talk, 0, events)
                                .await
                            {
                                // begin talk function if it exists
                                if let Some(event) = events.last_mut() {
                                    event.0.on_talk(&event.1, *actor_id, lua_player).await;
                                }
                            }
                        }
                        ClientZoneIpcData::EventReturnHandler2(handler) => {
                            // TODO: merge all implementations
                            tracing::info!(message = "Event returned", handler_id = %handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            if let Some(event) = events.last() {
                                event
                                    .0
                                    .on_return(
                                        &event.1,
                                        connection,
                                        handler.scene,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    )
                                    .await;
                            } else {
                                tracing::warn!("There's no current event to return from!");
                            }
                        }
                        ClientZoneIpcData::EventReturnHandler8(handler) => {
                            tracing::info!(message = "Event returned", handler_id = %handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            if let Some(event) = events.last() {
                                event
                                    .0
                                    .on_return(
                                        &event.1,
                                        connection,
                                        handler.scene,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    )
                                    .await;
                            } else {
                                tracing::warn!("There's no current event to return from!");
                            }
                        }
                        ClientZoneIpcData::EventYieldHandler2(handler) => {
                            tracing::info!(message = "Event yielded", handler_id = %handler.handler_id, yield_id = handler.yield_id, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            if let Some(event) = events.last() {
                                event
                                    .0
                                    .on_yield(
                                        &event.1,
                                        connection,
                                        handler.scene,
                                        handler.yield_id,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    )
                                    .await;
                            } else {
                                tracing::warn!("There's no current event to yield from!");
                            }
                        }
                        ClientZoneIpcData::EventYieldHandler4(handler) => {
                            tracing::info!(message = "Event yielded", handler_id = %handler.handler_id, yield_id = handler.yield_id, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            if let Some(event) = events.last() {
                                event
                                    .0
                                    .on_yield(
                                        &event.1,
                                        connection,
                                        handler.scene,
                                        handler.yield_id,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    )
                                    .await;
                            } else {
                                tracing::warn!("There's no current event to yield from!");
                            }
                        }
                        ClientZoneIpcData::EventYieldHandler16(handler) => {
                            tracing::info!(message = "Event yielded", handler_id = %handler.handler_id, yield_id = handler.yield_id, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            if let Some(event) = events.last() {
                                event
                                    .0
                                    .on_yield(
                                        &event.1,
                                        connection,
                                        handler.scene,
                                        handler.yield_id,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    )
                                    .await;
                            } else {
                                tracing::warn!("There's no current event to yield from!");
                            }
                        }
                        ClientZoneIpcData::EventYieldHandler128(handler) => {
                            tracing::info!(message = "Event yielded", handler_id = %handler.handler_id, yield_id = handler.yield_id, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            if let Some(event) = events.last() {
                                event
                                    .0
                                    .on_yield(
                                        &event.1,
                                        connection,
                                        handler.scene,
                                        handler.yield_id,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    )
                                    .await;
                            } else {
                                tracing::warn!("There's no current event to yield from!");
                            }
                        }
                        ClientZoneIpcData::Config(config) => {
                            // Update our own state so it's committed on log out
                            connection.player_data.volatile.display_flags = config.display_flag;
                            connection
                                .handle
                                .send(ToServer::Config(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    config.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::StandardControlsPivot { .. } => {
                            /* No-op because we already seem to handle this, other nearby clients can see the sending player
                             * pivoting anyway. */
                        }
                        ClientZoneIpcData::UnkCall2 { .. } => {
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UnkResponse2 {
                                unk1: 1,
                            });
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::QueueDuties(queue_duties) => {
                            connection.content_settings = Some(queue_duties.flags);
                            lua_player.content_data.settings =
                                DutyOption::from_content_flags(queue_duties.flags).bits(); // TODO: is this the best place to update this?
                            connection
                                .register_for_content(queue_duties.content_ids)
                                .await;
                        }
                        ClientZoneIpcData::QueueRoulette { .. } => {
                            tracing::warn!("Queueing for roulettes is not implemented!");
                        }
                        ClientZoneIpcData::ContentFinderAction { action, .. } => {
                            if *action == ContentFinderUserAction::Accepted {
                                // commencing
                                {
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::ContentFinderCommencing {
                                            unk1: [
                                                4, 0, 0, 0, 1, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0,
                                                0, 1, 1, 0, 0, 0, 0,
                                            ],
                                        },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }

                                connection
                                    .join_content(connection.queued_content.unwrap())
                                    .await;
                            }

                            // If we don't send this, the content finder gets stuck.
                            // TODO: this may be screwing up the in-duty menu, probably need to fill it with data!
                            let ipc =
                                ServerZoneIpcSegment::new(ServerZoneIpcData::UnkContentFinder {
                                    unk: [0; 16],
                                });
                            connection.send_ipc_self(ipc).await;

                            connection.queued_content = None;
                        }
                        ClientZoneIpcData::EquipGearset {
                            gearset_index,
                            containers,
                            indices,
                            ..
                        } => {
                            // TODO: handle missing items, full inventory and such
                            for slot in 0..14 {
                                let from_slot = indices[slot];
                                let from_container = containers[slot];

                                if from_container == ContainerType::Equipped {
                                    continue;
                                }

                                let from_item = if from_slot != -1 {
                                    connection
                                        .player_data
                                        .inventory
                                        .get_item(from_container, from_slot as u16)
                                } else {
                                    Item::default()
                                };
                                let equipped_item = connection
                                    .player_data
                                    .inventory
                                    .equipped
                                    .get_slot(slot as u16);

                                if !from_item.is_empty_slot() && !equipped_item.is_empty_slot() {
                                    // If there is something equipped and a replacement for it, we must swap.
                                    connection
                                        .swap_items(
                                            from_container,
                                            from_slot as u16,
                                            ContainerType::Equipped,
                                            slot as u16,
                                        )
                                        .await;
                                } else if !from_item.is_empty_slot()
                                    && equipped_item.is_empty_slot()
                                {
                                    // If there is nothing equipped but a new item in that slot, we just have to move it.
                                    // TODO: be a little smarter about this maybe?
                                    connection
                                        .swap_items(
                                            from_container,
                                            from_slot as u16,
                                            ContainerType::Equipped,
                                            slot as u16,
                                        )
                                        .await;
                                } else if from_item.is_empty_slot()
                                    && !equipped_item.is_empty_slot()
                                {
                                    // If there is something equipped but the slot is empty in the gearset, we have to move it somewhere.
                                    let target_container_type =
                                        ContainerType::from_equip_slot(slot as u8);

                                    let target_container = connection
                                        .player_data
                                        .inventory
                                        .get_container(target_container_type);
                                    if let Some(free_slot) = get_next_free_slot(target_container) {
                                        connection
                                            .swap_items(
                                                ContainerType::Equipped,
                                                slot as u16,
                                                target_container_type,
                                                free_slot,
                                            )
                                            .await;
                                    }
                                }
                            }

                            // Inform the client that the gearset was successfully equipped.
                            connection
                                .actor_control_self(ActorControlCategory::GearSetEquipped {
                                    gearset_index: *gearset_index,
                                })
                                .await;

                            // And that we're done modifying the inventory.
                            connection
                                .send_inventory_transaction_finish(567, 3584)
                                .await;

                            // Retail also re-sends the equipped container
                            connection.send_equipped_inventory().await;
                            connection.inform_equip().await;

                            // Change class as needed.
                            connection.change_class_based_on_weapon().await;

                            // Then finally, resend stats.
                            connection.send_stats().await;
                        }
                        ClientZoneIpcData::EquipGearset2 { .. } => {
                            tracing::warn!("Bigger gearsets not supported yet!");
                        }
                        ClientZoneIpcData::StartWalkInEvent {
                            event_arg,
                            handler_id,
                            ..
                        } => {
                            // Yes, an ActorControl is sent here, not an ActorControlSelf!
                            connection
                                .actor_control(
                                    connection.player_data.character.actor_id,
                                    ActorControlCategory::ToggleWeapon {
                                        shown: false,
                                        unk_flag: 1,
                                    },
                                )
                                .await;

                            let actor_id = ObjectTypeId {
                                object_id: connection.player_data.character.actor_id,
                                object_type: ObjectTypeKind::None,
                            };
                            connection
                                .start_event(
                                    actor_id,
                                    handler_id.0,
                                    EventType::WithinRange,
                                    *event_arg,
                                    events,
                                )
                                .await;

                            // begin walk-in trigger function if it exists
                            if let Some(event) = events.last_mut() {
                                event
                                    .0
                                    .on_enter_trigger(&event.1, lua_player, *event_arg)
                                    .await;
                            }
                        }
                        ClientZoneIpcData::WalkOutsideEvent {
                            event_arg,
                            handler_id,
                            ..
                        } => {
                            // TODO: allow Lua scripts to handle these differently?

                            // Yes, an ActorControl is sent here, not an ActorControlSelf!
                            connection
                                .actor_control(
                                    connection.player_data.character.actor_id,
                                    ActorControlCategory::ToggleWeapon {
                                        shown: false,
                                        unk_flag: 1,
                                    },
                                )
                                .await;

                            let actor_id = ObjectTypeId {
                                object_id: connection.player_data.character.actor_id,
                                object_type: ObjectTypeKind::None,
                            };
                            connection
                                .start_event(
                                    actor_id,
                                    handler_id.0,
                                    EventType::OutsideRange,
                                    *event_arg,
                                    events,
                                )
                                .await;

                            // begin walk-in trigger function if it exists
                            if let Some(event) = events.last_mut() {
                                event
                                    .0
                                    .on_enter_trigger(&event.1, lua_player, *event_arg)
                                    .await;
                            }
                        }
                        ClientZoneIpcData::NewDiscovery { layout_id, pos } => {
                            tracing::info!(
                                "Client discovered a new location on {:?} at {:?}!",
                                layout_id,
                                pos
                            );

                            connection
                                .handle
                                .send(ToServer::NewLocationDiscovered(
                                    connection.id,
                                    *layout_id,
                                    *pos,
                                    connection.player_data.volatile.zone_id as u16,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RequestBlacklist(request) => {
                            // TODO: Actually implement this beyond simply sending a blank list
                            // NOTE: Failing to respond to this request means SpawnPlayer will not work and other players will be invisible, have their chat ignored and possibly other issues by the client! Beware!
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Blacklist(
                                Blacklist {
                                    data: vec![
                                        BlacklistedCharacter::default();
                                        Blacklist::NUM_ENTRIES
                                    ],
                                    sequence: request.sequence,
                                },
                            ));
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::RequestFellowships { .. } => {
                            tracing::info!("Fellowships is unimplemented");
                        }
                        ClientZoneIpcData::RequestCrossworldLinkshells { .. } => {
                            connection.send_crossworld_linkshells(true).await;
                        }
                        ClientZoneIpcData::SearchFellowships { .. } => {
                            tracing::info!("Fellowships is unimplemented");
                        }
                        ClientZoneIpcData::StartCountdown {
                            starter_actor_id,
                            duration,
                            starter_name,
                        } => {
                            connection
                                .handle
                                .send(ToServer::StartCountdown(
                                    connection.party_id,
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    starter_name.clone(),
                                    *starter_actor_id,
                                    *duration,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RequestPlaytime { .. } => {
                            connection.send_playtime().await;
                        }
                        ClientZoneIpcData::SetFreeCompanyGreeting { .. } => {
                            tracing::info!("Setting the free company greeting is unimplemented");
                        }
                        ClientZoneIpcData::SetClientLanguage { language } => {
                            connection.player_data.volatile.client_language = *language;
                        }
                        ClientZoneIpcData::RequestCharaInfoFromContentIds { .. } => {
                            tracing::info!(
                                "Requesting character info from content ids is unimplemented"
                            );
                        }
                        ClientZoneIpcData::InviteCharacter {
                            content_id,
                            world_id,
                            invite_type,
                            character_name,
                        } => {
                            tracing::info!(
                                "Client invited a character! {:#?} {:#?} {:#?} {:#?} {:#?}",
                                content_id,
                                world_id,
                                invite_type,
                                character_name,
                                data.data
                            );
                            match invite_type {
                                InviteType::Party => {
                                    connection
                                        .handle
                                        .send(ToServer::InvitePlayerToParty(
                                            connection.player_data.character.actor_id,
                                            *content_id,
                                            character_name.clone(),
                                        ))
                                        .await;
                                }
                                InviteType::FriendList => {
                                    connection.add_to_friend_list(*content_id);

                                    connection
                                        .handle
                                        .send(ToServer::InvitePlayerToFriendList(
                                            connection.player_data.character.actor_id,
                                            *content_id,
                                            character_name.clone(),
                                        ))
                                        .await
                                }
                                _ => todo!(),
                            }
                        }
                        ClientZoneIpcData::InviteReply {
                            sender_content_id,
                            sender_world_id,
                            invite_type,
                            response,
                        } => {
                            tracing::info!(
                                "Client replied to invite: {:#?} {:#?} {:#?} {:#?}",
                                sender_content_id,
                                sender_world_id,
                                invite_type,
                                response
                            );
                            connection
                                .handle
                                .send(ToServer::InvitationResponse(
                                    connection.id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                    *sender_content_id,
                                    *invite_type,
                                    *response,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::InviteReply2 {
                            sender_content_id,
                            sender_world_id,
                            response,
                            invite_type,
                            character_name,
                            ..
                        } => {
                            tracing::info!(
                                "Client replied to friend invite: {:#?} {:#?} {:#?} {:#?}",
                                sender_content_id,
                                sender_world_id,
                                invite_type,
                                response
                            );
                            // TODO: all of these are sort of wrong?
                            connection
                                .handle
                                .send(ToServer::InvitationResponse(
                                    connection.id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    character_name.clone(),
                                    *sender_content_id,
                                    *invite_type,
                                    *response,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::PartyDisband { .. } => {
                            tracing::info!("Client is disbanding their party!");
                            connection
                                .handle
                                .send(ToServer::PartyDisband(
                                    connection.party_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::PartyMemberKick {
                            content_id,
                            character_name,
                            ..
                        } => {
                            tracing::info!(
                                "Player is kicking another player from their party! {} {}",
                                content_id,
                                character_name
                            );
                            connection
                                .handle
                                .send(ToServer::PartyMemberKick(
                                    connection.party_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                    *content_id,
                                    character_name.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::PartyChangeLeader {
                            content_id,
                            character_name,
                            ..
                        } => {
                            tracing::info!(
                                "Player is promoting another player in their party to leader! {} {}",
                                content_id,
                                character_name
                            );
                            connection
                                .handle
                                .send(ToServer::PartyChangeLeader(
                                    connection.party_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                    *content_id,
                                    character_name.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::PartyLeave { .. } => {
                            tracing::info!("Client is leaving their party!");
                            connection
                                .handle
                                .send(ToServer::PartyMemberLeft(
                                    connection.party_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.name.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RequestSearchInfo { content_id, .. } => {
                            let search_info;
                            {
                                let mut database = connection.database.lock();
                                let mut game_data = connection.gamedata.lock();
                                search_info =
                                    database.get_search_info(&mut game_data, *content_id as i64);
                            }

                            let ipc = ServerZoneIpcSegment::new(search_info);
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::RequestAdventurerPlate { .. } => {
                            tracing::info!("Requesting adventurer plates is unimplemented");
                        }
                        ClientZoneIpcData::SearchPlayers {
                            classjobs,
                            minimum_level,
                            maximum_level,
                            grand_company,
                            languages,
                            online_status,
                            areas,
                            name,
                            ..
                        } => {
                            connection
                                .search_players(
                                    *classjobs,
                                    *minimum_level,
                                    *maximum_level,
                                    *grand_company,
                                    *languages,
                                    *online_status,
                                    *areas,
                                    name.clone(),
                                )
                                .await;
                        }
                        ClientZoneIpcData::EditSearchInfo(search_info) => {
                            connection.player_data.search_info.online_status = search_info
                                .online_status
                                .mask()
                                .last()
                                .copied()
                                .unwrap_or(OnlineStatus::Online); // TODO: unsure if this makes sense?
                            connection.player_data.search_info.comment =
                                search_info.comment.clone();
                            connection.player_data.search_info.selected_languages =
                                search_info.selected_languages;
                            {
                                let mut database = connection.database.lock();
                                database.commit_search_info(&connection.player_data);
                            }
                            connection.update_online_status().await;
                        }
                        ClientZoneIpcData::RequestOwnSearchInfo { .. } => {
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SetSearchInfo(
                                SearchInfo {
                                    online_status: OnlineStatusMask::from_online_status(
                                        connection.player_data.search_info.online_status,
                                    ),
                                    comment: connection.player_data.search_info.comment.clone(),
                                    selected_languages: connection
                                        .player_data
                                        .search_info
                                        .selected_languages,
                                    ..Default::default()
                                },
                            ));
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::EnterTerritoryEvent { handler_id } => {
                            connection
                                .start_event(
                                    ObjectTypeId {
                                        object_id: connection.player_data.character.actor_id,
                                        object_type: ObjectTypeKind::None,
                                    },
                                    handler_id.0,
                                    EventType::EnterTerritory,
                                    connection.player_data.volatile.zone_id as u32,
                                    events,
                                )
                                .await;
                            if let Some(event) = events.last_mut() {
                                event.0.on_enter_territory(&event.1, lua_player).await;
                            }
                        }
                        ClientZoneIpcData::Trade { .. } => {
                            tracing::info!("Trading is unimplemented");
                        }
                        ClientZoneIpcData::ShareStrategyBoard {
                            content_id,
                            board_data,
                        } => {
                            tracing::info!(
                                "{} is sharing a strategy board with their party!",
                                connection.player_data.character.actor_id
                            );
                            connection
                                .handle
                                .send(ToServer::ShareStrategyBoard(
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.content_id as u64,
                                    connection.party_id,
                                    *content_id,
                                    board_data.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::StrategyBoardReceived { content_id, .. } => {
                            tracing::info!(
                                "{} has received a strategy board from another player in their party!",
                                connection.player_data.character.actor_id
                            );
                            connection
                                .handle
                                .send(ToServer::StrategyBoardReceived(
                                    connection.party_id,
                                    connection.player_data.character.content_id as u64,
                                    *content_id,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::StrategyBoardUpdate(update_data) => {
                            // No logging here due to how spammy it is since it sends an update every frame or so while the object is moving.
                            connection
                                .handle
                                .send(ToServer::StrategyBoardRealtimeUpdate(
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.content_id as u64,
                                    connection.party_id,
                                    update_data.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RealtimeStrategyBoardFinished { .. } => {
                            tracing::info!(
                                "{} is finished sharing their strategy board in realtime!",
                                connection.player_data.character.actor_id
                            );
                            connection
                                .handle
                                .send(ToServer::StrategyBoardRealtimeFinished(connection.party_id))
                                .await;
                        }
                        ClientZoneIpcData::ApplyFieldMarkerPreset(waymark_preset) => {
                            connection
                                .handle
                                .send(ToServer::ApplyWaymarkPreset(
                                    connection.player_data.character.actor_id,
                                    connection.party_id,
                                    *waymark_preset,
                                    connection.player_data.volatile.zone_id,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RequestFreeCompanyShortMessage { .. } => {
                            tracing::warn!(
                                "Requesting a free company short message is unimplemented"
                            );
                        }
                        ClientZoneIpcData::InitiateReadyCheck { .. } => {
                            // TODO: Remove this `let party_id` in an upcoming party refactor, this is temporary
                            let party_id = if connection.party_id != 0 {
                                Some(connection.party_id)
                            } else {
                                None
                            };

                            connection
                                .handle
                                .send(ToServer::ReadyCheckInitiated(
                                    party_id,
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::ReadyCheckResponse { response } => {
                            // TODO: Remove this `let party_id` in an upcoming party refactor, this is temporary
                            let party_id = if connection.party_id != 0 {
                                Some(connection.party_id)
                            } else {
                                None
                            };

                            // As usual, another client value that doesn't match up with values we respond with...
                            let response = match response {
                                1 => ReadyCheckReply::Yes,
                                _ => ReadyCheckReply::No, // This opcode should only ever give us 1 or 0, but it doesn't hurt to guard against bad inputs.
                            };

                            tracing::info!(
                                "client {} replied to ready check with {response:#?}",
                                connection.player_data.character.actor_id
                            );

                            connection
                                .handle
                                .send(ToServer::ReadyCheckResponse(
                                    party_id,
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.service_account_id as u64,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                    response,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RequestMarketBoardItems { sequence, .. } => {
                            // TODO: placeholder, of course
                            let ipc =
                                ServerZoneIpcSegment::new(ServerZoneIpcData::MarketBoardItems {
                                    sequence: *sequence,
                                    items: vec![
                                        MarketBoardItem {
                                            item_id: 1659,
                                            count: 3,
                                        },
                                        MarketBoardItem {
                                            item_id: 1649,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1621,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 31542,
                                            count: 3,
                                        },
                                        MarketBoardItem {
                                            item_id: 1657,
                                            count: 3,
                                        },
                                        MarketBoardItem {
                                            item_id: 1642,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1613,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1650,
                                            count: 2,
                                        },
                                        MarketBoardItem {
                                            item_id: 1648,
                                            count: 1,
                                        },
                                        MarketBoardItem {
                                            item_id: 1643,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1616,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1639,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1635,
                                            count: 1,
                                        },
                                        MarketBoardItem {
                                            item_id: 1606,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1637,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1633,
                                            count: 1,
                                        },
                                        MarketBoardItem {
                                            item_id: 1627,
                                            count: 2,
                                        },
                                        MarketBoardItem {
                                            item_id: 1625,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1622,
                                            count: 0,
                                        },
                                        MarketBoardItem {
                                            item_id: 1614,
                                            count: 1,
                                        },
                                        MarketBoardItem {
                                            item_id: 92,
                                            count: 0,
                                        },
                                    ],
                                });
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::SetFriendGroupIcon { .. } => {
                            tracing::warn!("Setting friend group icons is unimplemented");
                        }
                        ClientZoneIpcData::CreateLocalLinkshellRequest { .. } => {
                            tracing::warn!("Creating local linkshells is unimplemented");
                        }
                        ClientZoneIpcData::CrossworldLinkshellMemberListRequest {
                            linkshell_id,
                            sequence,
                        } => {
                            connection
                                .send_cwlinkshell_members(*linkshell_id, *sequence)
                                .await;
                        }
                        ClientZoneIpcData::OpenTreasure { .. } => {
                            tracing::warn!("Opening treasure chests is unimplemented");
                        }
                        ClientZoneIpcData::CrossRealmListingsRequest1 { max_results, .. } => {
                            let results_aligned = max_results.div_ceil(4) * 4; // each packet holds 4
                            let required_packets = results_aligned / 4;
                            for i in 0..required_packets {
                                let ipc = ServerZoneIpcSegment::new(
                                    ServerZoneIpcData::CrossRealmListings(CrossRealmListings {
                                        unk10: 0,
                                        unk11: 0xFFFFFFFF,
                                        unk12: 0xFFFFFFFF,
                                        segment_index: if i + 1 == required_packets {
                                            0
                                        } else {
                                            i + 1
                                        },
                                        entries: vec![
                                            CrossRealmListing {
                                                listing_id: 1,
                                                account_id: 1,
                                                content_id: 1,
                                                category: 1,
                                                duty: 1,
                                                duty_type: 1,
                                                world_id: 1,
                                                objective: 1,
                                                beginners_welcome: 1,
                                                duty_finder_settings: 1,
                                                loot_rule: 1,
                                                last_patch_hotfix_timestamp: 1,
                                                time_left: 1,
                                                avg_item_lv: 1,
                                                home_world_id: 1,
                                                client_language: 1,
                                                total_slots: 1,
                                                slots_filled: 1,
                                                join_condition_flags: 1,
                                                is_alliance: 1,
                                                number_of_parties: 1,
                                                slot_flags: [1; 8],
                                                jobs_present: [1; 8],
                                                bad_padding: Vec::new(),
                                                recruiter_name: "Test Listing".to_string(),
                                                comment: "Riduculous Ties".to_string(),
                                                bad_padding2: Vec::new(),
                                            };
                                            4
                                        ],
                                    }),
                                );
                                connection.send_ipc_self(ipc).await;
                            }

                            // send overview
                            let ipc = ServerZoneIpcSegment::new(
                                ServerZoneIpcData::CrossRealmListingsOverview { unk: [0; 48] },
                            );
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::CrossRealmListingsRequest2 { .. } => {
                            // NOTE: not sure what we need from here
                        }
                        ClientZoneIpcData::ViewCrossRealmListing { listing_id } => {
                            let ipc = ServerZoneIpcSegment::new(
                                ServerZoneIpcData::CrossRealmListingInformation {
                                    listing_id: *listing_id,
                                    unk: [0; 456],
                                },
                            );
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::CheckCWLinkshellNameAvailability { name, .. } => {
                            connection
                                .check_cwlinkshell_name_availability(name.clone())
                                .await;
                        }
                        ClientZoneIpcData::CreateNewCrossworldLinkshell { name } => {
                            connection.create_crossworld_linkshell(name.clone()).await;
                        }
                        ClientZoneIpcData::LeaveCrossworldLinkshell { linkshell_id } => {
                            connection
                                .handle
                                .send(ToServer::LeaveLinkshell(
                                    connection.player_data.character.actor_id,
                                    connection.player_data.character.content_id as u64,
                                    connection.player_data.character.name.clone(),
                                    *linkshell_id,
                                ))
                                .await;
                        }
                        ClientZoneIpcData::DisbandCrossworldLinkshell { linkshell_id } => {
                            connection.disband_linkshell(*linkshell_id).await;
                        }
                        ClientZoneIpcData::Unknown { unk } => {
                            tracing::warn!(
                                "Unknown Zone packet {:?} recieved ({} bytes), this should be handled!",
                                data.header.op_code,
                                unk.len()
                            );
                        }
                    }
                }
                SegmentData::KeepAliveRequest { id, timestamp } => {
                    connection.send_keep_alive(*id, *timestamp).await
                }
                SegmentData::KeepAliveResponse { .. } => {
                    // these should be safe to ignore
                }
                _ => {
                    panic!(
                        "ZoneConnection: The server is recieving a response or unknown packet: {segment:#?}"
                    )
                }
            }
        }

        // Process any queued packets from scripts and whatnot
        lua_player.queued_tasks.append(&mut connection.queued_tasks);
        connection.process_lua_player(lua_player, events).await;

        // update lua player
        lua_player.player_data = connection.player_data.clone();
    }

    true
}

/// Process internal server messages.
async fn process_server_msg(
    connection: &mut ZoneConnection,
    lua_player: &mut LuaPlayer,
    events: &mut Vec<(Box<dyn EventHandler>, Event)>,
    client_handle: ClientHandle,
    msg: Option<FromServer>,
) {
    if let Some(msg) = msg {
        match msg {
            FromServer::Message(msg) => connection.send_message(msg).await,
            FromServer::ActorSpawn(actor, spawn) => connection.spawn_actor(actor, spawn).await,
            FromServer::ActorMove(
                actor_id,
                position,
                rotation,
                anim_type,
                anim_state,
                jump_state,
            ) => {
                connection
                    .set_actor_position(
                        actor_id, position, rotation, anim_type, anim_state, jump_state,
                    )
                    .await
            }
            FromServer::DeleteActor(object_id, spawn_index) => {
                connection.delete_actor(object_id, spawn_index).await
            }
            FromServer::DeleteObject(spawn_index) => connection.delete_object(spawn_index).await,
            FromServer::ActorControl(actor_id, actor_control) => {
                connection.actor_control(actor_id, actor_control).await
            }
            FromServer::ActorControlTarget(actor_id, target, actor_control) => {
                connection
                    .actor_control_target(actor_id, target, actor_control)
                    .await
            }
            FromServer::ActorControlSelf(actor_control) => {
                connection.actor_control_self(actor_control).await
            }
            FromServer::ActorSummonsMinion(minion_id) => {
                connection
                    .handle
                    .send(ToServer::ActorSummonsMinion(
                        connection.player_data.character.actor_id,
                        minion_id,
                    ))
                    .await;
                connection.active_minion = minion_id;
            }
            FromServer::ActorDespawnsMinion() => {
                connection
                    .handle
                    .send(ToServer::ActorDespawnsMinion(
                        connection.player_data.character.actor_id,
                    ))
                    .await;
                connection.active_minion = 0;
            }
            FromServer::UpdateConfig(actor_id, config) => {
                connection.update_config(actor_id, config).await
            }
            FromServer::ActorEquip(actor_id, main_weapon_id, sub_weapon_id, model_ids) => {
                connection
                    .update_equip(actor_id, main_weapon_id, sub_weapon_id, model_ids)
                    .await
            }
            FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id) => {
                connection
                    .lose_effect(effect_id, effect_param, effect_source_actor_id)
                    .await
            }
            FromServer::Conditions(conditions) => {
                connection.conditions = conditions;
                connection.send_conditions().await;
            }
            FromServer::ChangeZone(
                zone_id,
                content_finder_condition_id,
                weather_id,
                position,
                rotation,
                lua_zone,
                initial_login,
                director_vars,
            ) => {
                connection
                    .handle_zone_change(
                        zone_id,
                        content_finder_condition_id,
                        weather_id,
                        position,
                        rotation,
                        initial_login,
                        director_vars,
                        &lua_zone,
                        &mut lua_player.content_data,
                    )
                    .await;
                lua_player.zone_data = lua_zone;
            }
            FromServer::NewPosition(position, rotation, fade_out) => {
                connection
                    .set_player_position(position, rotation, fade_out)
                    .await
            }
            FromServer::PartyInvite(sender_account_id, sender_content_id, sender_name) => {
                connection
                    .received_party_invite(sender_account_id, sender_content_id, sender_name)
                    .await
            }
            FromServer::InvitationResult(
                sender_account_id,
                sender_content_id,
                sender_name,
                invite_type,
                invite_reply,
            ) => {
                connection
                    .received_invitation_response(
                        sender_account_id,
                        sender_content_id,
                        sender_name,
                        invite_type,
                        invite_reply,
                    )
                    .await
            }
            FromServer::InvitationReplyResult(
                sender_account_id,
                sender_name,
                invite_type,
                invite_reply,
            ) => {
                connection
                    .send_invite_reply_result(
                        sender_account_id,
                        sender_name,
                        invite_type,
                        invite_reply,
                    )
                    .await
            }
            FromServer::PartyUpdate(targets, update_status, party_info) => {
                connection
                    .send_party_update(targets, update_status, party_info)
                    .await
            }
            FromServer::InviteCharacterResult(
                content_id,
                message_id,
                world_id,
                invite_type,
                character_name,
            ) => {
                connection
                    .invite_character_result(
                        content_id,
                        message_id,
                        world_id,
                        invite_type,
                        character_name.clone(),
                    )
                    .await
            }
            FromServer::RejoinPartyAfterDisconnect(party_id) => {
                connection.party_id = party_id;
                connection.rejoining_party = true;
            }
            FromServer::PacketSegment(ipc, from_actor_id) => {
                connection.send_ipc_from(from_actor_id, ipc).await;
            }
            FromServer::NewTasks(mut tasks) => connection.queued_tasks.append(&mut tasks),
            FromServer::NewStatusEffects(status_effects) => {
                lua_player.status_effects = status_effects
            }
            FromServer::SpawnObject(object) => connection.spawn_object(object).await,
            FromServer::LocationDiscovered(map_id, map_part_id) => {
                connection.discover_location(map_id, map_part_id).await
            }
            FromServer::StrategyBoardShared(content_id, board_data) => {
                connection
                    .received_strategy_board(content_id, board_data)
                    .await
            }
            FromServer::StrategyBoardSharedAck(content_id) => {
                connection.strategy_board_ack(content_id).await
            }
            FromServer::StrategyBoardRealtimeUpdate(update_data) => {
                connection.strategy_board_updated(update_data).await
            }
            FromServer::StrategyBoardRealtimeFinished() => {
                connection.strategy_board_realtime_finished().await
            }
            FromServer::WaymarkUpdated(id, placement_mode, position, zone_id) => {
                connection
                    .waymark_updated(id, placement_mode, position, zone_id)
                    .await
            }
            FromServer::WaymarkPreset(data, zone_id) => {
                connection.waymark_preset(data, zone_id).await
            }
            FromServer::EnteredInstanceEntranceRange(arg) => {
                tracing::info!("Showing leave duty dialog...");

                let object = ObjectTypeId {
                    object_id: connection.player_data.character.actor_id,
                    object_type: ObjectTypeKind::None,
                };
                let handler_id = HandlerId::new(HandlerType::GimmickRect, 1).0;

                connection
                    .start_event(object, handler_id, EventType::WithinRange, arg, events)
                    .await;

                connection
                    .event_scene(
                        &events.last().unwrap().1,
                        2,
                        SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                        Vec::new(),
                    )
                    .await;
            }
            FromServer::IncrementRestedExp() => connection.add_rested_exp_seconds(10).await,
            FromServer::Countdown(account_id, content_id, name, starter_actor_id, duration) => {
                connection
                    .start_countdown(account_id, content_id, name, starter_actor_id, duration)
                    .await
            }
            FromServer::TargetSignToggled(sign_id, from_actor_id, target_actor) => {
                connection
                    .target_sign_toggled(sign_id, from_actor_id, target_actor)
                    .await
            }
            FromServer::LeaveContent() => {
                connection
                    .handle
                    .send(ToServer::LeaveContent(
                        connection.id,
                        connection.player_data.character.actor_id,
                        connection.old_zone_id,
                        connection.old_position,
                        connection.old_rotation,
                    ))
                    .await;
            }
            FromServer::FinishEvent() => {
                connection.event_finish(events).await;
            }
            FromServer::FishBite() => {
                let handler_id = HandlerId::new(HandlerType::Fishing, 1).0;

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LogMessage {
                    handler_id: HandlerId(handler_id),
                    message_type: 1127,
                    params_count: 0,
                    item_id: 0,
                    item_quantity: 0,
                });
                connection.send_ipc_self(ipc).await;

                connection
                    .event_scene(
                        &events.last().unwrap().1,
                        4,
                        SceneFlags::NO_DEFAULT_CAMERA,
                        vec![271, 0, 0],
                    )
                    .await;
            }
            FromServer::ActorDismounted(from_actor_id) => {
                // SetMode seems unnecessary (the dismount sequence works without it) but it's included for accuracy.
                connection
                    .set_character_mode(CharacterMode::Normal, 0)
                    .await;
                connection
                    .actor_control(
                        from_actor_id,
                        ActorControlCategory::PlayDismountAnimation {
                            unk1: 0,
                            unk2: 0,
                            unk3: 0,
                        },
                    )
                    .await;

                connection
                    .actor_control(
                        from_actor_id,
                        ActorControlCategory::RidePillion {
                            target_actor_id: ObjectId::default(),
                            target_seat_index: 0,
                        },
                    )
                    .await;
            }
            FromServer::PartyMemberPositionsUpdate(positions) => {
                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::PartyMemberPositions(positions));
                connection.send_ipc_self(ipc).await;
            }
            FromServer::FriendInvite(sender_account_id, sender_content_id, sender_name) => {
                connection
                    .received_friend_invite(sender_account_id, sender_content_id, sender_name)
                    .await
            }
            FromServer::CommitParties(parties) => {
                let mut database = connection.database.lock();
                database.commit_parties(parties);
            }
            FromServer::TreasureSpawn(treasure) => connection.spawn_treasure(treasure).await,
            FromServer::SetLinkshellChatChannels(cwlses, _locals, need_to_send_linkshells) => {
                // TODO: There might be a better way to do this. We need the chatchannels to be set *before* sending the "overview" or chat will break.
                connection.set_linkshell_chatchannels(cwlses).await;
                if need_to_send_linkshells {
                    connection.send_crossworld_linkshells(false).await;
                }
            }
            FromServer::LinkshellDisbanded(linkshell_id, _) => {
                connection
                    .crossworld_linkshell_disbanded(linkshell_id)
                    .await;
            }
            FromServer::LinkshellLeft(
                from_actor_id,
                from_content_id,
                from_name,
                rank,
                linkshell_id,
                _,
            ) => {
                connection
                    .leave_linkshell(
                        from_actor_id,
                        from_content_id,
                        from_name.clone(),
                        rank,
                        linkshell_id,
                    )
                    .await
            }
            _ => {
                tracing::error!(
                    "Zone connection {:#?} received a FromServer message we don't care about: {:#?}, ensure you're using the right client network or that you've implemented a handler for it if we actually care about it!",
                    client_handle.id,
                    msg
                );
            }
        }
    }
}

async fn client_loop(
    mut connection: ZoneConnection,
    mut internal_recv: UnboundedReceiver<FromServer>,
    client_handle: ClientHandle,
) {
    let mut lua_player = LuaPlayer::default();

    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    let mut client_handle = client_handle.clone();
    client_handle.actor_id = connection.player_data.character.actor_id;

    // Do an initial update otherwise it may be uninitialized for the first packet that needs Lua
    lua_player.player_data = connection.player_data.clone();

    // tell the server we exist, now that we confirmed we are a legitimate connection
    connection
        .handle
        .send(ToServer::NewClient(client_handle.clone()))
        .await;

    // This is living outside of ZoneConnection (which is weird)
    // because we need the functions in EventHandler to mutate it.
    // Of course, Rust's mutability rules disallow that.
    let mut events: Vec<(Box<dyn EventHandler>, Event)> = Vec::new();

    loop {
        tokio::select! {
            biased; // client data should always be prioritized
            n = connection.socket.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        if !process_packet(&mut connection, &mut lua_player, &mut events, client_handle.clone(), n, &buf).await {
                            break;
                        }
                    },
                    Err(_) => {
                        tracing::info!("ZoneConnection {:#?} was killed because of a network error!", client_handle.id);
                        break;
                    },
                }
            }
            msg = internal_recv.recv() => process_server_msg(&mut connection, &mut lua_player, &mut events, client_handle.clone(), msg).await,
        }
    }

    // forcefully log out the player if they weren't logging out but force D/C'd
    if connection.player_data.character.actor_id.is_valid() {
        if !connection.gracefully_logged_out {
            tracing::info!(
                "Forcefully logging out connection {:#?}...",
                client_handle.id
            );
            connection.begin_log_out().await;
        }
        connection
            .handle
            .send(ToServer::Disconnected(
                connection.id,
                connection.player_data.character.actor_id,
            ))
            .await;

        if connection.is_in_party() {
            connection
                .handle
                .send(ToServer::PartyMemberOffline(
                    connection.party_id,
                    connection.player_data.character.service_account_id as u64,
                    connection.player_data.character.content_id as u64,
                    connection.player_data.character.actor_id,
                    connection.player_data.character.name.clone(),
                ))
                .await;
        }
    }
}

async fn root() -> String {
    "1".to_string()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.world.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    let database = Arc::new(Mutex::new(WorldDatabase::new()));
    let lua = Arc::new(Mutex::new(KawariLua::new()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

    tracing::info!("Server started on {addr}");

    {
        let mut lua = lua.lock();
        if let Err(err) = lua.init(game_data.clone()) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }

    {
        let mut database = database.lock();
        database.do_cleanup_tasks();
    }

    let (handle, _) = spawn_main_loop(game_data.clone(), database.clone());

    // This is a static healthcheck meant for the Kawari Toolbox plugin.
    let app = Router::new().route("/healthcheck", get(root));

    let mut healthcheck_addr = addr;
    healthcheck_addr.set_port(config.world.healthcheck_port);
    let healthcheck_listener = tokio::net::TcpListener::bind(healthcheck_addr)
        .await
        .unwrap();

    tokio::spawn(async move {
        axum::serve(healthcheck_listener, app).await.unwrap();
    });

    loop {
        if let Ok((socket, _)) = listener.accept().await {
            let id = handle.next_id();

            spawn_initial_setup(
                id,
                socket,
                lua.clone(),
                database.clone(),
                game_data.clone(),
                handle.clone(),
            );
        }
    }
}
