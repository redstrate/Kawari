use std::sync::Arc;
use std::time::{Instant, SystemTime};

use axum::Router;
use axum::routing::get;
use kawari::common::{
    ClientLanguage, ContainerType, DirectorEvent, DirectorTrigger, DutyOption, HandlerId,
    HandlerType, INVALID_OBJECT_ID, ItemOperationKind, ObjectId, ObjectTypeId, ObjectTypeKind,
    PlayerStateFlags1, PlayerStateFlags2, PlayerStateFlags3, Position, calculate_max_level,
};
use kawari::config::get_config;
use kawari_world::inventory::{Item, Storage, get_next_free_slot};

use kawari::ipc::chat::{ChatChannel, ClientChatIpcData};

use kawari::ipc::zone::{
    ActorControlCategory, Condition, Conditions, ContentFinderUserAction, EventType, InviteType,
    OnlineStatusMask, PlayerStatus, SceneFlags, SearchInfo, TrustContent, TrustInformation,
};

use kawari::ipc::zone::{
    Blacklist, BlacklistedCharacter, ClientTriggerCommand, ClientZoneIpcData, ServerZoneIpcData,
    ServerZoneIpcSegment,
};

use kawari::common::{NETWORK_TIMEOUT, RECEIVE_BUFFER_SIZE};
use kawari::constants::{AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, CLASSJOB_ARRAY_SIZE};
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{ConnectionState, ConnectionType, SegmentData, parse_packet_header};
use kawari_world::lua::{ExtraLuaState, LuaPlayer, load_init_script};
use kawari_world::{
    ChatConnection, ChatHandler, CustomIpcConnection, GameData, ObsfucationData, TeleportReason,
    ZoneConnection,
};
use kawari_world::{
    ClientHandle, ClientId, FromServer, MessageInfo, PlayerData, ServerHandle, ToServer,
    WorldDatabase, server_main_loop,
};

use mlua::{Function, Lua};
use parking_lot::Mutex;
use tokio::io::AsyncReadExt;
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender, channel, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use kawari::common::INVENTORY_ACTION_ACK_GENERAL;

