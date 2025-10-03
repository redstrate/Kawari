use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kawari::common::{
    GameData, INVALID_OBJECT_ID, ItemInfoQuery, ObjectId, ObjectTypeId, ObjectTypeKind,
    value_to_flag_byte_index_value,
};
use kawari::config::get_config;
use kawari::inventory::{
    BuyBackItem, ContainerType, CurrencyKind, Item, ItemOperationKind, get_container_type,
};

use kawari::ipc::chat::{ClientChatIpcData, ServerChatIpcSegment};

use kawari::ipc::zone::{
    ActorControl, ActorControlCategory, ActorControlSelf, ClientLanguage, Condition, Conditions,
    ItemOperation, OnlineStatusMask, PlayerEntry, PlayerSpawn, PlayerStatus, SocialList,
    SocialListUILanguages,
};

use kawari::ipc::zone::{
    Blacklist, BlacklistedCharacter, ClientTriggerCommand, ClientZoneIpcData, GameMasterRank,
    OnlineStatus, ServerZoneIpcData, ServerZoneIpcSegment, SocialListRequestType,
    SocialListUIFlags,
};

use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{
    ConnectionState, ConnectionType, SegmentData, parse_packet_header, send_keep_alive,
};
use kawari::world::lua::{ExtraLuaState, LuaPlayer, load_init_script};
use kawari::world::{
    ChatConnection, ChatHandler, CustomIpcConnection, ObsfucationData, TeleportReason,
    ZoneConnection,
};
use kawari::world::{
    ClientHandle, ClientId, EventFinishType, FromServer, MessageInfo, PlayerData, ServerHandle,
    StatusEffects, ToServer, WorldDatabase, server_main_loop,
};
use kawari::{
    ERR_INVENTORY_ADD_FAILED, LogMessageType, MINION_BITMASK_SIZE, MOUNT_BITMASK_SIZE,
    RECEIVE_BUFFER_SIZE, TITLE_UNLOCK_BITMASK_SIZE,
};

use mlua::{Function, Lua};
use tokio::io::AsyncReadExt;
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender, channel, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use kawari::{INVENTORY_ACTION_ACK_GENERAL, INVENTORY_ACTION_ACK_SHOP};

fn spawn_main_loop() -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(64);

    let handle = ServerHandle {
        chan: send,
        next_id: Default::default(),
    };

    let join = tokio::spawn(async move {
        let res = server_main_loop(recv).await;
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
    ip: SocketAddr,
    id: ClientId,
    socket: TcpStream,
    lua: Arc<Mutex<Lua>>,
    database: Arc<WorldDatabase>,
    gamedata: Arc<Mutex<GameData>>,
    handle: ServerHandle,
) {
    let _kill = tokio::spawn(initial_setup(
        ip, id, socket, lua, database, gamedata, handle,
    ));
}

/// The initial setup loop, which figures out what the remote connection wants and branches off to provide a chat connection, zone connection, or custom IPC connection.
async fn initial_setup(
    ip: SocketAddr,
    id: ClientId,
    mut socket: TcpStream,
    lua: Arc<Mutex<Lua>>,
    database: Arc<WorldDatabase>,
    game_data: Arc<Mutex<GameData>>,
    handle: ServerHandle,
) {
    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    let last_keep_alive = Instant::now();

    loop {
        tokio::select! {
            biased; // client data should always be prioritized

            n = socket.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        // if the last response was over >5 seconds, the client is probably gone
                        if n == 0 {
                            let now = Instant::now();
                            if now.duration_since(last_keep_alive) > Duration::from_secs(5) {
                                tracing::info!("initial_setup: Connection was killed because of timeout or they are now handled by the proper connection type");
                                break;
                            }
                        } else {
                            let header = parse_packet_header(&buf[..n]);
                            if header.connection_type == ConnectionType::KawariIpc {
                                let mut connection = CustomIpcConnection {
                                    socket,
                                    ip,
                                    state: ConnectionState::None,
                                    last_keep_alive: Instant::now(),
                                    database: database.clone(),
                                    gamedata: game_data.clone(),
                                };
                                // Handle the first batch of segments before handing off control to the loop proper.
                                let (segments, _) = connection.parse_packet(&buf[..n]);
                                for segment in segments {
                                    match &segment.data {
                                        SegmentData::KawariIpc(data) => connection.handle_custom_ipc(data).await,
                                        _ => panic!("initial_setup: The KawariIpc connection type only supports KawariIpc segments! Was a mistake made somewhere? Received: {segment:#?}")
                                    }
                                }

                                spawn_customipc_connection(connection);
                                break;
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
                                    spawn_index: 0,
                                    status_effects: StatusEffects::default(),
                                    event: None,
                                    actors: Vec::new(),
                                    ip,
                                    id,
                                    handle: handle.clone(),
                                    database: database.clone(),
                                    lua: lua.clone(),
                                    gamedata: game_data.clone(),
                                    exit_position: None,
                                    exit_rotation: None,
                                    last_keep_alive: Instant::now(),
                                    gracefully_logged_out: false,
                                    weather_id: 0,
                                    obsfucation_data: ObsfucationData::default(),
                                    queued_content: None,
                                    event_type: 0,
                                    conditions: Conditions::default(),
                                };

                                // Handle setup before passing off control to the zone connection.
                                let (segments, _) = connection.parse_packet(&buf[..n]);
                                for segment in segments {
                                    match &segment.data {
                                        SegmentData::Setup { actor_id } => {
                                            // for some reason they send a string representation
                                            let actor_id = actor_id.parse::<u32>().unwrap();

                                            // initialize player data if it doesn't exist
                                            if connection.player_data.actor_id == 0 {
                                                let player_data;
                                                {
                                                    let mut game_data = connection.gamedata.lock().unwrap();
                                                    player_data = database.find_player_data(actor_id, &mut game_data);
                                                }
                                                connection.player_data = player_data;
                                            }

                                            // collect actor data
                                            connection.initialize(actor_id).await;

                                        }
                                        _ => panic!("initial_setup: The zone connection type must start with a Setup segment! What happened? Received: {segment:#?}")
                                    }
                                }
                                spawn_client(connection);
                                break;
                            } else if header.connection_type == ConnectionType::Chat {
                                let state = ConnectionState::Zone {
                                    clientbound_oodle: OodleNetwork::new(),
                                    serverbound_oodle: OodleNetwork::new(),
                                    scrambler_keys: None,
                                };

                                let mut connection = ChatConnection {
                                    config: get_config().world,
                                    ip,
                                    id,
                                    actor_id: 0,
                                    state,
                                    last_keep_alive: Instant::now(),
                                    socket,
                                };

                                // Handle setup before passing off control to the chat connection.
                                let (segments, _) = connection.parse_packet(&buf[..n]);
                                for segment in segments {
                                    match &segment.data {
                                        SegmentData::Setup { actor_id } => {
                                            // for some reason they send a string representation
                                            let actor_id = actor_id.parse::<u32>().unwrap();
                                            connection.actor_id = actor_id;
                                            connection.initialize().await;
                                        }
                                        _ => panic!("initial_setup: The chat connection type must start with a Setup segment! What happened? Received: {segment:#?}")
                                    }
                                }
                                spawn_chat_connection(connection);
                                break;
                            } else {
                                panic!("Connection type is None! How did this happen?");
                            }
                        }
                    }
                    Err(_) => {
                        tracing::info!("initial_setup: Connection was killed because of a network error!");
                        break;
                    },
                }
            }
        }
    }
}

