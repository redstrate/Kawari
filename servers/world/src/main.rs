use std::sync::Arc;
use std::time::Instant;

use kawari::common::{
    ClientLanguage, ContainerType, DirectorEvent, DirectorTrigger, EventHandlerType, GameData,
    INVALID_OBJECT_ID, ItemOperationKind, ObjectId, ObjectTypeId, ObjectTypeKind,
    calculate_max_level,
};
use kawari::config::get_config;
use kawari_world::inventory::{Item, Storage, get_next_free_slot};

use kawari::ipc::chat::{ChatChannel, ClientChatIpcData};

use kawari::ipc::zone::{
    ActorControl, ActorControlCategory, ActorControlSelf, Condition, Conditions,
    ContentFinderUserAction, EventType, InviteType, OnlineStatus, OnlineStatusMask, PlayerSpawn,
    PlayerStatus, SearchInfo, TrustContent, TrustInformation,
};

use kawari::ipc::zone::{
    Blacklist, BlacklistedCharacter, ClientTriggerCommand, ClientZoneIpcData, GameMasterRank,
    ServerZoneIpcData, ServerZoneIpcSegment,
};

use kawari::common::{NETWORK_TIMEOUT, RECEIVE_BUFFER_SIZE};
use kawari::constants::{
    AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, CLASSJOB_ARRAY_SIZE, TITLE_UNLOCK_BITMASK_SIZE,
};
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{
    ConnectionState, ConnectionType, PacketSegment, SegmentData, SegmentType, parse_packet_header,
};
use kawari_world::lua::{ExtraLuaState, LuaPlayer, load_init_script};
use kawari_world::{
    ChatConnection, ChatHandler, CustomIpcConnection, ObsfucationData, TeleportReason,
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
    let last_keep_alive = Instant::now();

    match socket.read(&mut buf).await {
        Ok(n) => {
            // if the last response was over >5 seconds, the client is probably gone
            if n == 0 {
                let now = Instant::now();
                if now.duration_since(last_keep_alive) > NETWORK_TIMEOUT {
                    tracing::info!(
                        "initial_setup: Connection was killed because of timeout or they are now handled by the proper connection type"
                    );
                }
            } else {
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
                            SegmentData::KawariIpc(data) => {
                                connection.handle_custom_ipc(data).await
                            }
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
                    };

                    // Handle setup before passing off control to the zone connection.
                    let segments = connection.parse_packet(&buf[..n]);
                    for segment in segments {
                        match &segment.data {
                            SegmentData::Setup { actor_id } => {
                                // for some reason they send a string representation
                                let actor_id = actor_id.parse::<u32>().unwrap();

                                // initialize player data if it doesn't exist
                                if !connection.player_data.actor_id.is_valid() {
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
        }
        Err(_) => {
            tracing::info!("initial_setup: Connection was killed because of a network error!");
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
    let actor_id = &connection.player_data.actor_id.clone();

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

async fn client_loop(
    mut connection: ZoneConnection,
    mut internal_recv: UnboundedReceiver<FromServer>,
    client_handle: ClientHandle,
) {
    let database = connection.database.clone();
    let lua = connection.lua.clone();
    let config = get_config();

    let mut lua_player = LuaPlayer::default();

    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    let mut client_handle = client_handle.clone();
    client_handle.actor_id = connection.player_data.actor_id;

    // Do an initial update otherwise it may be uninitialized for the first packet that needs Lua
    lua_player.player_data = connection.player_data.clone();

    // tell the server we exist, now that we confirmed we are a legitimate connection
    connection
        .handle
        .send(ToServer::NewClient(client_handle.clone()))
        .await;

    'outer: loop {
        tokio::select! {
            biased; // client data should always be prioritized
            n = connection.socket.read(&mut buf) => {
                match n {
                    Ok(n) => {
                        // if the last response was over >5 seconds, the client is probably gone
                        if n == 0 {
                            let now = Instant::now();
                            if now.duration_since(connection.last_keep_alive) > NETWORK_TIMEOUT {
                                tracing::info!("ZoneConnection {:#?} was killed because of timeout", client_handle.id);
                                break;
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
                                                tracing::info!(
                                                    "Client is now requesting zone information. Sending!"
                                                );

                                                // IPC Init(?)
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InitResponse {
                                                        actor_id: connection.player_data.actor_id,
                                                    });
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                let service_account_id;
                                                {
                                                    let mut database = database.lock();
                                                    service_account_id = database.find_service_account(connection.player_data.content_id);
                                                }

                                                let Ok(mut login_reply) = ureq::get(format!(
                                                    "{}/_private/max_ex?service={}",
                                                    config.login.server_name, service_account_id,
                                                )).call()
                                                else {
                                                    tracing::warn!(
                                                        "Failed to find service account {service_account_id}, just going to stop talking to this connection..."
                                                    );
                                                    break 'outer; // We break the outer loop here because we're in the middle of a segment loop!
                                                };

                                                let expansion = login_reply.body_mut().read_to_string().unwrap().parse().unwrap();
                                                // Send inventory
                                                connection.send_inventory().await;

                                                // set equip display flags
                                                connection
                                                .actor_control_self(ActorControlSelf {
                                                    category: ActorControlCategory::SetEquipDisplayFlags {
                                                        display_flag: connection.player_data.display_flags
                                                    },
                                                })
                                                .await;

                                                // Stats
                                                connection.send_stats().await;

                                                // As seen in retail, they pad it with the first value
                                                let mut padded_exp = connection.player_data.classjob_exp.clone();
                                                padded_exp.resize(CLASSJOB_ARRAY_SIZE, connection.player_data.classjob_exp[0]);

                                                // Ditto for levels
                                                let mut padded_levels = connection.player_data.classjob_levels.clone();
                                                padded_levels.resize(CLASSJOB_ARRAY_SIZE, connection.player_data.classjob_levels[0]);

                                                let chara_make;
                                                let city_state;
                                                {
                                                    let mut database = database.lock();
                                                    chara_make = database.get_chara_make(connection.player_data.content_id);
                                                    city_state = database.get_city_state(connection.player_data.content_id);
                                                }

                                                // Player Setup
                                                {
                                                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerStatus(PlayerStatus {
                                                        content_id: connection.player_data.content_id,
                                                        exp: padded_exp,
                                                        max_level: calculate_max_level(expansion),
                                                        expansion,
                                                        name: connection.player_data.name.clone(),
                                                        actor_id: connection.player_data.actor_id,
                                                        race: chara_make.customize.race,
                                                        gender: chara_make.customize.gender,
                                                        tribe: chara_make.customize.subrace,
                                                        city_state,
                                                        nameday_month: chara_make.birth_month
                                                        as u8,
                                                        nameday_day: chara_make.birth_day as u8,
                                                        deity: chara_make.guardian as u8,
                                                        current_class: connection.player_data.classjob_id,
                                                        first_class: connection.player_data.classjob_id, // TODO: placeholder
                                                        levels: padded_levels,
                                                        unlocks: connection.player_data.unlock.unlocks.0.clone(),
                                                        aetherytes: connection.player_data.aetheryte.unlocked.0.clone(),
                                                        unlocked_raids: connection.player_data.content.unlocked_raids.0.clone(),
                                                        unlocked_dungeons: connection.player_data.content.unlocked_dungeons.0.clone(),
                                                        unlocked_guildhests: connection.player_data.content.unlocked_guildhests.0.clone(),
                                                        unlocked_trials: connection.player_data.content.unlocked_trials.0.clone(),
                                                        unlocked_crystalline_conflict: connection.player_data.content.unlocked_crystalline_conflicts.0.clone(),
                                                        unlocked_frontline: connection.player_data.content.unlocked_frontlines.0.clone(),
                                                        cleared_raids: connection.player_data.content.cleared_raids.0.clone(),
                                                        cleared_dungeons: connection.player_data.content.cleared_dungeons.0.clone(),
                                                        cleared_guildhests: connection.player_data.content.cleared_guildhests.0.clone(),
                                                        cleared_trials: connection.player_data.content.cleared_trials.0.clone(),
                                                        cleared_crystalline_conflict: connection.player_data.content.cleared_crystalline_conflicts.0.clone(),
                                                        cleared_frontline: connection.player_data.content.cleared_frontlines.0.clone(),
                                                        minions: connection.player_data.unlock.minions.0.clone(),
                                                        mount_guide_mask: connection.player_data.unlock.mounts.0.clone(),
                                                        home_aetheryte_id: 8, // hardcoded to limsa for now
                                                        favourite_aetheryte_count: 1,
                                                        favorite_aetheryte_ids: [8, 0, 0, 0],
                                                        seen_active_help: connection.player_data.unlock.seen_active_help.0.clone(),
                                                        aether_currents_mask: connection.player_data.aether_current.unlocked.0.clone(),
                                                        orchestrion_roll_mask: connection.player_data.unlock.orchestrion_rolls.0.clone(),
                                                        buddy_equip_mask: connection.player_data.companion.unlocked_equip.0.clone(),
                                                        cutscene_seen_mask: connection.player_data.unlock.cutscene_seen.0.clone(),
                                                        ornament_mask: connection.player_data.unlock.ornaments.0.clone(),
                                                        caught_fish_mask: connection.player_data.unlock.caught_fish.0.clone(),
                                                        caught_spearfish_mask: connection.player_data.unlock.caught_spearfish.0.clone(),
                                                        adventure_mask: connection.player_data.unlock.adventures.0.clone(),
                                                        triple_triad_cards: connection.player_data.unlock.triple_triad_cards.0.clone(),
                                                        glasses_styles_mask: connection.player_data.unlock.glasses_styles.0.clone(),
                                                        chocobo_taxi_stands_mask: connection.player_data.unlock.chocobo_taxi_stands.0.clone(),
                                                        aether_current_comp_flg_set_bitmask1: connection.player_data.aether_current.comp_flg_set.0[0],
                                                        aether_current_comp_flg_set_bitmask2: connection.player_data.aether_current.comp_flg_set.0[1..AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE].to_vec(),
                                                        ..Default::default()
                                                    }));
                                                    connection.send_ipc_self(ipc).await;
                                                }

                                                connection.actor_control_self(ActorControlSelf {
                                                    category: ActorControlCategory::SetItemLevel {
                                                        level: connection.player_data.inventory.equipped.calculate_item_level() as u32,
                                                    }
                                                }).await;

                                                connection.handle.send(ToServer::ReadySpawnPlayer(connection.id, connection.player_data.actor_id, connection.player_data.zone_id, connection.player_data.position, connection.player_data.rotation)).await;

                                                let lua = lua.lock();
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
                                                let common = connection.get_player_common_spawn(connection.exit_position, connection.exit_rotation, true);

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
                                                connection.handle.send(ToServer::ZoneLoaded(connection.id, connection.player_data.actor_id, spawn.clone())).await;

                                                // If we're in a party, we need to tell the other members we changed areas or reconnected.
                                                if connection.is_in_party() {
                                                    if !connection.player_data.rejoining_party {
                                                    connection.handle.send(ToServer::PartyMemberChangedAreas(connection.player_data.party_id, connection.player_data.account_id, connection.player_data.content_id, connection.player_data.name.clone())).await;
                                                    } else {
                                                        connection.handle.send(ToServer::PartyMemberReturned(connection.player_data.actor_id)).await;
                                                        connection.player_data.rejoining_party = false;
                                                    }
                                                }

                                                connection.send_stats().await;

                                                connection.respawn_player(true).await;

                                                // wipe any exit position so it isn't accidentally reused
                                                connection.exit_position = None;
                                                connection.exit_rotation = None;
                                            }
                                            ClientZoneIpcData::ClientTrigger(trigger) => {
                                                match trigger.trigger {
                                                    ClientTriggerCommand::RequestTitleList {} => {
                                                        // send full title list for now

                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::TitleList {
                                                            unlock_bitmask: [0xFF; TITLE_UNLOCK_BITMASK_SIZE]
                                                        });
                                                        connection.send_ipc_self(ipc).await;
                                                    },
                                                    ClientTriggerCommand::FinishZoning {} => {
                                                        connection.handle.send(ToServer::ZoneIn(connection.id, connection.player_data.actor_id, connection.player_data.teleport_reason == TeleportReason::Aetheryte)).await;
                                                    },
                                                    ClientTriggerCommand::BeginContentsReplay {} => {
                                                        connection.conditions.set_condition(Condition::ExecutingGatheringAction);
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

                                                        connection.respawn_player(false).await;

                                                        connection.conditions.remove_condition(Condition::ExecutingGatheringAction);
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
                                                            category: ActorControlCategory::SetPetEntityId { unk1: 0 }
                                                        }).await;

                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::CompanionUnlock { unk1: 0, unk2: 0 }
                                                        }).await;

                                                        connection.actor_control_self(ActorControlSelf {
                                                            category: ActorControlCategory::SetPetParameters {
                                                                pet_id: 0,
                                                                unk2: 0,
                                                                unk3: 0,
                                                                unk4: 7,
                                                            }
                                                        }).await;
                                                    },
                                                    ClientTriggerCommand::ShownActiveHelp { id } => {
                                                        // Save this so it isn't shown again on next login
                                                        connection.player_data.unlock.seen_active_help.set(id);
                                                    }
                                                    ClientTriggerCommand::SeenCutscene { id } => {
                                                        connection.player_data.unlock.cutscene_seen.set(id);
                                                    }
                                                    ClientTriggerCommand::DirectorTrigger { director_id, trigger, arg } => {
                                                        match trigger {
                                                            DirectorTrigger::Sync => {
                                                                // Always send a sync response for now
                                                                connection.actor_control_self(ActorControlSelf { category: ActorControlCategory::DirectorEvent { director_id, event: DirectorEvent::SyncResponse, arg: 1 } }).await;
                                                            }
                                                            DirectorTrigger::SummonStrikingDummy => {
                                                                connection
                                                                .handle
                                                                .send(ToServer::DebugNewEnemy(
                                                                    connection.id,
                                                                    connection.player_data.actor_id,
                                                                    11744, // TODO: this doesn't seem to be right?!
                                                                ))
                                                                .await;
                                                            }
                                                            _ => tracing::info!("DirectorTrigger: {director_id} {trigger:?} {arg}")
                                                        }

                                                    }
                                                    ClientTriggerCommand::OpenGoldSaucerGeneralTab {} => {
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::GoldSaucerInformation { unk: [0; 40] });
                                                        connection.send_ipc_self(ipc).await;
                                                    }
                                                    ClientTriggerCommand::OpenTrustWindow {} => {
                                                        // We have to send at least one valid trust to the client, otherwise the window never shows.
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::TrustInformation(
                                                            TrustInformation {
                                                                available_content: vec![
                                                                    TrustContent { trust_content_id: 1, // Holminster Switch
                                                                        last_selected_characters: [0xFF; 16] }
                                                                ],
                                                                levels: [0; 34],
                                                                exp: [0; 34],
                                                            }
                                                        ));
                                                        connection.send_ipc_self(ipc).await;
                                                    }
                                                    ClientTriggerCommand::OpenDutySupportWindow {} => {
                                                        // We have to send at least one available duty to the client, otherwise it crashes.
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DutySupportInformation { available_content: vec![1] });
                                                        connection.send_ipc_self(ipc).await;
                                                    }
                                                    ClientTriggerCommand::OpenPortraitsWindow {} => {
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PortraitsInformation { unk: [0; 56] });
                                                        connection.send_ipc_self(ipc).await;
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
                                                connection.handle.send(ToServer::RequestSocialList(connection.id, connection.player_data.actor_id, connection.player_data.party_id, request.clone())).await;
                                            }
                                            ClientZoneIpcData::UpdatePositionHandler { position, rotation, anim_type, anim_state, jump_state, } => {
                                                connection.player_data.rotation = *rotation;
                                                connection.player_data.position = *position;

                                                connection.handle.send(ToServer::ActorMoved(connection.id, connection.player_data.actor_id, *position, *rotation, *anim_type, *anim_state, *jump_state)).await;
                                            }
                                            ClientZoneIpcData::LogOut { .. } => {
                                                connection.gracefully_logged_out = true;
                                                connection.begin_log_out().await;
                                            }
                                            ClientZoneIpcData::Disconnected { .. } => {
                                                tracing::info!("Client disconnected!");

                                                // We no longer send ToServer::Disconnected here because the end of the function already does it unconditionally
                                                break 'outer;  // We break the outer loop here because we're in the middle of a segment loop!
                                            }
                                            ClientZoneIpcData::SendChatMessage(chat_message) => {
                                                let info = MessageInfo {
                                                    sender_actor_id: connection.player_data.actor_id,
                                                    sender_account_id: connection.player_data.account_id,
                                                    sender_world_id: config.world.world_id,
                                                    sender_position: connection.player_data.position,
                                                    sender_name: connection.player_data.name.clone(),
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
                                                        let lua = lua.lock();
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

                                                        let lua = lua.lock();

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
                                                        dst_actor_id: INVALID_OBJECT_ID,
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
                                            ClientZoneIpcData::EventReturnHandler4(handler) => {
                                                let event_type = handler.handler_id >> 16;
                                                let Some(event_type) = EventHandlerType::from_repr(event_type) else {
                                                    tracing::warn!("Unknown event type: {event_type}!");
                                                    continue;
                                                };

                                                // It always assumes a shop... for now
                                                if event_type == EventHandlerType::Shop {
                                                    connection.process_shop_event_return(handler).await;
                                                } else {
                                                    tracing::info!(message = "Event returned", handler_id = handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                                                    if let Some(event) = connection.events.last_mut() {
                                                        event.do_return(handler.scene, &handler.params[..handler.num_results as usize], &mut lua_player);
                                                    } else {
                                                        tracing::warn!("Don't know how to return in {event_type} and there's no current event!");
                                                    }
                                                }
                                            }
                                            ClientZoneIpcData::StartTalkEvent { actor_id, event_id } => {
                                                if connection.start_event(*actor_id, *event_id, EventType::Talk, 0, &mut lua_player).await {
                                                    connection.conditions.set_condition(Condition::OccupiedInQuestEvent);
                                                    connection.send_conditions().await;

                                                    /* TODO: ServerZoneIpcType::Unk18 with data [64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
                                                        * was observed to always be sent by the server upon interacting with shops. They open and function fine without
                                                        * it, but should we send it anyway, for the sake of accuracy? It's also still unclear if this
                                                        * happens for -every- NPC/actor. */

                                                    // begin talk function if it exists
                                                    if let Some(event) = connection.events.last_mut() {
                                                        event.talk(*actor_id, &mut lua_player);
                                                    }
                                                } else {
                                                    connection.send_conditions().await;
                                                }
                                            }
                                            ClientZoneIpcData::EventYieldHandler(handler) => {
                                                tracing::info!(message = "Event yielded", handler_id = handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                                                connection
                                                .events
                                                    .last_mut()
                                                    .unwrap()
                                                    .finish(handler.scene, &handler.params[..handler.num_results as usize], &mut lua_player);
                                            }
                                            ClientZoneIpcData::EventYieldHandler8(handler) => {
                                                tracing::info!(message = "Event yielded", handler_id = handler.handler_id, error_code = handler.error_code, scene = handler.scene, params = ?&handler.params[..handler.num_results as usize]);

                                                connection
                                                    .events
                                                    .last_mut()
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

                                                connection.register_for_content(*content_ids).await;
                                            }
                                            ClientZoneIpcData::ContentFinderAction { action, .. } => {
                                                if *action == ContentFinderUserAction::Accepted {
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

                                                    connection.handle.send(ToServer::JoinContent(connection.id, connection.player_data.actor_id, connection.queued_content.unwrap())).await;
                                                }

                                                // If we don't send this, the content finder gets stuck.
                                                // TODO: this may be screwing up the in-duty menu, probably need to fill it with data!
                                                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UnkContentFinder { unk: [0; 16] });
                                                connection.send_ipc_self(ipc).await;

                                                connection.queued_content = None;
                                            }
                                            ClientZoneIpcData::EquipGearset { gearset_index, containers, indices, .. } => {
                                                // TODO: handle missing items, full inventory and such
                                                for slot in 0..14 {
                                                    let from_slot = indices[slot];
                                                    let from_container = containers[slot];

                                                    if from_container == ContainerType::Equipped {
                                                        continue;
                                                    }

                                                    let from_item = if from_slot != -1 {
                                                        connection.player_data.inventory.get_item(from_container, from_slot as u16)
                                                    } else {
                                                        Item::default()
                                                    };
                                                    let equipped_item = connection.player_data.inventory.equipped.get_slot(slot as u16);

                                                    if !from_item.is_empty_slot() && !equipped_item.is_empty_slot() {
                                                        // If there is something equipped and a replacement for it, we must swap.
                                                        connection.swap_items(from_container, from_slot as u16, ContainerType::Equipped, slot as u16).await;
                                                    } else if !from_item.is_empty_slot() && equipped_item.is_empty_slot() {
                                                        // If there is nothing equipped but a new item in that slot, we just have to move it.
                                                        // TODO: be a little smarter about this maybe?
                                                        connection.swap_items(from_container, from_slot as u16, ContainerType::Equipped, slot as u16).await;
                                                    } else if from_item.is_empty_slot() && !equipped_item.is_empty_slot() {
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

                                                        let target_container = connection.player_data.inventory.get_container(target_container_type);
                                                        if let Some(free_slot) = get_next_free_slot(target_container) {
                                                            connection.swap_items(ContainerType::Equipped, slot as u16, target_container_type, free_slot).await;
                                                        }
                                                    }
                                                }

                                                // Inform the client that the gearset was successfully equipped.
                                                connection.actor_control_self(ActorControlSelf { category: ActorControlCategory::GearSetEquipped { gearset_index: *gearset_index } }).await;

                                                // And that we're done modifying the inventory.
                                                connection.send_inventory_transaction_finish(567, 3584).await;

                                                // Retail also re-sends the equipped container
                                                connection.send_equipped_inventory().await;
                                            }
                                            ClientZoneIpcData::EquipGearset2 { .. } => {
                                                tracing::warn!("Bigger gearsets not supported yet!");
                                            }
                                            ClientZoneIpcData::StartWalkInEvent { event_arg, event_id, .. } => {
                                                // Yes, an ActorControl is sent here, not an ActorControlSelf!
                                                connection.actor_control(connection.player_data.actor_id, ActorControl {
                                                    category: ActorControlCategory::ToggleWeapon {
                                                        shown: false,
                                                        unk_flag: 1,
                                                    }
                                                }).await;
                                                connection.conditions.set_condition(Condition::OccupiedInQuestEvent);
                                                connection.send_conditions().await;

                                                let actor_id = ObjectTypeId { object_id: connection.player_data.actor_id, object_type: ObjectTypeKind::None };
                                                connection.start_event(actor_id, *event_id, EventType::WithinRange, *event_arg, &mut lua_player).await;

                                                // begin walk-in trigger function if it exists
                                                if let Some(event) = connection.events.last_mut() {
                                                     event.enter_trigger(&mut lua_player, *event_arg);
                                                }
                                            }
                                            ClientZoneIpcData::WalkOutsideEvent { event_arg, event_id, .. } => {
                                                // TODO: allow Lua scripts to handle these differently?

                                                // Yes, an ActorControl is sent here, not an ActorControlSelf!
                                                connection.actor_control(connection.player_data.actor_id, ActorControl {
                                                    category: ActorControlCategory::ToggleWeapon {
                                                        shown: false,
                                                        unk_flag: 1,
                                                    }
                                                }).await;
                                                connection.conditions.set_condition(Condition::OccupiedInQuestEvent);
                                                connection.send_conditions().await;

                                                let actor_id = ObjectTypeId { object_id: connection.player_data.actor_id, object_type: ObjectTypeKind::None };
                                                connection.start_event(actor_id, *event_id, EventType::OutsideRange, *event_arg, &mut lua_player).await;

                                                // begin walk-in trigger function if it exists
                                                if let Some(event) = connection.events.last_mut() {
                                                    event.enter_trigger(&mut lua_player, *event_arg);
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
                                            ClientZoneIpcData::SetFreeCompanyGreeting { .. } => {
                                                tracing::info!("Setting the free company greeting is unimplemented");
                                            }
                                            ClientZoneIpcData::SetClientLanguage { language } => {
                                                connection.client_language = *language;
                                            }
                                            ClientZoneIpcData::RequestCharaInfoFromContentIds { .. } => {
                                                tracing::info!("Requesting character info from content ids is unimplemented");
                                            }
                                            ClientZoneIpcData::InviteCharacter {content_id, world_id, invite_type, character_name } => {
                                                tracing::info!("Client invited a character! {:#?} {:#?} {:#?} {:#?} {:#?}", content_id, world_id, invite_type, character_name, data.data);
                                                match invite_type {
                                                    InviteType::Party => {
                                                        connection.handle.send(ToServer::InvitePlayerToParty(connection.player_data.actor_id, *content_id, character_name.clone())).await;
                                                        // Inform the client about the invite they just sent.
                                                        // TODO: Is this static? unk1 and unk2 haven't been observed to have other values so far.
                                                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteCharacterResult {
                                                            content_id: *content_id,
                                                            world_id: *world_id,
                                                            unk1: 1,
                                                            unk2: 1,
                                                            character_name: character_name.clone(),
                                                        });
                                                        connection.send_ipc_self(ipc).await;
                                                    }
                                                    InviteType::FriendList => connection.send_notice("The friend list is not yet implemented.").await,
                                                }
                                            }
                                            ClientZoneIpcData::InviteReply { sender_content_id, sender_world_id, invite_type, response } => {
                                                tracing::info!("Client replied to invite: {:#?} {:#?} {:#?} {:#?}", sender_content_id, sender_world_id, invite_type, response);
                                                connection.handle.send(ToServer::InvitationResponse(connection.id, connection.player_data.account_id, connection.player_data.content_id, connection.player_data.name.clone(), *sender_content_id, *invite_type, *response)).await;
                                            }
                                            ClientZoneIpcData::PartyDisband { .. } => {
                                                tracing::info!("Client is disbanding their party!");
                                                connection.handle.send(ToServer::PartyDisband(connection.player_data.party_id, connection.player_data.account_id, connection.player_data.content_id, connection.player_data.name.clone())).await;
                                            }
                                            ClientZoneIpcData::PartyMemberKick { content_id, character_name, .. } => {
                                                tracing::info!("Player is kicking another player from their party! {} {}", content_id, character_name);
                                                connection.handle.send(ToServer::PartyMemberKick(connection.player_data.party_id, connection.player_data.account_id, connection.player_data.content_id, connection.player_data.name.clone(), *content_id, character_name.clone())).await;
                                            }
                                            ClientZoneIpcData::PartyChangeLeader { content_id, character_name, .. } => {
                                                tracing::info!("Player is promoting another player in their party to leader! {} {}", content_id, character_name);
                                                connection.handle.send(ToServer::PartyChangeLeader(connection.player_data.party_id, connection.player_data.account_id, connection.player_data.content_id, connection.player_data.name.clone(), *content_id, character_name.clone())).await;
                                            }
                                            ClientZoneIpcData::PartyLeave { .. } => {
                                                tracing::info!("Client is leaving their party!");
                                                connection.handle.send(ToServer::PartyMemberLeft(connection.player_data.party_id, connection.player_data.account_id, connection.player_data.content_id, connection.player_data.actor_id, connection.player_data.name.clone(),)).await;
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
                                                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateSearchInfo { online_status: OnlineStatusMask::default(), unk1: 0, unk2: 0, region: 0, message: String::default() });
                                                connection.send_ipc_self(ipc).await;
                                            }
                                            ClientZoneIpcData::RequestOwnSearchInfo { .. } => {
                                                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SetSearchInfo(SearchInfo::default()));
                                                connection.send_ipc_self(ipc).await;
                                            }
                                            ClientZoneIpcData::EnterTerritoryEvent { event_id } => {
                                                connection.start_event(ObjectTypeId { object_id: connection.player_data.actor_id, object_type: ObjectTypeKind::None }, *event_id, EventType::EnterTerritory, connection.player_data.zone_id as u32, &mut lua_player).await;
                                                if let Some(event) = connection.events.last_mut() {
                                                    event.enter_territory(&mut lua_player);
                                                }
                                            }
                                            ClientZoneIpcData::Unknown { unk } => {
                                                tracing::warn!("Unknown Zone packet {:?} recieved ({} bytes), this should be handled!", data.header.op_code, unk.len());
                                            }
                                        }
                                    }
                                    SegmentData::KeepAliveRequest { id, timestamp } => connection.send_keep_alive(*id, *timestamp).await,
                                    SegmentData::KeepAliveResponse { .. } => {
                                        // these should be safe to ignore
                                    }
                                    _ => {
                                        panic!("ZoneConnection: The server is recieving a response or unknown packet: {segment:#?}")
                                    }
                                }
                            }

                            // Process any queued packets from scripts and whatnot
                            lua_player.queued_tasks.append(&mut connection.queued_tasks);
                            connection.process_lua_player(&mut lua_player).await;

                            // update lua player
                            lua_player.player_data = connection.player_data.clone();
                        }
                    },
                    Err(_) => {
                        tracing::info!("ZoneConnection {:#?} was killed because of a network error!", client_handle.id);
                        break;
                    },
                }
            }
            msg = internal_recv.recv() => match msg {
                Some(msg) => match msg {
                    FromServer::Message(msg) => connection.send_message(msg).await,
                    FromServer::ActorSpawn(actor, spawn) => connection.spawn_actor(actor, spawn).await,
                    FromServer::ActorMove(actor_id, position, rotation, anim_type, anim_state, jump_state) => connection.set_actor_position(actor_id, position, rotation, anim_type, anim_state, jump_state).await,
                    FromServer::DeleteActor(object_id, spawn_index) => connection.delete_actor(object_id, spawn_index).await,
                    FromServer::DeleteObject(spawn_index) => connection.delete_object(spawn_index).await,
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
                    FromServer::UpdateConfig(actor_id, config) => connection.update_config(actor_id, config).await,
                    FromServer::ActorEquip(actor_id, main_weapon_id, sub_weapon_id, model_ids) => connection.update_equip(actor_id, main_weapon_id, sub_weapon_id, model_ids).await,
                    FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id) => connection.lose_effect(effect_id, effect_param, effect_source_actor_id).await,
                    FromServer::Conditions(conditions) => {
                        connection.conditions = conditions;
                        connection.send_conditions().await;
                    },
                    FromServer::ChangeZone(zone_id, content_finder_condition_id, weather_id, position, rotation, lua_zone, initial_login) => {
                        connection.handle_zone_change(zone_id, content_finder_condition_id, weather_id, position, rotation, initial_login, &lua_zone).await;
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
                        connection.player_data.party_id = party_id;
                        connection.player_data.rejoining_party = true;
                    }
                    FromServer::PacketSegment(ipc, from_actor_id) => {
                        let segment = PacketSegment {
                            source_actor: from_actor_id,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc,
                            data: SegmentData::Ipc(ipc),
                        };
                        connection.send_segment(segment).await;
                    }
                    FromServer::NewTasks(mut tasks) => connection.queued_tasks.append(&mut tasks),
                    FromServer::NewStatusEffects(status_effects) => lua_player.status_effects = status_effects,
                    FromServer::ObjectSpawn(object) => connection.spawn_object(object).await,
                    _ => { tracing::error!("Zone connection {:#?} received a FromServer message we don't care about: {:#?}, ensure you're using the right client network or that you've implemented a handler for it if we actually care about it!", client_handle.id, msg); }
                },
                None => break,
            }
        }
    }

    // forcefully log out the player if they weren't logging out but force D/C'd
    if connection.player_data.actor_id != INVALID_OBJECT_ID {
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
                connection.player_data.actor_id,
            ))
            .await;

        if connection.is_in_party() {
            connection
                .handle
                .send(ToServer::PartyMemberOffline(
                    connection.player_data.party_id,
                    connection.player_data.account_id,
                    connection.player_data.content_id,
                    connection.player_data.actor_id,
                    connection.player_data.name.clone(),
                ))
                .await;
        }
    }
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

    let (handle, _) = spawn_main_loop();

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
