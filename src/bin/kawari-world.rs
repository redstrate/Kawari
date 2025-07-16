use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kawari::common::Position;
use kawari::common::{GameData, ItemInfoQuery, timestamp_secs};
use kawari::config::get_config;
use kawari::inventory::{
    BuyBackItem, ContainerType, CurrencyKind, Item, ItemOperationKind, get_container_type,
};
use kawari::ipc::chat::{ServerChatIpcData, ServerChatIpcSegment};
use kawari::ipc::zone::{
    ActorControlCategory, ActorControlSelf, ItemOperation, PlayerEntry, PlayerSpawn, PlayerStatus,
    SocialList,
};

use kawari::ipc::zone::{
    ClientTriggerCommand, ClientZoneIpcData, EventStart, GameMasterRank, OnlineStatus,
    ServerZoneIpcData, ServerZoneIpcSegment, SocialListRequestType,
};
use kawari::opcodes::{ServerChatIpcType, ServerZoneIpcType};
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{
    ConnectionType, PacketSegment, PacketState, SegmentData, SegmentType, send_keep_alive,
};
use kawari::world::{
    ChatHandler, ExtraLuaState, LuaZone, ObsfucationData, Zone, ZoneConnection, load_init_script,
};
use kawari::world::{
    ClientHandle, Event, FromServer, LuaPlayer, PlayerData, ServerHandle, StatusEffects, ToServer,
    WorldDatabase, handle_custom_ipc, server_main_loop,
};
use kawari::{
    ERR_INVENTORY_ADD_FAILED, LogMessageType, RECEIVE_BUFFER_SIZE, TITLE_UNLOCK_BITMASK_SIZE,
};

use mlua::{Function, Lua};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender, channel, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use kawari::{INVALID_ACTOR_ID, INVENTORY_ACTION_ACK_GENERAL, INVENTORY_ACTION_ACK_SHOP};

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

struct ClientData {
    /// Socket for data recieved from the global server
    recv: Receiver<FromServer>,
    connection: ZoneConnection,
}