// TODO: Is there a sensible we can reuse the other ClientData type so we don't need 2?
struct ClientChatData {
    /// Socket for data recieved from the global server
    recv: Receiver<FromServer>,
    connection: ChatConnection,
}

/// A task that spawns the CustomIpcConnection loop.
fn spawn_customipc_connection(connection: CustomIpcConnection) {
    let _kill = tokio::spawn(customipc_loop(connection));
}

/// The CustomIpcConnection loop. It processes everything the lobby server needs to log clients in.
async fn customipc_loop(mut connection: CustomIpcConnection) {
    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];

    loop {
        tokio::select! {
            biased; // client data should always be prioritized

            n = connection.socket.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        // if the last response was over >5 seconds, the client is probably gone; we also don't care about id numbers on this connection
                        if n == 0 {
                            let now = Instant::now();
                            if now.duration_since(connection.last_keep_alive) > Duration::from_secs(5) {
                                tracing::info!("CustomIpcConnection: Connection was killed because of timeout");
                                break;
                            }
                        } else {
                            connection.last_keep_alive = Instant::now();
                            let (segments, _) = connection.parse_packet(&buf);
                            for segment in segments {
                                match &segment.data {
                                        SegmentData::KawariIpc(data) => connection.handle_custom_ipc(data).await,
                                        // TODO: Should this support keepalives/keepaliveresponses someday?
                                        _ => panic!("CustomIpcConnection: Received an invalid segment type! The custom IPC connection only supports KawariIpc! {segment:#?}"),
                                }
                            }
                        }
                    }
                    Err(_) => {
                        tracing::info!("CustomIpcConnection: Connection was killed because of a network error!");
                        break;
                    },
                }
            }
        }
    }
}

/// Spawn a new chat connection for an incoming client.
fn spawn_chat_connection(connection: ChatConnection) {
    let (send, recv) = channel(64);

    let id = &connection.id.clone();
    let ip = &connection.ip.clone();

    let data = ClientChatData { recv, connection };

    // Spawn a new client task
    let (my_send, my_recv) = oneshot::channel();
    let _kill = tokio::spawn(start_chat_connection(my_recv, data));

    // Send client information to said task
    let handle = ClientHandle {
        id: *id,
        ip: *ip,
        channel: send,
        actor_id: 0,
    };
    let _ = my_send.send(handle);
}

/// THe task that kickstarts the client chat connection loop.
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
                            if now.duration_since(connection.last_keep_alive) > Duration::from_secs(5) {
                                tracing::info!("Connection {:#?} was killed because of timeout", client_handle.id);
                                break;
                            }
                        } else {
                            connection.last_keep_alive = Instant::now();
                            let (segments, _) = connection.parse_packet(&buf);
                            for segment in segments {
                                match &segment.data {
                                    SegmentData::None() => {}
                                    SegmentData::Setup {.. } => {
                                        // Handled before our connection was spawned!
                                    }
                                    SegmentData::Ipc(data) => {
                                        match &data.data {
                                            // TODO: Add support for tells, party messages, and so on!
                                            // These are added as a skeleton for now, to bring us up to feature parity with upstream.
                                            ClientChatIpcData::SendTellMessage(data) => {
                                                tracing::info!("SendTellMessage: {:#?} from {}", data, connection.actor_id);
                                            }
                                            ClientChatIpcData::SendPartyMessage(data) => {
                                                 tracing::info!("SendPartyMessage: {:#?} from {}", data, connection.actor_id);
                                            }
                                            ClientChatIpcData::Unknown { unk } => {
                                                tracing::warn!("Unknown Chat packet {:?} recieved ({} bytes), this should be handled!", data.header.op_code, unk.len());
                                            }
                                        }
                                    }
                                    SegmentData::KeepAliveRequest { id, timestamp } => {
                                        send_keep_alive::<ServerChatIpcSegment> (
                                            &mut connection.socket,
                                            &mut connection.state,
                                            ConnectionType::Chat,
                                            *id,
                                            *timestamp,
                                        )
                                        .await
                                    }
                                    SegmentData::KeepAliveResponse { .. } => {
                                        // these should be safe to ignore
                                    }
                                    _ => panic!("ChatConnection: The server is receiving a response or an unknown packet: {segment:#?}"),
                                }
                            }
                        }
                    }
                    Err(_) => {
                        tracing::info!("Connection {:#?} was killed because of a network error!", client_handle.id);
                        break;
                    },
                }
            }
            // TODO: We don't yet have any chat-specific messages!
            msg = internal_recv.recv() => match msg {
                Some(_msg) => todo!(), // This is only here to suppress clippy warning: "this match could be replaced by its body itself"
                // TODO: uncomment this and fill it in once we have chat-specific messages!
                /*Some(msg) => match msg {
                    _ => todo!()
                },*/
                None => break,
            }
        }
    }
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
    let ip = &connection.ip.clone();

    let data = ClientZoneData { recv, connection };

    // Spawn a new client task
    let (my_send, my_recv) = oneshot::channel();
    let _kill = tokio::spawn(start_client(my_recv, data));

    // Send client information to said task
    let handle = ClientHandle {
        id: *id,
        ip: *ip,
        channel: send,
        actor_id: 0,
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
        internal_send.send(msg).unwrap()
    }
}