fn spawn_main_loop(game_data: Arc<Mutex<GameData>>) -> (ServerHandle, JoinHandle<()>) {
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
        let res = server_main_loop(game_data_new, recv).await;
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
    lua: Arc<Mutex<Lua>>,
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
    lua: Arc<Mutex<Lua>>,
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
                    events: Vec::new(),
                    id,
                    handle: handle.clone(),
                    database: database.clone(),
                    lua: lua.clone(),
                    gamedata: game_data.clone(),
                    exit_position: None,
                    exit_rotation: None,
                    last_keep_alive: Instant::now(),
                    gracefully_logged_out: false,
                    obsfucation_data: ObsfucationData::default(),
                    queued_content: None,
                    conditions: Conditions::default(),
                    client_language: ClientLanguage::English,
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
                    transaction_sequence: 0,
                    content_settings: None,
                    current_instance_id: None,
                    glamour_information: None,
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
                    actor_id: INVALID_OBJECT_ID,
                    state,
                    last_keep_alive: Instant::now(),
                    socket,
                    handle,
                    party_chatchannel: ChatChannel::default(),
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

// TODO: Is there a sensible we can reuse the other ClientData type so we don't need 2?
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
                                                connection.handle.send(ToServer::PartyMessageSent(connection.actor_id, data.clone())).await;
                                            }
                                            ClientChatIpcData::GetChannelList { unk } => {
                                                tracing::info!("GetChannelList: {:#?} from {}", unk, connection.actor_id);
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

    let data = ClientZoneData { recv, connection };

    // Spawn a new client task
    let (my_send, my_recv) = oneshot::channel();
    let _kill = tokio::spawn(start_client(my_recv, data));

    // Send client information to said task
    let handle = ClientHandle {
        id: *id,
        channel: send,
        actor_id: *actor_id, // We have the actor id by this point, since Setup is done earlier
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

                            // set equip display flags
                            connection
                                .actor_control_self(ActorControlCategory::SetEquipDisplayFlags {
                                    display_flag: connection.player_data.volatile.display_flags,
                                })
                                .await;

                            // Store when we logged in, for various purposes.
                            connection.login_time = Some(SystemTime::now());

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
                                    ServerZoneIpcData::PlayerStatus(PlayerStatus {
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
                                        unlocks: connection.player_data.unlock.unlocks.0.clone(),
                                        aetherytes: connection
                                            .player_data
                                            .aetheryte
                                            .unlocked
                                            .0
                                            .clone(),
                                        minions: connection.player_data.unlock.minions.0.clone(),
                                        mount_guide_mask: connection
                                            .player_data
                                            .unlock
                                            .mounts
                                            .0
                                            .clone(),
                                        homepoint: connection.player_data.aetheryte.homepoint
                                            as u16,
                                        favourite_aetheryte_count: 1,
                                        favorite_aetheryte_ids: [8, 0, 0, 0],
                                        seen_active_help: connection
                                            .player_data
                                            .unlock
                                            .seen_active_help
                                            .0
                                            .clone(),
                                        aether_currents_mask: connection
                                            .player_data
                                            .aether_current
                                            .unlocked
                                            .0
                                            .clone(),
                                        orchestrion_roll_mask: connection
                                            .player_data
                                            .unlock
                                            .orchestrion_rolls
                                            .0
                                            .clone(),
                                        buddy_equip_mask: connection
                                            .player_data
                                            .companion
                                            .unlocked_equip
                                            .0
                                            .clone(),
                                        cutscene_seen_mask: connection
                                            .player_data
                                            .unlock
                                            .cutscene_seen
                                            .0
                                            .clone(),
                                        ornament_mask: connection
                                            .player_data
                                            .unlock
                                            .ornaments
                                            .0
                                            .clone(),
                                        caught_fish_mask: connection
                                            .player_data
                                            .unlock
                                            .caught_fish
                                            .0
                                            .clone(),
                                        caught_spearfish_mask: connection
                                            .player_data
                                            .unlock
                                            .caught_spearfish
                                            .0
                                            .clone(),
                                        adventure_mask: connection
                                            .player_data
                                            .unlock
                                            .adventures
                                            .0
                                            .clone(),
                                        triple_triad_cards: connection
                                            .player_data
                                            .unlock
                                            .triple_triad_cards
                                            .0
                                            .clone(),
                                        glasses_styles_mask: connection
                                            .player_data
                                            .unlock
                                            .glasses_styles
                                            .0
                                            .clone(),
                                        chocobo_taxi_stands_mask: connection
                                            .player_data
                                            .unlock
                                            .chocobo_taxi_stands
                                            .0
                                            .clone(),
                                        aether_current_comp_flg_set_bitmask1: connection
                                            .player_data
                                            .aether_current
                                            .comp_flg_set
                                            .0[0],
                                        aether_current_comp_flg_set_bitmask2: connection
                                            .player_data
                                            .aether_current
                                            .comp_flg_set
                                            .0[1..AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE]
                                            .to_vec(),

                                        // content
                                        unlocked_special_content: connection
                                            .player_data
                                            .content
                                            .unlocked_special_content
                                            .0
                                            .clone(),
                                        unlocked_dungeons: connection
                                            .player_data
                                            .content
                                            .unlocked_dungeons
                                            .0
                                            .clone(),
                                        unlocked_raids: connection
                                            .player_data
                                            .content
                                            .unlocked_raids
                                            .0
                                            .clone(),
                                        unlocked_guildhests: connection
                                            .player_data
                                            .content
                                            .unlocked_guildhests
                                            .0
                                            .clone(),
                                        unlocked_trials: connection
                                            .player_data
                                            .content
                                            .unlocked_trials
                                            .0
                                            .clone(),
                                        unlocked_crystalline_conflict: connection
                                            .player_data
                                            .content
                                            .unlocked_crystalline_conflicts
                                            .0
                                            .clone(),
                                        unlocked_frontline: connection
                                            .player_data
                                            .content
                                            .unlocked_frontlines
                                            .0
                                            .clone(),
                                        cleared_raids: connection
                                            .player_data
                                            .content
                                            .cleared_raids
                                            .0
                                            .clone(),
                                        cleared_dungeons: connection
                                            .player_data
                                            .content
                                            .cleared_dungeons
                                            .0
                                            .clone(),
                                        cleared_guildhests: connection
                                            .player_data
                                            .content
                                            .cleared_guildhests
                                            .0
                                            .clone(),
                                        cleared_trials: connection
                                            .player_data
                                            .content
                                            .cleared_trials
                                            .0
                                            .clone(),
                                        cleared_crystalline_conflict: connection
                                            .player_data
                                            .content
                                            .cleared_crystalline_conflicts
                                            .0
                                            .clone(),
                                        cleared_frontline: connection
                                            .player_data
                                            .content
                                            .cleared_frontlines
                                            .0
                                            .clone(),
                                        cleared_masked_carnivale: connection
                                            .player_data
                                            .content
                                            .cleared_masked_carnivale
                                            .0
                                            .clone(),
                                        unlocked_misc_content: connection
                                            .player_data
                                            .content
                                            .unlocked_misc_content
                                            .0
                                            .clone(),
                                        cleared_misc_content: connection
                                            .player_data
                                            .content
                                            .cleared_misc_content
                                            .0
                                            .clone(),

                                        ..Default::default()
                                    }),
                                );
                                connection.send_ipc_self(ipc).await;
                            }

                            connection
                                .actor_control_self(ActorControlCategory::SetItemLevel {
                                    level: connection
                                        .player_data
                                        .inventory
                                        .equipped
                                        .calculate_item_level()
                                        as u32,
                                })
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

                            // Send login message
                            connection.send_notice(&config.world.login_message).await;
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
                                            connection.player_data.character.name.clone(),
                                        ))
                                        .await;
                                } else {
                                    connection
                                        .handle
                                        .send(ToServer::PartyMemberReturned(
                                            connection.player_data.character.actor_id,
                                        ))
                                        .await;
                                    connection.rejoining_party = false;
                                }
                            }

                            connection.send_stats().await;

                            // wipe any exit position so it isn't accidentally reused
                            connection.exit_position = None;
                            connection.exit_rotation = None;
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
                                                .0
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
                                            let mut states = [0; 65];
                                            for (i, layout_id) in map_effects.iter().enumerate() {
                                                // A layout ID of zero means the effect should be skipped.
                                                if *layout_id != 0 {
                                                    states[i] = 4; // 4 means to play it, I guess?
                                                }
                                            }

                                            let ipc = ServerZoneIpcSegment::new(
                                                ServerZoneIpcData::DirectorSetupMapEffects {
                                                    handler_id: connection.content_handler_id,
                                                    unk_flag: 5,
                                                    states,
                                                },
                                            );
                                            connection.send_ipc_self(ipc).await;
                                        }
                                    }
                                }
                                ClientTriggerCommand::BeginContentsReplay {} => {
                                    connection
                                        .conditions
                                        .set_condition(Condition::ExecutingGatheringAction);
                                    connection.send_conditions().await;

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

                                    connection
                                        .conditions
                                        .remove_condition(Condition::ExecutingGatheringAction);
                                    connection.send_conditions().await;
                                }
                                ClientTriggerCommand::Dismount { sequence } => {
                                    connection.conditions = Conditions::default();
                                    connection.send_conditions().await;

                                    // TODO: not sure if it's important, retail sends an AC 2 with a param of 1

                                    // Retail indeed does send an AC, not an ACS for this.
                                    connection
                                        .actor_control(
                                            connection.player_data.character.actor_id,
                                            ActorControlCategory::UnkDismountRelated {
                                                unk1: 47494,
                                                unk2: 32711,
                                                unk3: 1510381914,
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
                        ClientZoneIpcData::SetSearchInfoHandler { .. } => {
                            tracing::info!("Recieved SetSearchInfoHandler!");
                        }
                        ClientZoneIpcData::SocialListRequest(request) => {
                            connection
                                .handle
                                .send(ToServer::RequestSocialList(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    connection.party_id,
                                    request.clone(),
                                ))
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

                            connection
                                .handle
                                .send(ToServer::ActorMoved(
                                    connection.id,
                                    connection.player_data.character.actor_id,
                                    *position,
                                    *rotation,
                                    *anim_type,
                                    *anim_state,
                                    *jump_state,
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
                                .send(ToServer::Message(connection.id, info))
                                .await;

                            let mut handled = false;
                            let command_trigger: char = '!';
                            if chat_message.message.starts_with(command_trigger) {
                                let parts: Vec<&str> = chat_message.message.split(' ').collect();
                                let command_name = &parts[0][1..];

                                {
                                    let lua = connection.lua.lock();
                                    let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

                                    // If a Lua command exists, try using that first
                                    if let Some(command_script) =
                                        state.command_scripts.get(command_name)
                                    {
                                        handled = true;

                                        let file_name = format!(
                                            "{}/{}",
                                            &config.world.scripts_location, command_script
                                        );

                                        let mut run_script = || -> mlua::Result<()> {
                                            lua.scope(|scope| {
                                                    let connection_data = scope
                                                    .create_userdata_ref_mut(lua_player)?;

                                                    /* TODO: Instead of panicking we ought to send a message to the player
                                                     * and the console log, and abandon execution. */
                                                    lua.load(
                                                        std::fs::read(&file_name).unwrap_or_else(|_| panic!("Failed to load script file {}!", &file_name)),
                                                    )
                                                    .set_name("@".to_string() + &file_name)
                                                    .exec()?;

                                                    let required_rank = lua.globals().get("required_rank");
                                                    if let Err(error) = required_rank {
                                                        tracing::info!("Script is missing required_rank! Unable to run command, sending error to user. Additional information: {}", error);
                                                        let func: Function =
                                                        lua.globals().get("onCommandRequiredRankMissingError")?;
                                                        func.call::<()>((error.to_string(), connection_data))?;
                                                        return Ok(());
                                                    }

                                                    /* Reset state for future commands. Without this it'll stay set to the last value
                                                     * and allow other commands that omit required_rank to run, which is undesirable. */
                                                    lua.globals().set("required_rank", mlua::Value::Nil)?;

                                                    if connection.player_data.character.gm_rank as u8 >= required_rank? {
                                                        let mut func_args = Vec::new();
                                                        if parts.len() > 1 {
                                                            func_args = (parts[1..]).to_vec();
                                                            tracing::info!("Args passed to Lua command {}: {:?}", command_name, func_args);
                                                        } else {
                                                            tracing::info!("No additional args passed to Lua command {}.", command_name);
                                                        }
                                                        let func: Function =
                                                        lua.globals().get("onCommand")?;
                                                        func.call::<()>((func_args, connection_data))?;

                                                        /* `command_sender` is an optional variable scripts can define to identify themselves in print messages.
                                                         * It's okay if this global isn't set. We also don't care what its value is, just that it exists.
                                                         * This is reset -after- running the command intentionally. Resetting beforehand will never display the command's identifier.
                                                         */
                                                        let command_sender: Result<mlua::prelude::LuaValue, mlua::prelude::LuaError> = lua.globals().get("command_sender");
                                                        if command_sender.is_ok() {
                                                            lua.globals().set("command_sender", mlua::Value::Nil)?;
                                                        }
                                                        Ok(())
                                                    } else {
                                                        tracing::info!("User with account_id {} tried to invoke GM command {} with insufficient privileges!",
                                                                       connection.player_data.character.service_account_id, command_name);
                                                        let func: Function =
                                                        lua.globals().get("onCommandRequiredRankInsufficientError")?;
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
                                        lua_player,
                                    )
                                    .await;
                                }

                                // If it's truly not existent:
                                if !handled {
                                    tracing::info!("Unknown command {command_name}");

                                    let lua = connection.lua.lock();

                                    let mut call_func = || {
                                        lua.scope(|scope| {
                                            let connection_data =
                                                scope.create_userdata_ref_mut(lua_player)?;
                                            let func: Function =
                                                lua.globals().get("onUnknownCommandError")?;
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
                        ClientZoneIpcData::EventRelatedUnk {
                            unk1,
                            unk2,
                            unk3,
                            unk4,
                        } => {
                            tracing::info!("Recieved EventRelatedUnk! {unk1} {unk2} {unk3} {unk4}");
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
                                        dst_actor_id: INVALID_OBJECT_ID,
                                        dummy_container: ContainerType::DiscardingItemSentinel,
                                        dst_storage_id: ContainerType::DiscardingItemSentinel,
                                        dst_container_index: u16::MAX,
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
                                && action.src_container_index == 0)
                                || (action.dst_storage_id == ContainerType::Equipped
                                    && action.dst_container_index == 0)
                            {
                                // We need to update our current class based on the weapon...
                                connection.change_class_based_on_weapon().await;

                                let id;
                                {
                                    let mut gamedata = connection.gamedata.lock();
                                    id = gamedata.get_soul_crystal_item_id(
                                        connection.player_data.classjob.current_class as u16,
                                    );
                                }
                                if let Some(id) = id {
                                    connection.player_data.inventory.equip_soul_crystal(id);

                                    // Then re-check the soul crystal...
                                    connection.change_class_based_on_soul_crystal().await;
                                } else {
                                    connection.player_data.inventory.unequip_soul_crystal();
                                }
                            }

                            // If the soul crystal is changed, ensure we update accordingly.
                            if (action.src_storage_id == ContainerType::Equipped
                                && action.src_container_index == 13)
                                || (action.dst_storage_id == ContainerType::Equipped
                                    && action.dst_container_index == 13)
                            {
                                let soul_crystal =
                                    connection.player_data.inventory.equipped.soul_crystal;
                                if soul_crystal.quantity > 0 {
                                    connection.change_class_based_on_soul_crystal().await;
                                } else {
                                    connection.change_class_based_on_weapon().await;
                                }
                            }

                            // If the client modified their equipped items, we have to process that
                            if action.src_storage_id == ContainerType::Equipped
                                || action.dst_storage_id == ContainerType::Equipped
                            {
                                connection.inform_equip().await;
                                connection.update_server_stats().await;
                            }
                        }
                        ClientZoneIpcData::EventReturnHandler4(handler) => {
                            let event_type = handler.handler_id.handler_type();

                            // It always assumes a shop... for now
                            if event_type == HandlerType::Shop {
                                connection
                                    .process_shop_event_return(handler, lua_player)
                                    .await;
                            } else {
                                tracing::info!(message = "Event returned", handler_id = %handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                                if let Some(event) = connection.events.last_mut() {
                                    event.do_return(
                                        handler.scene,
                                        &handler.params[..handler.num_results as usize],
                                        lua_player,
                                    );
                                } else {
                                    tracing::warn!(
                                        "Don't know how to return in {event_type} and there's no current event!"
                                    );
                                }
                            }
                        }
                        ClientZoneIpcData::StartTalkEvent {
                            actor_id,
                            handler_id,
                        } => {
                            if connection
                                .start_event(
                                    *actor_id,
                                    handler_id.0,
                                    EventType::Talk,
                                    0,
                                    Some(Condition::OccupiedInQuestEvent),
                                    lua_player,
                                )
                                .await
                            {
                                connection
                                    .conditions
                                    .set_condition(Condition::OccupiedInQuestEvent);
                                connection.send_conditions().await;

                                // begin talk function if it exists
                                if let Some(event) = connection.events.last_mut() {
                                    event.talk(*actor_id, lua_player);
                                }
                            } else {
                                connection.send_conditions().await;
                            }
                        }
                        ClientZoneIpcData::EventYieldHandler(handler) => {
                            tracing::info!(message = "Event yielded", handler_id = %handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            connection.events.last_mut().unwrap().finish(
                                handler.scene,
                                &handler.params[..handler.num_results as usize],
                                lua_player,
                            );
                        }
                        ClientZoneIpcData::EventYieldHandler8(handler) => {
                            tracing::info!(message = "Event yielded", handler_id = %handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                            connection.events.last_mut().unwrap().finish(
                                handler.scene,
                                &handler.params[..handler.num_results as usize],
                                lua_player,
                            );
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
                        ClientZoneIpcData::EventUnkRequest {
                            handler_id,
                            unk1,
                            unk2,
                            unk3,
                        } => {
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventUnkReply {
                                handler_id: *handler_id,
                                unk1: *unk1,
                                unk2: *unk2,
                                unk3: *unk3 + 1,
                            });
                            connection.send_ipc_self(ipc).await;
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

                                    let target_container_type = match slot {
                                        0 => ContainerType::ArmoryWeapon,
                                        1 => ContainerType::ArmoryOffWeapon,
                                        2 => ContainerType::ArmoryHead,
                                        3 => ContainerType::ArmoryBody,
                                        4 => ContainerType::ArmoryHand,
                                        5 => ContainerType::ArmoryWaist,
                                        6 => ContainerType::ArmoryLeg,
                                        7 => ContainerType::ArmoryFoot,
                                        8 => ContainerType::ArmoryEarring,
                                        9 => ContainerType::ArmoryNeck,
                                        10 => ContainerType::ArmoryWrist,
                                        11 => ContainerType::ArmoryRing,
                                        12 => ContainerType::ArmoryRing,
                                        13 => ContainerType::ArmorySoulCrystal,
                                        _ => unreachable!(),
                                    };

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
                            connection.update_server_stats().await;
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

                            let condition = if handler_id.handler_type() == HandlerType::Opening {
                                Condition::Occupied33 // This stops you in your tracks
                            } else {
                                Condition::OccupiedInEvent // S9 teleporters and stuff
                            };

                            connection.conditions.set_condition(condition);
                            connection.send_conditions().await;

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
                                    Some(condition),
                                    lua_player,
                                )
                                .await;

                            // begin walk-in trigger function if it exists
                            if let Some(event) = connection.events.last_mut() {
                                event.enter_trigger(lua_player, *event_arg);
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

                            let condition = if handler_id.handler_type() == HandlerType::Opening {
                                Condition::Occupied33 // This stops you in your tracks
                            } else {
                                Condition::OccupiedInEvent
                            };

                            connection.conditions.set_condition(condition);
                            connection.send_conditions().await;

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
                                    Some(condition),
                                    lua_player,
                                )
                                .await;

                            // begin walk-in trigger function if it exists
                            if let Some(event) = connection.events.last_mut() {
                                event.enter_trigger(lua_player, *event_arg);
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
                            // NOTE: Failing to respond to this request means PlayerSpawn will not work and other players will be invisible, have their chat ignored and possibly other issues by the client! Beware!
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
                            tracing::info!("Linkshells is unimplemented");
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
                            connection.client_language = *language;
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
                                    // Inform the client about the invite they just sent.
                                    // TODO: Is this static? unk1 and unk2 haven't been observed to have other values so far.
                                    let ipc = ServerZoneIpcSegment::new(
                                        ServerZoneIpcData::InviteCharacterResult {
                                            content_id: *content_id,
                                            world_id: *world_id,
                                            unk1: 1,
                                            unk2: 1,
                                            character_name: character_name.clone(),
                                        },
                                    );
                                    connection.send_ipc_self(ipc).await;
                                }
                                InviteType::FriendList => {
                                    connection
                                        .send_notice("The friend list is not yet implemented.")
                                        .await
                                }
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
                        ClientZoneIpcData::RequestSearchInfo { .. } => {
                            tracing::info!("Requesting search info is unimplemented");
                        }
                        ClientZoneIpcData::RequestAdventurerPlate { .. } => {
                            tracing::info!("Requesting adventurer plates is unimplemented");
                        }
                        ClientZoneIpcData::SearchPlayers { .. } => {
                            tracing::info!("Searching for players is unimplemented");
                        }
                        ClientZoneIpcData::EditSearchInfo { .. } => {
                            let ipc =
                                ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateSearchInfo {
                                    online_status: OnlineStatusMask::default(),
                                    unk1: 0,
                                    unk2: 0,
                                    region: 0,
                                    message: String::default(),
                                });
                            connection.send_ipc_self(ipc).await;
                        }
                        ClientZoneIpcData::RequestOwnSearchInfo { .. } => {
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SetSearchInfo(
                                SearchInfo::default(),
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
                                    None,
                                    lua_player,
                                )
                                .await;
                            if let Some(event) = connection.events.last_mut() {
                                event.enter_territory(lua_player);
                            }
                        }
                        ClientZoneIpcData::Trade { .. } => {
                            tracing::info!("Trading is unimplemented");
                        }
                        ClientZoneIpcData::BuyInclusionShop {
                            shop_id,
                            special_shop_id,
                            item_index,
                            ..
                        } => {
                            tracing::info!("Buying item {item_index} from {special_shop_id}...");
                            connection
                                .buy_special_shop(
                                    *shop_id,
                                    *special_shop_id,
                                    *item_index,
                                    lua_player,
                                )
                                .await;
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
                                    waymark_preset.clone(),
                                ))
                                .await;
                        }
                        ClientZoneIpcData::RequestFreeCompanyShortMessage { .. } => {
                            tracing::warn!(
                                "Requesting a free company short message is unimplemented"
                            );
                        }
                        ClientZoneIpcData::PlayGoldSaucerMachine {
                            handler_id,
                            unk1,
                            unk2,
                            unk3,
                        } => {
                            tracing::info!("Playing machine {handler_id} {unk1} {unk2} {unk3}");
                        }
                        ClientZoneIpcData::InitiateReadyCheck { .. } => {
                            tracing::info!("Initiating ready checks is unimplemented");
                        }
                        ClientZoneIpcData::ReadyCheckResponse { response: _ } => {
                            tracing::info!("Replying to ready checks is unimplemented");
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
        connection.process_lua_player(lua_player).await;

        // update lua player
        lua_player.player_data = connection.player_data.clone();
    }

    true
}

/// Process internal server messages.
async fn process_server_msg(
    connection: &mut ZoneConnection,
    lua_player: &mut LuaPlayer,
    client_handle: ClientHandle,
    msg: Option<FromServer>,
) {
    if let Some(msg) = msg {
        match msg {
        FromServer::Message(msg) => connection.send_message(msg).await,
        FromServer::ActorSpawn(actor, spawn) => connection.spawn_actor(actor, spawn).await,
        FromServer::ActorMove(actor_id, position, rotation, anim_type, anim_state, jump_state) => connection.set_actor_position(actor_id, position, rotation, anim_type, anim_state, jump_state).await,
        FromServer::DeleteActor(object_id, spawn_index) => connection.delete_actor(object_id, spawn_index).await,
        FromServer::DeleteObject(spawn_index) => connection.delete_object(spawn_index).await,
        FromServer::ActorControl(actor_id, actor_control) => connection.actor_control(actor_id, actor_control).await,
        FromServer::ActorControlTarget(actor_id, actor_control) => connection.actor_control_target(actor_id, actor_control).await,
        FromServer::ActorControlSelf(actor_control) => connection.actor_control_self(actor_control).await,
        FromServer::ActorSummonsMinion(minion_id) => {
            connection.handle.send(ToServer::ActorSummonsMinion(connection.id, connection.player_data.character.actor_id, minion_id)).await;
            connection.active_minion = minion_id;
        }
        FromServer::ActorDespawnsMinion() => {
            connection.handle.send(ToServer::ActorDespawnsMinion(connection.id, connection.player_data.character.actor_id)).await;
            connection.active_minion = 0;
        }
        FromServer::UpdateConfig(actor_id, config) => connection.update_config(actor_id, config).await,
        FromServer::ActorEquip(actor_id, main_weapon_id, sub_weapon_id, model_ids) => connection.update_equip(actor_id, main_weapon_id, sub_weapon_id, model_ids).await,
        FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id) => connection.lose_effect(effect_id, effect_param, effect_source_actor_id).await,
        FromServer::Conditions(conditions) => {
            connection.conditions = conditions;
            connection.send_conditions().await;
        },
        FromServer::ChangeZone(zone_id, content_finder_condition_id, weather_id, position, rotation, lua_zone, initial_login) => {
            connection.handle_zone_change(zone_id, content_finder_condition_id, weather_id, position, rotation, initial_login, &lua_zone, &mut lua_player.content_data).await;
            lua_player.zone_data = lua_zone;
        },
        FromServer::NewPosition(position, rotation, fade_out) => connection.set_player_position(position, rotation, fade_out).await,
        FromServer::PartyInvite(sender_account_id, sender_content_id, sender_name) => connection.received_party_invite(sender_account_id, sender_content_id, sender_name).await,
        FromServer::InvitationResult(sender_account_id, sender_content_id, sender_name, invite_type, invite_reply) => connection.received_invitation_response(sender_account_id, sender_content_id, sender_name, invite_type, invite_reply).await,
        FromServer::InvitationReplyResult(sender_account_id, sender_name, invite_type, invite_reply) => connection.send_invite_reply_result(sender_account_id, sender_name, invite_type, invite_reply).await,
        FromServer::SocialListResponse(request_type, sequence, entries) => connection.send_social_list(request_type, sequence, entries).await,
        FromServer::PartyUpdate(targets, update_status, party_info) => connection.send_party_update(targets, update_status, party_info).await,
        FromServer::CharacterAlreadyInParty() => connection.send_notice("That player is already in a party. You are seeing this message because Kawari doesn't yet send information correctly in a way that your game will display the error on its own.").await,
        FromServer::RejoinPartyAfterDisconnect(party_id) => {
            connection.party_id = party_id;
            connection.rejoining_party = true;
        }
        FromServer::PacketSegment(ipc, from_actor_id) => {
            connection.send_ipc_from(from_actor_id, ipc).await;
        }
        FromServer::NewTasks(mut tasks) => connection.queued_tasks.append(&mut tasks),
        FromServer::NewStatusEffects(status_effects) => lua_player.status_effects = status_effects,
        FromServer::ObjectSpawn(object) => connection.spawn_object(object).await,
        FromServer::LocationDiscovered(map_id, map_part_id) => connection.discover_location(map_id, map_part_id).await,
        FromServer::StrategyBoardShared(content_id, board_data) => connection.received_strategy_board(content_id, board_data).await,
        FromServer::StrategyBoardSharedAck(content_id) => connection.strategy_board_ack(content_id).await,
        FromServer::StrategyBoardRealtimeUpdate(update_data) => connection.strategy_board_updated(update_data).await,
        FromServer::StrategyBoardRealtimeFinished() => connection.strategy_board_realtime_finished().await,
        FromServer::WaymarkUpdated(id, placement_mode, unk1, unk2, unk3) => connection.waymark_updated(id, placement_mode, unk1, unk2, unk3).await,
        FromServer::WaymarkPreset(data) => connection.waymark_preset(data).await,
        FromServer::EnteredInstanceEntranceRange(arg) => {
            tracing::info!("Showing leave duty dialog...");

            let object = ObjectTypeId { object_id: connection.player_data.character.actor_id, object_type: ObjectTypeKind::None };
            let handler_id = HandlerId::new(HandlerType::GimmickRect, 1).0;

            connection.start_event(object, handler_id, EventType::WithinRange, arg, Some(Condition::OccupiedInEvent), lua_player).await;

            connection.conditions.set_condition(Condition::OccupiedInEvent);
            connection.send_conditions().await;

            connection.event_scene(handler_id, 2, SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR, Vec::new(), lua_player).await;
        }
        FromServer::IncrementRestedExp() => connection.add_rested_exp_seconds(10).await,
        FromServer::Countdown(account_id, content_id, name, starter_actor_id, duration) => connection.start_countdown(account_id, content_id, name, starter_actor_id, duration).await,
        FromServer::TargetSignToggled(sign_id, from_actor_id, target_actor_id, on) => connection.target_sign_toggled(sign_id, from_actor_id, target_actor_id, on).await,
        _ => { tracing::error!("Zone connection {:#?} received a FromServer message we don't care about: {:#?}, ensure you're using the right client network or that you've implemented a handler for it if we actually care about it!", client_handle.id, msg); }
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

    loop {
        tokio::select! {
            biased; // client data should always be prioritized
            n = connection.socket.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        if !process_packet(&mut connection, &mut lua_player, client_handle.clone(), n, &buf).await {
                            break;
                        }
                    },
                    Err(_) => {
                        tracing::info!("ZoneConnection {:#?} was killed because of a network error!", client_handle.id);
                        break;
                    },
                }
            }
            msg = internal_recv.recv() => process_server_msg(&mut connection, &mut lua_player, client_handle.clone(), msg).await,
        }
    }

    // forcefully log out the player if they weren't logging out but force D/C'd
    if connection.player_data.character.actor_id != INVALID_OBJECT_ID {
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
    let lua = Arc::new(Mutex::new(Lua::new()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

    tracing::info!("Server started on {addr}");

    {
        let mut lua = lua.lock();
        if let Err(err) = load_init_script(&mut lua, game_data.clone()) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }

    let (handle, _) = spawn_main_loop(game_data.clone());

    // This is a static healthcheck meant for the Kawari Toolbox plugin.
    let app = Router::new().route("/healthcheck", get(root));

    let mut healthcheck_addr = addr;
    healthcheck_addr.set_port(5807); // TODO: make configurable
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