/// Spawn a new client actor.
pub fn spawn_client(connection: ZoneConnection) {
    let (send, recv) = channel(64);

    let id = &connection.id.clone();
    let ip = &connection.ip.clone();

    let data = ClientData { recv, connection };

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

async fn start_client(my_handle: oneshot::Receiver<ClientHandle>, data: ClientData) {
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

    // TODO: this is terrible, just have a separate zone/chat connection
    let mut is_zone_connection = false;

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
                        }

                        if n > 0 {
                            connection.last_keep_alive = Instant::now();

                            let (segments, connection_type) = connection.parse_packet(&buf[..n]);
                            for segment in &segments {
                                match &segment.data {
                                    SegmentData::None() => {},
                                    SegmentData::Setup { actor_id } => {
                                        // for some reason they send a string representation
                                        let actor_id = actor_id.parse::<u32>().unwrap();

                                        // initialize player data if it doesn't exist'
                                        if connection.player_data.actor_id == 0 {
                                            connection.player_data = database.find_player_data(actor_id);
                                        }

                                        if connection_type == ConnectionType::Zone {
                                            is_zone_connection = true;

                                            // collect actor data
                                            connection.initialize(actor_id).await;

                                            connection.exit_position = Some(connection.player_data.position);
                                            connection.exit_rotation = Some(connection.player_data.rotation);

                                            let mut client_handle = client_handle.clone();
                                            client_handle.actor_id = actor_id;

                                            // tell the server we exist, now that we confirmed we are a legitimate connection
                                            connection.handle.send(ToServer::NewClient(client_handle)).await;
                                        } else if connection_type == ConnectionType::Chat {
                                            // We have send THEM a keep alive
                                            connection.send_chat_segment(PacketSegment {
                                                segment_type: SegmentType::KeepAliveRequest,
                                                data: SegmentData::KeepAliveRequest {
                                                    id: 0xE0037603u32,
                                                    timestamp: timestamp_secs(),
                                                },
                                                ..Default::default()
                                            })
                                            .await;

                                            // initialize connection
                                            connection.send_chat_segment(PacketSegment {
                                                segment_type: SegmentType::Initialize,
                                                data: SegmentData::Initialize {
                                                    actor_id: connection.player_data.actor_id,
                                                    timestamp: timestamp_secs(),
                                                },
                                                ..Default::default()
                                            })
                                            .await;

                                            // we need the actor id at this point!
                                            assert!(connection.player_data.actor_id != 0);

                                            // send login reply
                                            {
                                                let ipc = ServerChatIpcSegment {
                                                    op_code: ServerChatIpcType::LoginReply,
                                                    timestamp: timestamp_secs(),
                                                    data: ServerChatIpcData::LoginReply {
                                                        timestamp: 0,
                                                        sid: 0,
                                                    },
                                                    ..Default::default()
                                                };

                                                connection.send_chat_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc,
                                                    data: SegmentData::Ipc { data: ipc },
                                                })
                                                .await;
                                            }
                                        }
                                    }
                                    SegmentData::Ipc { data } => {
                                        match &data.data {
                                            ClientZoneIpcData::InitRequest { .. } => {
                                                tracing::info!(
                                                    "Client is now requesting zone information. Sending!"
                                                );

                                                // IPC Init(?)
                                                {
                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::InitResponse,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::InitResponse {
                                                            unk1: 0,
                                                            character_id: connection.player_data.actor_id,
                                                            unk2: 0,
                                                        },
                                                        ..Default::default()
                                                    };

                                                    connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc,
                                                        data: SegmentData::Ipc { data: ipc },
                                                    })
                                                    .await;
                                                }

                                                let chara_details =
                                                database.find_chara_make(connection.player_data.content_id);

                                                // Send inventory
                                                connection.send_inventory(true).await;

                                                // set chara gear param
                                                connection
                                                .actor_control_self(ActorControlSelf {
                                                    category: ActorControlCategory::SetCharaGearParamUI {
                                                        unk1: 1,
                                                        unk2: 1,
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
                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::PlayerStatus,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::PlayerStatus(PlayerStatus {
                                                            content_id: connection.player_data.content_id,
                                                            // Disabled for now until the client stops freaking out
                                                            //exp: connection.player_data.classjob_exp,
                                                            max_level: 100,
                                                            expansion: 5,
                                                            name: chara_details.name,
                                                            char_id: connection.player_data.actor_id,
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
                                                            unlocks: connection.player_data.unlocks.clone(),
                                                            aetherytes: connection.player_data.aetherytes.clone(),
                                                            ..Default::default()
                                                        }),
                                                        ..Default::default()
                                                    };

                                                    connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc,
                                                        data: SegmentData::Ipc { data: ipc },
                                                    })
                                                    .await;
                                                }

                                                connection.send_quest_information().await;

                                                let zone_id = connection.player_data.zone_id;
                                                connection.change_zone(zone_id).await;

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

                                                // tell the server we loaded into the zone, so it can start sending us actors
                                                connection.handle.send(ToServer::ZoneLoaded(connection.id, connection.zone.as_ref().unwrap().id, common.clone())).await;

                                                let chara_details = database.find_chara_make(connection.player_data.content_id);

                                                connection.send_inventory(false).await;
                                                connection.send_stats(&chara_details).await;

                                                let online_status = if connection.player_data.gm_rank == GameMasterRank::NormalUser {
                                                    OnlineStatus::Online
                                                } else {
                                                    OnlineStatus::GameMasterBlue
                                                };

                                                // send player spawn
                                                {
                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::PlayerSpawn,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::PlayerSpawn(PlayerSpawn {
                                                            account_id: connection.player_data.account_id,
                                                            content_id: connection.player_data.content_id,
                                                            current_world_id: config.world.world_id,
                                                            home_world_id: config.world.world_id,
                                                            gm_rank: connection.player_data.gm_rank,
                                                            online_status,
                                                            common: common.clone(),
                                                                                             ..Default::default()
                                                        }),
                                                        ..Default::default()
                                                    };

                                                    connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc,
                                                        data: SegmentData::Ipc { data: ipc },
                                                    })
                                                    .await;
                                                }

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

                                                        let ipc = ServerZoneIpcSegment {
                                                            op_code: ServerZoneIpcType::TitleList,
                                                            timestamp: timestamp_secs(),
                                                            data: ServerZoneIpcData::TitleList {
                                                                unlock_bitmask: [0xFF; TITLE_UNLOCK_BITMASK_SIZE]
                                                            },
                                                            ..Default::default()
                                                        };

                                                        connection
                                                        .send_segment(PacketSegment {
                                                            source_actor: connection.player_data.actor_id,
                                                            target_actor: connection.player_data.actor_id,
                                                            segment_type: SegmentType::Ipc,
                                                            data: SegmentData::Ipc { data: ipc },
                                                        })
                                                        .await;
                                                    },
                                                    _ => {
                                                        // inform the server of our trigger, it will handle sending it to other clients
                                                        connection.handle.send(ToServer::ClientTrigger(connection.id, connection.player_data.actor_id, trigger.clone())).await;
                                                    }
                                                }
                                            }
                                            ClientZoneIpcData::Unk2 { .. } => {
                                                // no-op
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

                                                match &request.request_type {
                                                    SocialListRequestType::Party => {
                                                        let ipc = ServerZoneIpcSegment {
                                                            op_code: ServerZoneIpcType::SocialList,
                                                            timestamp: timestamp_secs(),
                                                            data: ServerZoneIpcData::SocialList(SocialList {
                                                                request_type: request.request_type,
                                                                sequence: request.count,
                                                                entries: vec![PlayerEntry {
                                                                    // TODO: fill with actual player data, it also shows up wrong in game
                                                                    content_id: connection.player_data.content_id,
                                                                    zone_id: connection.zone.as_ref().unwrap().id,
                                                                                                zone_id1: 0x0100,
                                                                                                class_job: 36,
                                                                                                level: 100,
                                                                                                one: 1,
                                                                                                name: "INVALID".to_string(),
                                                                                                ..Default::default()
                                                                }],
                                                            }),
                                                            ..Default::default()
                                                        };

                                                        connection
                                                        .send_segment(PacketSegment {
                                                            source_actor: connection.player_data.actor_id,
                                                            target_actor: connection.player_data.actor_id,
                                                            segment_type: SegmentType::Ipc,
                                                            data: SegmentData::Ipc { data: ipc },
                                                        })
                                                        .await;
                                                    }
                                                    SocialListRequestType::Friends => {
                                                        let ipc = ServerZoneIpcSegment {
                                                            op_code: ServerZoneIpcType::SocialList,
                                                            timestamp: timestamp_secs(),
                                                            data: ServerZoneIpcData::SocialList(SocialList {
                                                                request_type: request.request_type,
                                                                sequence: request.count,
                                                                entries: Default::default(),
                                                            }),
                                                            ..Default::default()
                                                        };

                                                        connection
                                                        .send_segment(PacketSegment {
                                                            source_actor: connection.player_data.actor_id,
                                                            target_actor: connection.player_data.actor_id,
                                                            segment_type: SegmentType::Ipc,
                                                            data: SegmentData::Ipc { data: ipc },
                                                        })
                                                        .await;
                                                    }
                                                }
                                            }
                                            ClientZoneIpcData::UpdatePositionHandler { position, rotation } => {
                                                connection.player_data.rotation = *rotation;
                                                connection.player_data.position = *position;

                                                connection.handle.send(ToServer::ActorMoved(connection.id, connection.player_data.actor_id, *position, *rotation)).await;
                                            }
                                            ClientZoneIpcData::LogOut { .. } => {
                                                tracing::info!("Recieved log out from client!");

                                                connection.begin_log_out().await;
                                            }
                                            ClientZoneIpcData::Disconnected { .. } => {
                                                tracing::info!("Client disconnected!");

                                                connection.handle.send(ToServer::Disconnected(connection.id)).await;

                                                break;
                                            }
                                            ClientZoneIpcData::ChatMessage(chat_message) => {
                                                connection.handle.send(ToServer::Message(connection.id, chat_message.message.clone())).await;

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
                                                let lua = lua.lock().unwrap();
                                                let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

                                                if let Some(command_script) =
                                                    state.gm_command_scripts.get(command)
                                                {
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
                                                                let func: Function =
                                                                    lua.globals().get("onCommand")?;
                                                                func.call::<()>(([*arg0, *arg1, *arg2, *arg3], connection_data))?;

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
                                                                connection.player_data.account_id, command);
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
                                            ClientZoneIpcData::ZoneJump {
                                                exit_box,
                                                position,
                                                ..
                                            } => {
                                                tracing::info!(
                                                    "Character entered {exit_box} with a position of {position:#?}!"
                                                );

                                                // find the exit box id
                                                let new_territory;
                                                {
                                                    let (_, exit_box) = connection
                                                    .zone
                                                    .as_ref()
                                                    .unwrap()
                                                    .find_exit_box(*exit_box)
                                                    .unwrap();

                                                    // find the pop range on the other side
                                                    let mut game_data = game_data.lock().unwrap();
                                                    let new_zone = Zone::load(&mut game_data, exit_box.territory_type);
                                                    if let Some((destination_object, _)) = new_zone
                                                        .find_pop_range(exit_box.destination_instance_id) {
                                                        // set the exit position
                                                        connection.exit_position = Some(Position {
                                                            x: destination_object.transform.translation[0],
                                                            y: destination_object.transform.translation[1],
                                                            z: destination_object.transform.translation[2],
                                                        });
                                                    }
                                                    new_territory = exit_box.territory_type;
                                                }

                                                connection.change_zone(new_territory).await;
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
                                            ClientZoneIpcData::Unk17 { unk1, .. } => {
                                                // this is *usually* sent in response, but not always
                                                let ipc = ServerZoneIpcSegment {
                                                    op_code: ServerZoneIpcType::UnkCall,
                                                    timestamp: timestamp_secs(),
                                                    data: ServerZoneIpcData::UnkCall {
                                                        unk1: *unk1, // copied from here
                                                        unk2: 333, // always this for some reason
                                                    },
                                                    ..Default::default()
                                                };

                                                connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc,
                                                    data: SegmentData::Ipc { data: ipc },
                                                })
                                                .await;
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

                                                // if updated equipped items, we have to process that
                                                if action.src_storage_id == ContainerType::Equipped || action.dst_storage_id == ContainerType::Equipped {
                                                    connection.inform_equip().await;
                                                }

                                                if action.operation_type == ItemOperationKind::Discard {
                                                    tracing::info!("Client is discarding from their inventory!");

                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::InventoryTransaction,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::InventoryTransaction {
                                                            sequence: connection.player_data.item_sequence,
                                                            operation_type: action.operation_type,
                                                            src_actor_id: connection.player_data.actor_id,
                                                            src_storage_id: action.src_storage_id,
                                                            src_container_index: action.src_container_index,
                                                            src_stack: action.src_stack,
                                                            src_catalog_id: action.src_catalog_id,
                                                            dst_actor_id: INVALID_ACTOR_ID,
                                                            dummy_container: ContainerType::DiscardingItemSentinel,
                                                            dst_storage_id: ContainerType::DiscardingItemSentinel,
                                                            dst_container_index: u16::MAX,
                                                            dst_stack: 0,
                                                            dst_catalog_id: 0,
                                                        },
                                                        ..Default::default()
                                                    };
                                                    connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc,
                                                        data: SegmentData::Ipc { data: ipc },
                                                    })
                                                    .await;
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
                                                            if let Some(add_result) = connection.player_data.inventory.add_in_next_free_slot(Item::new(*item_quantity, item_info.id), item_info.stack_size) {
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
                                                                connection.send_message(ERR_INVENTORY_ADD_FAILED).await;
                                                                connection.event_finish(*event_id).await;
                                                            }
                                                        } else {
                                                            connection.send_message("Insufficient gil to buy item. Nice try bypassing the client-side check!").await;
                                                            connection.event_finish(*event_id).await;
                                                        }
                                                    } else {
                                                        connection.send_message("Unable to find shop item, this is a bug in Kawari!").await;
                                                        connection.event_finish(*event_id).await;
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
                                                            stack_size: item_info.stack_size,
                                                        };
                                                        connection.player_data.buyback_list.push_item(*event_id, bb_item);

                                                        connection.player_data.inventory.currency.gil.quantity += quantity * item_info.price_low;
                                                        connection.send_gilshop_item_update(ContainerType::Currency as u16, 0, connection.player_data.inventory.currency.gil.quantity, CurrencyKind::Gil as u32).await;
                                                        connection.send_gilshop_item_update(storage as u16, index as u16, 0, 0).await;

                                                        // TODO: Refactor InventoryTransactions into connection.rs
                                                        let ipc = ServerZoneIpcSegment {
                                                            op_code: ServerZoneIpcType::InventoryTransaction,
                                                            timestamp: timestamp_secs(),
                                                            data: ServerZoneIpcData::InventoryTransaction {
                                                                sequence: connection.player_data.item_sequence,
                                                                operation_type: ItemOperationKind::UpdateCurrency,
                                                                src_actor_id: connection.player_data.actor_id,
                                                                src_storage_id: ContainerType::Currency,
                                                                src_container_index: 0,
                                                                src_stack: connection.player_data.inventory.currency.gil.quantity,
                                                                src_catalog_id: CurrencyKind::Gil as u32,
                                                                dst_actor_id: INVALID_ACTOR_ID,
                                                                dummy_container: ContainerType::DiscardingItemSentinel,
                                                                dst_storage_id: ContainerType::DiscardingItemSentinel,
                                                                dst_container_index: u16::MAX,
                                                                dst_stack: 0,
                                                                dst_catalog_id: 0,
                                                            },
                                                            ..Default::default()
                                                        };
                                                        connection
                                                        .send_segment(PacketSegment {
                                                            source_actor: connection.player_data.actor_id,
                                                            target_actor: connection.player_data.actor_id,
                                                            segment_type: SegmentType::Ipc,
                                                            data: SegmentData::Ipc { data: ipc },
                                                        })
                                                        .await;

                                                        // Process the server's inventory first.
                                                        let action = ItemOperation {
                                                            operation_type: ItemOperationKind::Discard,
                                                            src_storage_id: storage,
                                                            src_container_index: index as u16,
                                                            ..Default::default()
                                                        };

                                                        connection.player_data.inventory.process_action(&action);

                                                        let ipc = ServerZoneIpcSegment {
                                                            op_code: ServerZoneIpcType::InventoryTransaction,
                                                            timestamp: timestamp_secs(),
                                                            data: ServerZoneIpcData::InventoryTransaction {
                                                                sequence: connection.player_data.item_sequence,
                                                                operation_type: ItemOperationKind::Discard,
                                                                src_actor_id: connection.player_data.actor_id,
                                                                src_storage_id: storage,
                                                                src_container_index: index as u16,
                                                                src_stack: quantity,
                                                                src_catalog_id: item_info.id,
                                                                dst_actor_id: INVALID_ACTOR_ID,
                                                                dummy_container: ContainerType::DiscardingItemSentinel,
                                                                dst_storage_id: ContainerType::DiscardingItemSentinel,
                                                                dst_container_index: u16::MAX,
                                                                dst_stack: 0,
                                                                dst_catalog_id: 0,
                                                            },
                                                            ..Default::default()
                                                        };
                                                        connection
                                                        .send_segment(PacketSegment {
                                                            source_actor: connection.player_data.actor_id,
                                                            target_actor: connection.player_data.actor_id,
                                                            segment_type: SegmentType::Ipc,
                                                            data: SegmentData::Ipc { data: ipc },
                                                        })
                                                        .await;

                                                        connection.send_inventory_transaction_finish(0x100, 0x300).await;

                                                        connection.send_gilshop_ack(*event_id, item_info.id, quantity, item_info.price_low, LogMessageType::ItemSold).await;

                                                        let target_id = connection.player_data.target_actorid;

                                                        let mut params = connection.player_data.buyback_list.as_scene_params(*event_id, false);
                                                        params[0] = SELL;
                                                        params[1] = 0; // The "terminator" is 0 for sell mode.
                                                        connection.event_scene(&target_id, *event_id, 10, 8193, params).await;
                                                    } else {
                                                        connection.send_message("Unable to find shop item, this is a bug in Kawari!").await;
                                                        connection.event_finish(*event_id).await;
                                                    }
                                                } else {
                                                    tracing::error!("Received unknown transaction mode {buy_sell_mode}!");
                                                    connection.event_finish(*event_id).await;
                                                }
                                            }
                                            ClientZoneIpcData::StartTalkEvent { actor_id, event_id } => {
                                                connection.player_data.target_actorid = *actor_id;
                                                // load event
                                                {
                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::EventStart,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::EventStart(EventStart {
                                                            target_id: *actor_id,
                                                            event_id: *event_id,
                                                            event_type: 1, // talk?
                                                            ..Default::default()
                                                        }),
                                                        ..Default::default()
                                                    };

                                                    connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc,
                                                        data: SegmentData::Ipc { data: ipc },
                                                    })
                                                    .await;
                                                }

                                                /* TODO: ServerZoneIpcType::Unk18 with data [64,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]
                                                 * was observed to always be sent by the server upon interacting with shops. They open and function fine without
                                                 * it, but should we send it anyway, for the sake of accuracy? It's also still unclear if this
                                                 * happens for -every- NPC/actor. */

                                                let mut should_cancel = false;
                                                {
                                                    let lua = lua.lock().unwrap();
                                                    let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

                                                    if let Some(event_script) =
                                                        state.event_scripts.get(event_id)
                                                        {
                                                            connection.event = Some(Event::new(*event_id, event_script));
                                                            connection
                                                            .event
                                                            .as_mut()
                                                            .unwrap()
                                                            .talk(*actor_id, &mut lua_player);
                                                        } else {
                                                            tracing::warn!("Event {event_id} isn't scripted yet! Ignoring...");

                                                            should_cancel = true;
                                                        }
                                                }

                                                if should_cancel {
                                                    // give control back to the player so they aren't stuck
                                                    connection.event_finish(*event_id).await;
                                                    connection.send_message(&format!("Event {event_id} tried to start, but it doesn't have a script associated with it!")).await;
                                                }
                                            }
                                            ClientZoneIpcData::EventYieldHandler(handler) => {
                                                tracing::info!("Finishing this event... {} {} {} {:?}", handler.handler_id, handler.error_code, handler.scene, &handler.params[..handler.num_results as usize]);

                                                connection
                                                .event
                                                    .as_mut()
                                                    .unwrap()
                                                    .finish(handler.scene, &handler.params[..handler.num_results as usize], &mut lua_player);
                                            }
                                            ClientZoneIpcData::EventYieldHandler8(handler) => {
                                                tracing::info!("Finishing this event... {} {} {} {:?}", handler.handler_id, handler.error_code, handler.scene, &handler.params[..handler.num_results as usize]);

                                                connection
                                                    .event
                                                    .as_mut()
                                                    .unwrap()
                                                    .finish(handler.scene, &handler.params[..handler.num_results as usize], &mut lua_player);
                                            }
                                            ClientZoneIpcData::Config(config) => {
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
                                                 let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::EventUnkReply,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::EventUnkReply {
                                                            event_id: *event_id,
                                                            unk1: *unk1,
                                                            unk2: *unk2,
                                                            unk3: *unk3 + 1,
                                                        },
                                                        ..Default::default()
                                                    };

                                                    connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc,
                                                        data: SegmentData::Ipc { data: ipc },
                                                    })
                                                    .await;
                                            }
                                            ClientZoneIpcData::UnkCall2 { .. } => {
                                                let ipc = ServerZoneIpcSegment {
                                                    op_code: ServerZoneIpcType::UnkResponse2,
                                                    timestamp: timestamp_secs(),
                                                    data: ServerZoneIpcData::UnkResponse2 {
                                                        unk1: 1,
                                                    },
                                                    ..Default::default()
                                                };

                                                connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc,
                                                    data: SegmentData::Ipc { data: ipc },
                                                })
                                                .await;
                                            }
                                            ClientZoneIpcData::ContentFinderRegister { .. } => {
                                                let ipc = ServerZoneIpcSegment {
                                                    op_code: ServerZoneIpcType::ContentFinderFound,
                                                    timestamp: timestamp_secs(),
                                                    data: ServerZoneIpcData::ContentFinderFound {
                                                        state1: 2,
                                                        classjob_id: 1,
                                                        unk1: [
                                                            5,
                                                            2,
                                                            5,
                                                            2,
                                                            5,
                                                            2,
                                                            96,
                                                            4,
                                                            5,
                                                            64,
                                                            2,
                                                            5,
                                                            2,
                                                            5,
                                                            2,
                                                            2,
                                                            2,
                                                            2,
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
                                                        ]
                                                    },
                                                    ..Default::default()
                                                };

                                                connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc,
                                                    data: SegmentData::Ipc { data: ipc },
                                                })
                                                .await;
                                            }
                                            ClientZoneIpcData::Unknown { .. } => {
                                                tracing::warn!("Unknown packet {:?} recieved, this should be handled!", data.op_code);
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
                                        tracing::info!("Got keep alive response from client... cool...");
                                    }
                                    SegmentData::KawariIpc { data } => handle_custom_ipc(&mut connection, data).await,
                                    _ => {
                                        panic!("The server is recieving a response or unknown packet: {segment:#?}")
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

                            if let Some(zone) = &connection.zone {
                                lua_player.zone_data = LuaZone {
                                    zone_id: zone.id,
                                    weather_id: connection.weather_id,
                                    internal_name: zone.internal_name.clone(),
                                    region_name: zone.region_name.clone(),
                                    place_name: zone.place_name.clone(),
                                    intended_use: zone.intended_use,
                                };
                            }
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
                    FromServer::Message(msg) => connection.send_message(&msg).await,
                    FromServer::ActorSpawn(actor, spawn) => connection.spawn_actor(actor, spawn).await,
                    FromServer::ActorMove(actor_id, position, rotation) => connection.set_actor_position(actor_id, position, rotation).await,
                    FromServer::ActorDespawn(actor_id) => connection.remove_actor(actor_id).await,
                    FromServer::ActorControl(actor_id, actor_control) => connection.actor_control(actor_id, actor_control).await,
                    FromServer::ActorControlTarget(actor_id, actor_control) => connection.actor_control_target(actor_id, actor_control).await,
                    FromServer::ActorControlSelf(actor_control) => connection.actor_control_self(actor_control).await,
                    FromServer::ActionComplete(request) => connection.execute_action(request, &mut lua_player).await,
                    FromServer::ActionCancelled() => connection.cancel_action().await,
                    FromServer::UpdateConfig(actor_id, config) => connection.update_config(actor_id, config).await,
                    FromServer::ActorEquip(actor_id, main_weapon_id, model_ids) => connection.update_equip(actor_id, main_weapon_id, model_ids).await,
                    FromServer::ReplayPacket(segment) => connection.send_segment(segment).await,
                    FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id) => connection.lose_effect(effect_id, effect_param, effect_source_actor_id, &mut lua_player).await,
                },
                None => break,
            }
        }
    }

    // forcefully log out the player if they weren't logging out but force D/C'd
    if connection.player_data.actor_id != 0
        && !connection.gracefully_logged_out
        && is_zone_connection
    {
        tracing::info!(
            "Forcefully logging out connection {:#?}...",
            client_handle.id
        );
        connection.begin_log_out().await;
        connection
            .handle
            .send(ToServer::Disconnected(connection.id))
            .await;
    }
}