async fn client_loop(
    mut connection: ZoneConnection,
    mut internal_recv: UnboundedReceiver<FromServer>,
    client_handle: ClientHandle,
) {
    let database = connection.database.clone();
    let game_data = connection.gamedata.clone();
    let lua = connection.lua.clone();
    let config = get_config();

    let mut lua_player = LuaPlayer::default();

    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    let mut client_handle = client_handle.clone();
    client_handle.actor_id = connection.player_data.actor_id;

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
                        // if the last response was over >5 seconds, the client is probably gone
                        if n == 0 {
                            let now = Instant::now();
                            if now.duration_since(connection.last_keep_alive) > Duration::from_secs(5) {
                                tracing::info!("Connection {:#?} was killed because of timeout", client_handle.id);
                                break;
                            }
                        } else {
                            connection.last_keep_alive = Instant::now();

                            let (segments, _) = connection.parse_packet(&buf[..n]);
                            for segment in &segments {
                                match &segment.data {
                                    SegmentData::None() => {}
                                    SegmentData::Setup { .. } => {
                                        // Handled before our connection was spawned!
                                    }
                                    SegmentData::Ipc(data) => {
                                        match &data.data {
                                            ClientZoneIpcData::InitRequest { .. } => {
                                                tracing::info!(
                                                    "Client is now requesting zone information. Sending!"
                                                );

                                                // IPC Init(?)
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InitResponse {
                                                        unk1: 0,
                                                        character_id: connection.player_data.actor_id,
                                                        unk2: 0,
                                                    });
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                let chara_details =
                                                database.find_chara_make(connection.player_data.content_id);

                                                // Send inventory
                                                connection.send_inventory(true).await;

                                                // set equip display flags
                                                connection
                                                .actor_control_self(ActorControlSelf {
                                                    category: ActorControlCategory::SetEquipDisplayFlags {
                                                        display_flag: connection.player_data.display_flags
                                                    },
                                                })
                                                .await;

                                                // Stats
                                                connection.send_stats(&chara_details).await;

                                                let current_class;
                                                {
                                                    let game_data = connection.gamedata.lock().unwrap();
                                                    current_class = game_data.get_exp_array_index(connection.player_data.classjob_id as u16).unwrap();
                                                }

                                                // Player Setup
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerStatus(PlayerStatus {
                                                        content_id: connection.player_data.content_id,
                                                        exp: connection.player_data.classjob_exp,
                                                        max_level: 100,
                                                        expansion: 5,
                                                        name: chara_details.name,
                                                        actor_id: connection.player_data.actor_id,
                                                        race: chara_details.chara_make.customize.race,
                                                        gender: chara_details.chara_make.customize.gender,
                                                        tribe: chara_details.chara_make.customize.subrace,
                                                        city_state: chara_details.city_state,
                                                        nameday_month: chara_details.chara_make.birth_month
                                                        as u8,
                                                        nameday_day: chara_details.chara_make.birth_day as u8,
                                                        deity: chara_details.chara_make.guardian as u8,
                                                        current_class: current_class as u8,
                                                        current_job: connection.player_data.classjob_id,
                                                        levels: connection.player_data.classjob_levels.map(|x| x as u16),
                                                        unlocks: connection.player_data.unlocks.unlocks.clone(),
                                                        aetherytes: connection.player_data.unlocks.aetherytes.clone(),
                                                        unlocked_raids: connection.player_data.unlocks.unlocked_raids.clone(),
                                                        unlocked_dungeons: connection.player_data.unlocks.unlocked_dungeons.clone(),
                                                        unlocked_guildhests: connection.player_data.unlocks.unlocked_guildhests.clone(),
                                                        unlocked_trials: connection.player_data.unlocks.unlocked_trials.clone(),
                                                        unlocked_pvp: connection.player_data.unlocks.unlocked_pvp.clone(),
                                                        cleared_raids: connection.player_data.unlocks.cleared_raids.clone(),
                                                        cleared_dungeons: connection.player_data.unlocks.cleared_dungeons.clone(),
                                                        cleared_guildhests: connection.player_data.unlocks.cleared_guildhests.clone(),
                                                        cleared_trials: connection.player_data.unlocks.cleared_trials.clone(),
                                                        cleared_pvp: connection.player_data.unlocks.cleared_pvp.clone(),
                                                        minions: vec![0xFFu8; MINION_BITMASK_SIZE], // TODO: make this persistent?
                                                        mount_guide_mask: vec![0xFFu8; MOUNT_BITMASK_SIZE], // TODO: make this persistent too?
                                                        homepoint: 8, // hardcoded to limsa for now
                                                        fav_aetheryte_count: 1,
                                                        favorite_aetheryte_ids: [8, 0, 0, 0],
                                                        seen_active_help: connection.player_data.unlocks.seen_active_help.clone(),
                                                        ..Default::default()
                                                    }));
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                connection.actor_control_self(ActorControlSelf {
                                                    category: ActorControlCategory::SetItemLevel {
                                                        level: connection.player_data.inventory.equipped.calculate_item_level() as u32,
                                                    }
                                                }).await;

                                                connection.send_quest_information().await;

                                                connection.handle.send(ToServer::ReadySpawnPlayer(connection.id, connection.player_data.zone_id, connection.player_data.position, connection.player_data.rotation)).await;

                                                let lua = lua.lock().unwrap();
                                                lua.scope(|scope| {
                                                    let connection_data =
                                                    scope.create_userdata_ref_mut(&mut lua_player).unwrap();

                                                    let func: Function = lua.globals().get("onBeginLogin").unwrap();

                                                    func.call::<()>(connection_data).unwrap();

                                                    Ok(())
                                                })
                                                .unwrap();
                                            }
                                            ClientZoneIpcData::FinishLoading { .. } => {
                                                let common = connection.get_player_common_spawn(connection.exit_position, connection.exit_rotation);

                                                let online_status = if connection.player_data.gm_rank == GameMasterRank::NormalUser {
                                                    OnlineStatus::Online
                                                } else {
                                                    OnlineStatus::GameMasterBlue
                                                };

                                                let spawn = PlayerSpawn {
                                                    account_id: connection.player_data.account_id,
                                                    content_id: connection.player_data.content_id,
                                                    current_world_id: config.world.world_id,
                                                    home_world_id: config.world.world_id,
                                                    gm_rank: connection.player_data.gm_rank,
                                                    online_status,
                                                    common: common.clone(),
                                                    ..Default::default()
                                                };

                                                // tell the server we loaded into the zone, so it can start sending us actors
                                                connection.handle.send(ToServer::ZoneLoaded(connection.id, connection.player_data.zone_id, spawn.clone())).await;

                                                let chara_details = database.find_chara_make(connection.player_data.content_id);

                                                connection.send_inventory(false).await;
                                                connection.send_stats(&chara_details).await;

                                                // send player spawn
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerSpawn(spawn));
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                // If a zone has any eobjs that need spawning (e.g. Chocobo Square), do so
                                                connection.spawn_eobjs(&mut lua_player).await;

                                                // wipe any exit position so it isn't accidentally reused
                                                connection.exit_position = None;
                                                connection.exit_rotation = None;
                                            }
                                            ClientZoneIpcData::ClientTrigger(trigger) => {
                                                // store the query for scripts
                                                if let ClientTriggerCommand::TeleportQuery { aetheryte_id } = trigger.trigger {
                                                    connection.player_data.teleport_query.aetheryte_id = aetheryte_id as u16;
                                                }

                                                match trigger.trigger {
                                                    ClientTriggerCommand::RequestTitleList {} => {
                                                        // send full title list for now

                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::TitleList {
                                                            unlock_bitmask: [0xFF; TITLE_UNLOCK_BITMASK_SIZE]
                                                        });
                                                        connection.send_ipc_self(ipc).await;
                                                    },
                                                    ClientTriggerCommand::FinishZoning {} => {
                                                        {
                                                            let lua = lua.lock().unwrap();
                                                            lua.scope(|scope| {
                                                                let connection_data =
                                                                scope.create_userdata_ref_mut(&mut lua_player).unwrap();

                                                                let func: Function = lua.globals().get("onFinishZoning").unwrap();

                                                                func.call::<()>(connection_data).unwrap();

                                                                Ok(())
                                                            })
                                                            .unwrap();
                                                        }

                                                        connection.handle.send(ToServer::ZoneIn(connection.id, connection.player_data.actor_id, connection.player_data.teleport_reason == TeleportReason::Aetheryte)).await;
                                                    },
                                                    ClientTriggerCommand::BeginContentsReplay {} => {
                                                        connection.conditions.set_condition(Condition::ContentsReplay);
                                                        connection.send_conditions().await;

                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::BeginContentsReplay {
                                                                unk1: 1
                                                            }
                                                        }).await;
                                                    },
                                                    ClientTriggerCommand::EndContentsReplay {} => {
                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::EndContentsReplay {
                                                                unk1: 1
                                                            }
                                                        }).await;

                                                        // TODO: de-duplicate from ClientZoneIpcData::FinishLoading
                                                        let common = connection.get_player_common_spawn(connection.exit_position, connection.exit_rotation);

                                                        let online_status = if connection.player_data.gm_rank == GameMasterRank::NormalUser {
                                                            OnlineStatus::Online
                                                        } else {
                                                            OnlineStatus::GameMasterBlue
                                                        };

                                                        let spawn = PlayerSpawn {
                                                            account_id: connection.player_data.account_id,
                                                            content_id: connection.player_data.content_id,
                                                            current_world_id: config.world.world_id,
                                                            home_world_id: config.world.world_id,
                                                            gm_rank: connection.player_data.gm_rank,
                                                            online_status,
                                                            common: common.clone(),
                                                            ..Default::default()
                                                        };

                                                        // send player spawn
                                                        {
                                                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerSpawn(spawn));
                                                            connection.send_ipc_self(ipc).await;
                                                        }

                                                        // TODO: clear the ContentsReplay flag instead of going nuclear (see also: event_finish)
                                                        connection.conditions = Conditions::default();
                                                        connection.send_conditions().await;
                                                    },
                                                    ClientTriggerCommand::Dismount { sequence } => {
                                                        connection.conditions = Conditions::default();
                                                        connection.send_conditions().await;

                                                        // TODO: not sure if it's important, retail sends an AC 2 with a param of 1

                                                        // Retail indeed does send an AC, not an ACS for this.
                                                        connection.actor_control(connection.player_data.actor_id, ActorControl {
                                                            category: ActorControlCategory::UnkDismountRelated { unk1: 47494, unk2: 32711, unk3: 1510381914 }
                                                        }).await;

                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::Dismount { sequence }
                                                        }).await;

                                                        // Then these are also sent!
                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::WalkInTriggerRelatedUnk1 { unk1: 0 }
                                                        }).await;

                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::CompanionUnlock { unk1: 0, unk2: 0 }
                                                        }).await;

                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::WalkInTriggerRelatedUnk2 {
                                                                unk1: 0,
                                                                unk2: 0,
                                                                unk3: 0,
                                                                unk4: 7,
                                                            }
                                                        }).await;
                                                    },
                                                    ClientTriggerCommand::ShownActiveHelp { id } => {
                                                        // Save this so it isn't shown again on next login
                                                        let (value, index) = value_to_flag_byte_index_value(id);
                                                        connection.player_data.unlocks.seen_active_help[index as usize] |= value;
                                                    }
                                                    _ => {
                                                        // inform the server of our trigger, it will handle sending it to other clients
                                                        connection.handle.send(ToServer::ClientTrigger(connection.id, connection.player_data.actor_id, trigger.clone())).await;
                                                    }
                                                }
                                            }
                                            ClientZoneIpcData::Unk3 { .. } => {
                                                // no-op
                                            }
                                            ClientZoneIpcData::Unk4 { .. } => {
                                                // no-op
                                            }
                                            ClientZoneIpcData::SetSearchInfoHandler { .. } => {
                                                tracing::info!("Recieved SetSearchInfoHandler!");
                                            }
                                            ClientZoneIpcData::Unk5 { .. } => {
                                                // no-op
                                            }
                                            ClientZoneIpcData::SocialListRequest(request) => {
                                                tracing::info!("Recieved social list request!");
                                                // TODO: store this in player_data, and also update it when in parties, duties, etc.
                                                let mut online_status_mask = OnlineStatusMask::default();
                                                online_status_mask.set_status(OnlineStatus::Online);

                                                match &request.request_type {
                                                    // TODO: Fill in with other party members once support for parties is implemented.
                                                    SocialListRequestType::Party => {
                                                        let chara_details = database.find_chara_make(connection.player_data.content_id);
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SocialList(SocialList {
                                                            request_type: request.request_type,
                                                            sequence: request.count,
                                                            entries: vec![PlayerEntry {
                                                                current_world_id: config.world.world_id,
                                                                ui_flags: SocialListUIFlags::ENABLE_CONTEXT_MENU,
                                                                content_id: connection.player_data.content_id,
                                                                zone_id: connection.player_data.zone_id,
                                                                social_ui_languages: SocialListUILanguages::ENGLISH, // TODO: These languages and the primary client language seem to be set in the search info, but that is not yet implemented.
                                                                client_language: ClientLanguage::English,
                                                                online_status_mask,
                                                                home_world_id: config.world.world_id,
                                                                name: chara_details.name.to_string(),
                                                                classjob_id: connection.player_data.classjob_id,
                                                                classjob_level: connection.player_data.classjob_levels[connection.player_data.classjob_id as usize] as u8,
                                                                ..Default::default()
                                                            },],
                                                        }));
                                                        connection.send_ipc_self(ipc).await;
                                                    }
                                                    SocialListRequestType::Friends => {
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SocialList(SocialList {
                                                            request_type: request.request_type,
                                                            sequence: request.count,
                                                            entries: Default::default(),
                                                        }));
                                                        connection.send_ipc_self(ipc).await;
                                                    }
                                                }
                                            }
                                            ClientZoneIpcData::UpdatePositionHandler { position, rotation, anim_type, anim_state, jump_state, } => {
                                                connection.player_data.rotation = *rotation;
                                                connection.player_data.position = *position;

                                                connection.handle.send(ToServer::ActorMoved(connection.id, connection.player_data.actor_id, *position, *rotation, *anim_type, *anim_state, *jump_state)).await;
                                            }
                                            ClientZoneIpcData::LogOut { .. } => {
                                                tracing::info!("Recieved log out from client!");

                                                connection.gracefully_logged_out = true;
                                                connection.begin_log_out().await;
                                            }
                                            ClientZoneIpcData::Disconnected { .. } => {
                                                tracing::info!("Client disconnected!");

                                                connection.handle.send(ToServer::Disconnected(connection.id)).await;

                                                break;
                                            }
                                            ClientZoneIpcData::SendChatMessage(chat_message) => {
                                                let chara_details = database.find_chara_make(connection.player_data.content_id);
                                                let info = MessageInfo {
                                                    sender_actor_id: connection.player_data.actor_id,
                                                    sender_account_id: connection.player_data.account_id,
                                                    sender_world_id: config.world.world_id,
                                                    sender_position: connection.player_data.position,
                                                    sender_name: chara_details.name,
                                                    channel: chat_message.channel,
                                                    message: chat_message.message.clone(),
                                                };

                                                connection.handle.send(ToServer::Message(connection.id, info)).await;

                                                let mut handled = false;
                                                let command_trigger: char = '!';
                                                if chat_message.message.starts_with(command_trigger)
                                                {
                                                    let parts: Vec<&str> = chat_message.message.split(' ').collect();
                                                    let command_name = &parts[0][1..];

                                                    {
                                                        let lua = lua.lock().unwrap();
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
                                                                    .create_userdata_ref_mut(&mut lua_player)?;

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

                                                                    if connection.player_data.gm_rank as u8 >= required_rank? {
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
                                                                        connection.player_data.account_id, command_name);
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
                                                            &mut connection,
                                                            chat_message,
                                                        )
                                                        .await;
                                                    }

                                                    // If it's truly not existent:
                                                    if !handled {
                                                        tracing::info!("Unknown command {command_name}");

                                                        let lua = lua.lock().unwrap();

                                                        let mut call_func = || {
                                                            lua.scope(|scope| {
                                                                let connection_data = scope
                                                                .create_userdata_ref_mut(&mut lua_player)?;
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
                                            ClientZoneIpcData::GMCommand { command, arg0, arg1, arg2, arg3, .. } => {
                                                connection.run_gm_command(*command, *arg0, *arg1, *arg2, *arg3, &mut lua_player).await;
                                            }
                                            ClientZoneIpcData::GMCommandName { command, arg0, arg1, arg2, arg3, .. } => {
                                                connection.run_gm_command(*command, *arg0, *arg1, *arg2, *arg3, &mut lua_player).await;
                                            }
                                            ClientZoneIpcData::ZoneJump {
                                                exit_box,
                                                position,
                                                ..
                                            } => {
                                                tracing::info!(
                                                    "Character entered {exit_box} with a position of {position:#?}!"
                                                );

                                                connection.handle.send(ToServer::EnterZoneJump(connection.id, connection.player_data.actor_id, *exit_box)).await;
                                            }
                                            ClientZoneIpcData::ActionRequest(request) => {
                                                connection
                                                    .handle
                                                    .send(ToServer::ActionRequest(
                                                        connection.id,
                                                        connection.player_data.actor_id,
                                                        request.clone(),
                                                    ))
                                                    .await;
                                            }
                                            ClientZoneIpcData::Unk16 { .. } => {
                                                // no-op
                                            }
                                            ClientZoneIpcData::PingSync { timestamp, .. } => {
                                                // this is *usually* sent in response, but not always
                                                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PingSyncReply {
                                                    timestamp: *timestamp, // copied from here
                                                    transmission_interval: 333, // always this for some reason
                                                });
                                                connection.send_ipc_self(ipc).await;
                                            }
                                            ClientZoneIpcData::Unk18 { .. } => {
                                                // no-op
                                            }
                                            ClientZoneIpcData::EventRelatedUnk {
                                                unk1,
                                                unk2,
                                                unk3,
                                                unk4,
                                            } => {
                                                tracing::info!(
                                                    "Recieved EventRelatedUnk! {unk1} {unk2} {unk3} {unk4}"
                                                );

                                                if let Some(event) = connection.event.as_mut() {
                                                    event.scene_finished(&mut lua_player, *unk2);
                                                }
                                            }
                                            ClientZoneIpcData::Unk19 { .. } => {
                                                // no-op
                                            }
                                            ClientZoneIpcData::ItemOperation(action) => {
                                                tracing::info!("Client is modifying inventory! {action:#?}");
                                                connection.send_inventory_ack(action.context_id, INVENTORY_ACTION_ACK_GENERAL as u16).await;

                                                connection.player_data.inventory.process_action(action);

                                                // If the client modified their equipped items, we have to process that
                                                if action.src_storage_id == ContainerType::Equipped || action.dst_storage_id == ContainerType::Equipped {
                                                    connection.inform_equip().await;
                                                }

                                                if action.operation_type == ItemOperationKind::Discard {
                                                    tracing::info!("Client is discarding from their inventory!");

                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                                                        sequence: connection.player_data.item_sequence,
                                                        operation_type: action.operation_type,
                                                        src_actor_id: connection.player_data.actor_id,
                                                        src_storage_id: action.src_storage_id,
                                                        src_container_index: action.src_container_index,
                                                        src_stack: action.src_stack,
                                                        src_catalog_id: action.src_catalog_id,
                                                        dst_actor_id: INVALID_OBJECT_ID.0,
                                                        dummy_container: ContainerType::DiscardingItemSentinel,
                                                        dst_storage_id: ContainerType::DiscardingItemSentinel,
                                                        dst_container_index: u16::MAX,
                                                        dst_stack: 0,
                                                        dst_catalog_id: 0,
                                                    });
                                                    connection.send_ipc_self(ipc).await;
                                                    connection.send_inventory_transaction_finish(0x90, 0x200).await;
                                                }

                                                connection.player_data.item_sequence += 1;
                                            }
                                            // TODO: Likely rename this opcode if non-gil shops also use this same opcode
                                            ClientZoneIpcData::GilShopTransaction { event_id, unk1: _, buy_sell_mode, item_index, item_quantity, unk2: _ } => {
                                                tracing::info!("Client is interacting with a shop! {event_id:#?} {buy_sell_mode:#?} {item_quantity:#?} {item_index:#?}");
                                                const BUY:  u32 = 1;
                                                const SELL: u32 = 2;

                                                if *buy_sell_mode == BUY {
                                                    let result;
                                                    {
                                                        let mut game_data = connection.gamedata.lock().unwrap();
                                                        result = game_data.get_gilshop_item(*event_id, *item_index as u16);
                                                    }

                                                    if let Some(item_info) = result {
                                                        if connection.player_data.inventory.currency.gil.quantity >= *item_quantity * item_info.price_mid {
                                                            if let Some(add_result) = connection.player_data.inventory.add_in_next_free_slot(Item::new(item_info.clone(), *item_quantity)) {
                                                                connection.player_data.inventory.currency.gil.quantity -= *item_quantity * item_info.price_mid;
                                                                connection.send_gilshop_item_update(ContainerType::Currency as u16, 0, connection.player_data.inventory.currency.gil.quantity, CurrencyKind::Gil as u32).await;

                                                                connection.send_inventory_ack(u32::MAX, INVENTORY_ACTION_ACK_SHOP as u16).await;

                                                                connection.send_gilshop_item_update(add_result.container as u16, add_result.index, add_result.quantity, item_info.id).await;
                                                                connection.send_gilshop_ack(*event_id, item_info.id, *item_quantity, item_info.price_mid, LogMessageType::ItemBought).await;

                                                                let target_id = connection.player_data.target_actorid;
                                                                // See GenericShopkeeper.lua for information about this scene, the flags, and the params.
                                                                connection.event_scene(&target_id, *event_id, 10, 8193, vec![1, 100]).await;
                                                            } else {
                                                                tracing::error!(ERR_INVENTORY_ADD_FAILED);
                                                                connection.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                                                                connection.event_finish(*event_id, 0, EventFinishType::Normal).await;
                                                            }
                                                        } else {
                                                            connection.send_notice("Insufficient gil to buy item. Nice try bypassing the client-side check!").await;
                                                            connection.event_finish(*event_id, 0, EventFinishType::Normal).await;
                                                        }
                                                    } else {
                                                        connection.send_notice("Unable to find shop item, this is a bug in Kawari!").await;
                                                        connection.event_finish(*event_id, 0, EventFinishType::Normal).await;
                                                    }
                                                } else if *buy_sell_mode == SELL {
                                                    let storage = get_container_type(*item_index).unwrap();
                                                    let index = *item_quantity;
                                                    let result;
                                                    let quantity;
                                                    {
                                                        let item = connection.player_data.inventory.get_item(storage, index as u16);
                                                        let mut game_data = connection.gamedata.lock().unwrap();
                                                        result = game_data.get_item_info(ItemInfoQuery::ById(item.id));
                                                        quantity = item.quantity;
                                                    }

                                                    if let Some(item_info) = result {
                                                        let bb_item = BuyBackItem {
                                                            id: item_info.id,
                                                            quantity,
                                                            price_low: item_info.price_low,
                                                            item_level: item_info.item_level,
                                                            stack_size: item_info.stack_size,
                                                        };
                                                        connection.player_data.buyback_list.push_item(*event_id, bb_item);

                                                        connection.player_data.inventory.currency.gil.quantity += quantity * item_info.price_low;
                                                        connection.send_gilshop_item_update(ContainerType::Currency as u16, 0, connection.player_data.inventory.currency.gil.quantity, CurrencyKind::Gil as u32).await;
                                                        connection.send_gilshop_item_update(storage as u16, index as u16, 0, 0).await;

                                                        // TODO: Refactor InventoryTransactions into connection.rs
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                                                            sequence: connection.player_data.item_sequence,
                                                            operation_type: ItemOperationKind::Update,
                                                            src_actor_id: connection.player_data.actor_id,
                                                            src_storage_id: ContainerType::Currency,
                                                            src_container_index: 0,
                                                            src_stack: connection.player_data.inventory.currency.gil.quantity,
                                                            src_catalog_id: CurrencyKind::Gil as u32,
                                                            dst_actor_id: INVALID_OBJECT_ID.0,
                                                            dummy_container: ContainerType::DiscardingItemSentinel,
                                                            dst_storage_id: ContainerType::DiscardingItemSentinel,
                                                            dst_container_index: u16::MAX,
                                                            dst_stack: 0,
                                                            dst_catalog_id: 0,
                                                        });
                                                        connection.send_ipc_self(ipc).await;

                                                        // Process the server's inventory first.
                                                        let action = ItemOperation {
                                                            operation_type: ItemOperationKind::Discard,
                                                            src_storage_id: storage,
                                                            src_container_index: index as u16,
                                                            ..Default::default()
                                                        };

                                                        connection.player_data.inventory.process_action(&action);

                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                                                            sequence: connection.player_data.item_sequence,
                                                            operation_type: ItemOperationKind::Discard,
                                                            src_actor_id: connection.player_data.actor_id,
                                                            src_storage_id: storage,
                                                            src_container_index: index as u16,
                                                            src_stack: quantity,
                                                            src_catalog_id: item_info.id,
                                                            dst_actor_id: INVALID_OBJECT_ID.0,
                                                            dummy_container: ContainerType::DiscardingItemSentinel,
                                                            dst_storage_id: ContainerType::DiscardingItemSentinel,
                                                            dst_container_index: u16::MAX,
                                                            dst_stack: 0,
                                                            dst_catalog_id: 0,
                                                        });
                                                        connection.send_ipc_self(ipc).await;

                                                        connection.send_inventory_transaction_finish(0x100, 0x300).await;

                                                        connection.send_gilshop_ack(*event_id, item_info.id, quantity, item_info.price_low, LogMessageType::ItemSold).await;

                                                        let target_id = connection.player_data.target_actorid;

                                                        let mut params = connection.player_data.buyback_list.as_scene_params(*event_id, false);
                                                        params[0] = SELL;
                                                        params[1] = 0; // The "terminator" is 0 for sell mode.
                                                        connection.event_scene(&target_id, *event_id, 10, 8193, params).await;
                                                    } else {
                                                        connection.send_notice("Unable to find shop item, this is a bug in Kawari!").await;
                                                        connection.event_finish(*event_id, 0, EventFinishType::Normal).await;
                                                    }
                                                } else {
                                                    tracing::error!("Received unknown transaction mode {buy_sell_mode}!");
                                                    connection.event_finish(*event_id, 0, EventFinishType::Normal).await;
                                                }
                                            }
                                            ClientZoneIpcData::StartTalkEvent { actor_id, event_id } => {
                                                connection.start_event(*actor_id, *event_id, 1, 0).await;

                                                /* TODO: ServerZoneIpcType::Unk18 with data [64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
                                                    * was observed to always be sent by the server upon interacting with shops. They open and function fine without
                                                    * it, but should we send it anyway, for the sake of accuracy? It's also still unclear if this
                                                    * happens for -every- NPC/actor. */

                                                // begin talk function if it exists
                                                if let Some(event) = connection.event.as_mut() {
                                                     event.talk(*actor_id, &mut lua_player);
                                                }
                                            }
                                            ClientZoneIpcData::EventYieldHandler(handler) => {
                                                tracing::info!(message = "Event yielded", handler_id = handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                                                connection
                                                .event
                                                    .as_mut()
                                                    .unwrap()
                                                    .finish(handler.scene, &handler.params[..handler.num_results as usize], &mut lua_player);
                                            }
                                            ClientZoneIpcData::EventYieldHandler8(handler) => {
                                                tracing::info!(message = "Event yielded", handler_id = handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                                                connection
                                                    .event
                                                    .as_mut()
                                                    .unwrap()
                                                    .finish(handler.scene, &handler.params[..handler.num_results as usize], &mut lua_player);
                                            }
                                            ClientZoneIpcData::Config(config) => {
                                                // Update our own state so it's committed on log out
                                                connection.player_data.display_flags = config.display_flag;
                                                connection
                                                    .handle
                                                    .send(ToServer::Config(
                                                        connection.id,
                                                        connection.player_data.actor_id,
                                                        config.clone(),
                                                    ))
                                                    .await;
                                            }
                                            ClientZoneIpcData::StandardControlsPivot { .. } => {
                                                /* No-op because we already seem to handle this, other nearby clients can see the sending player
                                                 * pivoting anyway. */
                                            }
                                            ClientZoneIpcData::EventUnkRequest { event_id, unk1, unk2, unk3 } => {
                                                 let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventUnkReply {
                                                     event_id: *event_id,
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
                                            ClientZoneIpcData::ContentFinderRegister { content_ids, .. } => {
                                                tracing::info!("Searching for {content_ids:?}");

                                                connection.queued_content = Some(content_ids[0]);

                                                // update
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContentFinderUpdate {
                                                        state1: 1,
                                                        classjob_id: connection.player_data.classjob_id, // TODO: store what they registered with, because it can change
                                                        unk1: [
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            96,
                                                            4,
                                                            2,
                                                            64,
                                                            1,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            1,
                                                            1,
                                                        ],
                                                        content_ids: *content_ids,
                                                        unk2: [
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                        ],
                                                    });
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                // found
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContentFinderFound {
                                                        unk1: [
                                                            3,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            96,
                                                            4,
                                                            2,
                                                            64,
                                                            1,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            1,
                                                            0,
                                                            0,
                                                            0,
                                                        ],
                                                        content_id: content_ids[0],
                                                        unk2: [
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            1,
                                                        ],
                                                    });
                                                    connection.send_ipc_self(ipc).await;
                                                }
                                            }
                                            ClientZoneIpcData::ContentFinderAction { unk1 } => {
                                                dbg!(unk1);

                                                // commencing
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContentFinderCommencing {
                                                        unk1: [
                                                            4,
                                                            0,
                                                            0,
                                                            0,
                                                            1,
                                                            0,
                                                            0,
                                                            0,
                                                            4,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                            1,
                                                            1,
                                                            0,
                                                            0,
                                                            0,
                                                            0,
                                                        ],
                                                    });
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                // TODO: content finder should be moved to global state
                                                // For now, just send them to do the zone if they do anything
                                                let zone_id;
                                                {
                                                    let mut game_data = game_data.lock().unwrap();
                                                    zone_id = game_data.find_zone_for_content(connection.queued_content.unwrap());
                                                }

                                                if let Some(zone_id) = zone_id {
                                                    connection.change_zone(zone_id).await;
                                                } else {
                                                    tracing::warn!("Failed to find zone id for content?!");
                                                }

                                                connection.queued_content = None;
                                            }
                                            ClientZoneIpcData::EquipGearset { .. } => {
                                                tracing::info!("Client tried to equip a gearset!");
                                                connection.send_notice("Gearsets are not yet implemented.").await;
                                            }
                                            ClientZoneIpcData::StartWalkInEvent { event_arg, event_id, .. } => {
                                                // Yes, an ActorControl is sent here, not an ActorControlSelf!
                                                connection.actor_control(connection.player_data.actor_id, ActorControl {
                                                    category: ActorControlCategory::ToggleWeapon {
                                                        shown: false,
                                                        unk_flag: 1,
                                                    }
                                                }).await;
                                                connection.conditions.set_condition(Condition::WalkInEvent);
                                                connection.send_conditions().await;
                                                let actor_id = ObjectTypeId { object_id: ObjectId(connection.player_data.actor_id), object_type: ObjectTypeKind::None };
                                                connection.start_event(actor_id, *event_id, 10, *event_arg).await;

                                                // begin walk-in trigger function if it exists
                                                if let Some(event) = connection.event.as_mut() {
                                                     event.enter_trigger(&mut lua_player);
                                                }
                                            }
                                            ClientZoneIpcData::NewDiscovery { layout_id, pos } => {
                                                tracing::info!("Client discovered a new location on {:?} at {:?}!", layout_id, pos);

                                                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LocationDiscovered {
                                                    map_part_id: 162, // I'm not sure what this?
                                                    map_id: lua_player.zone_data.map_id as u32,
                                                });
                                                connection.send_ipc_self(ipc).await;
                                            }
                                            ClientZoneIpcData::RequestBlacklist(request) => {
                                                // TODO: Actually implement this beyond simply sending a blank list
                                                // NOTE: Failing to respond to this request means PlayerSpawn will not work and other players will be invisible, have their chat ignored and possibly other issues by the client! Beware!
                                                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Blacklist(Blacklist {
                                                    data: vec![BlacklistedCharacter::default(); Blacklist::NUM_ENTRIES],
                                                    sequence: request.sequence,
                                                }));
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
                                            ClientZoneIpcData::StartCountdown { .. } => {
                                                tracing::info!("Countdowns is unimplemented");
                                            }
                                            ClientZoneIpcData::RequestPlaytime { .. } => {
                                                tracing::info!("Playtime is unimplemented");
                                            }
                                            ClientZoneIpcData::Unknown { unk } => {
                                                tracing::warn!("Unknown Zone packet {:?} recieved ({} bytes), this should be handled!", data.header.op_code, unk.len());
                                            }
                                        }
                                    }
                                    SegmentData::KeepAliveRequest { id, timestamp } => {
                                        send_keep_alive::<ServerZoneIpcSegment>(
                                            &mut connection.socket,
                                            &mut connection.state,
                                            ConnectionType::Zone,
                                            *id,
                                            *timestamp,
                                        )
                                        .await
                                    }
                                    SegmentData::KeepAliveResponse { .. } => {
                                        // these should be safe to ignore
                                    }
                                    _ => {
                                        panic!("ZoneConnection: The server is recieving a response or unknown packet: {segment:#?}")
                                    }
                                }
                            }

                            // copy from lua player state, as they modify the status effects list
                            // TODO: i dunno?
                            connection.status_effects = lua_player.status_effects.clone();

                            // Process any queued packets from scripts and whatnot
                            connection.process_lua_player(&mut lua_player).await;

                            // check if status effects need sending
                            connection.process_effects_list().await;

                            // update lua player
                            lua_player.player_data = connection.player_data.clone();
                            lua_player.status_effects = connection.status_effects.clone();
                        }
                    },
                    Err(_) => {
                        tracing::info!("Connection {:#?} was killed because of a network error!", client_handle.id);
                        break;
                    },
                }
            }
            msg = internal_recv.recv() => match msg {
                Some(msg) => match msg {
                    FromServer::Message(msg) => connection.send_message(msg).await,
                    FromServer::ActorSpawn(actor, spawn) => connection.spawn_actor(actor, spawn).await,
                    FromServer::ActorMove(actor_id, position, rotation, anim_type, anim_state, jump_state) => connection.set_actor_position(actor_id, position, rotation, anim_type, anim_state, jump_state).await,
                    FromServer::ActorDespawn(actor_id) => connection.remove_actor(actor_id).await,
                    FromServer::ActorControl(actor_id, actor_control) => connection.actor_control(actor_id, actor_control).await,
                    FromServer::ActorControlTarget(actor_id, actor_control) => connection.actor_control_target(actor_id, actor_control).await,
                    FromServer::ActorControlSelf(actor_control) => connection.actor_control_self(actor_control).await,
                    FromServer::ActorSummonsMinion(minion_id) => {
                        connection.handle.send(ToServer::ActorSummonsMinion(connection.id, connection.player_data.actor_id, minion_id)).await;
                        connection.player_data.active_minion = minion_id;
                    }
                    FromServer::ActorDespawnsMinion() => {
                        connection.handle.send(ToServer::ActorDespawnsMinion(connection.id, connection.player_data.actor_id)).await;
                        connection.player_data.active_minion = 0;
                    }
                    FromServer::ActionComplete(request) => connection.execute_action(request, &mut lua_player).await,
                    FromServer::ActionCancelled() => connection.cancel_action().await,
                    FromServer::UpdateConfig(actor_id, config) => connection.update_config(actor_id, config).await,
                    FromServer::ActorEquip(actor_id, main_weapon_id, sub_weapon_id, model_ids) => connection.update_equip(actor_id, main_weapon_id, sub_weapon_id, model_ids).await,
                    FromServer::ReplayPacket(segment) => connection.send_segment(segment).await,
                    FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id) => connection.lose_effect(effect_id, effect_param, effect_source_actor_id, &mut lua_player).await,
                    FromServer::Conditions(conditions) => {
                        connection.conditions = conditions;
                        connection.send_conditions().await;
                    },
                    FromServer::ChangeZone(zone_id, weather_id, position, rotation, lua_zone, initial_login) => {
                        lua_player.zone_data = lua_zone;
                        connection.handle_zone_change(zone_id, weather_id, position, rotation, initial_login).await;
                    },
                },
                None => break,
            }
        }
    }

    // forcefully log out the player if they weren't logging out but force D/C'd
    if connection.player_data.actor_id != 0 {
        if !connection.gracefully_logged_out {
            tracing::info!(
                "Forcefully logging out connection {:#?}...",
                client_handle.id
            );
            connection.begin_log_out().await;
        }
        connection
            .handle
            .send(ToServer::Disconnected(connection.id))
            .await;
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.world.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    tracing::info!("Server started on {addr}");

    let database = Arc::new(WorldDatabase::new());
    let lua = Arc::new(Mutex::new(Lua::new()));
    let game_data = Arc::new(Mutex::new(GameData::new()));

    {
        let mut lua = lua.lock().unwrap();
        if let Err(err) = load_init_script(&mut lua) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }

    let (handle, _) = spawn_main_loop();

    loop {
        if let Ok((socket, ip)) = listener.accept().await {
            let id = handle.next_id();

            spawn_initial_setup(
                ip,
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