async fn handle_rcon(listener: &Option<TcpListener>) -> Option<(TcpStream, SocketAddr)> {
    match listener {
        Some(listener) => Some(listener.accept().await.ok()?),
        None => None,
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.world.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    let rcon_listener = if !config.world.rcon_password.is_empty() {
        Some(
            TcpListener::bind(config.world.get_rcon_socketaddr())
                .await
                .unwrap(),
        )
    } else {
        None
    };

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
        tokio::select! {
            Ok((socket, ip)) = listener.accept() => {
                let id = handle.next_id();

                let state = PacketState {
                    client_key: None,
                    clientbound_oodle: OodleNetwork::new(),
                    serverbound_oodle: OodleNetwork::new(),
                };

                spawn_client(ZoneConnection {
                    config: get_config().world,
                    socket,
                    state,
                    player_data: PlayerData::default(),
                    spawn_index: 0,
                    zone: None,
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
                });
            }
            Some((mut socket, _)) = handle_rcon(&rcon_listener) => {
                let mut authenticated = false;

                loop {
                    // read from client
                    let mut resp_bytes = [0u8; rkon::MAX_PACKET_SIZE];
                    let n = socket.read(&mut resp_bytes).await.unwrap();
                    if n > 0 {
                        let request = rkon::Packet::decode(&resp_bytes).unwrap();

                        match request.packet_type {
                            rkon::PacketType::Command => {
                                if authenticated {
                                    let response = rkon::Packet {
                                        request_id: request.request_id,
                                        packet_type: rkon::PacketType::Command,
                                        body: "hello world!".to_string()
                                    };
                                    let encoded = response.encode();
                                    socket.write_all(&encoded).await.unwrap();
                                }
                            },
                            rkon::PacketType::Login => {
                                let config = get_config();
                                if request.body == config.world.rcon_password {
                                    authenticated = true;

                                    let response = rkon::Packet {
                                        request_id: request.request_id,
                                        packet_type: rkon::PacketType::Command,
                                        body: String::default()
                                    };
                                    let encoded = response.encode();
                                    socket.write_all(&encoded).await.unwrap();
                                } else {
                                    authenticated = false;

                                    let response = rkon::Packet {
                                        request_id: -1,
                                        packet_type: rkon::PacketType::Command,
                                        body: String::default()
                                    };
                                    let encoded = response.encode();
                                    socket.write_all(&encoded).await.unwrap();
                                }
                            },
                            _ => tracing::warn!("Ignoring unknown RCON packet")
                        }
                    }
                }
            }
        };
    }
}
