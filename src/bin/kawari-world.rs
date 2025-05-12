use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use kawari::RECEIVE_BUFFER_SIZE;
use kawari::common::Position;
use kawari::common::{GameData, timestamp_secs};
use kawari::config::get_config;
use kawari::inventory::Item;
use kawari::ipc::chat::{ServerChatIpcData, ServerChatIpcSegment};
use kawari::ipc::zone::{
    ActionEffect, ActionResult, ClientTriggerCommand, ClientZoneIpcData, CommonSpawn, EffectKind,
    EventStart, GameMasterCommandType, GameMasterRank, OnlineStatus, ServerZoneIpcData,
    ServerZoneIpcSegment, SocialListRequestType,
};
use kawari::ipc::zone::{
    ActorControlCategory, ActorControlSelf, PlayerEntry, PlayerSpawn, PlayerStatus, SocialList,
};
use kawari::opcodes::{ServerChatIpcType, ServerZoneIpcType};
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{
    ConnectionType, PacketSegment, PacketState, SegmentData, SegmentType, send_keep_alive,
};
use kawari::world::{ChatHandler, Zone, ZoneConnection};
use kawari::world::{
    ClientHandle, EffectsBuilder, Event, FromServer, LuaPlayer, PlayerData, ServerHandle,
    StatusEffects, ToServer, WorldDatabase, handle_custom_ipc, server_main_loop,
};

use mlua::{Function, Lua};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender, channel, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

#[derive(Default)]
struct ExtraLuaState {
    action_scripts: HashMap<u32, String>,
    event_scripts: HashMap<u32, String>,
    command_scripts: HashMap<String, String>,
}

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
        common: CommonSpawn::default(),
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

                            let (segments, connection_type) = connection.parse_packet(&buf[..n]).await;
                            for segment in &segments {
                                match &segment.data {
                                    SegmentData::Setup { ticket } => {
                                        // for some reason they send a string representation
                                        let actor_id = ticket.parse::<u32>().unwrap();

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
                                            client_handle.common = connection.get_player_common_spawn(connection.exit_position, connection.exit_rotation);

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
                                                    player_id: connection.player_data.actor_id,
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
                                                connection.send_inventory(false).await;

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

                                                // Player Setup
                                                {
                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::PlayerStatus,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::PlayerStatus(PlayerStatus {
                                                            content_id: connection.player_data.content_id,
                                                            exp: [10000; 32],
                                                            levels: [100; 32],
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
                                                // tell the server we loaded into the zone, so it can start sending us acors
                                                connection.handle.send(ToServer::ZoneLoaded(connection.id, connection.zone.as_ref().unwrap().id)).await;

                                                let common = connection.get_player_common_spawn(connection.exit_position, connection.exit_rotation);

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

                                                // fade in?
                                                {
                                                    let ipc = ServerZoneIpcSegment {
                                                        op_code: ServerZoneIpcType::PrepareZoning,
                                                        timestamp: timestamp_secs(),
                                                        data: ServerZoneIpcData::PrepareZoning {
                                                            unk: [0, 0, 0, 0],
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

                                                // wipe any exit position so it isn't accidentally reused
                                                connection.exit_position = None;
                                                connection.exit_rotation = None;
                                            }
                                            ClientZoneIpcData::ClientTrigger(trigger) => {
                                                // store the query for scripts
                                                if let ClientTriggerCommand::TeleportQuery { aetheryte_id } = trigger.trigger {
                                                    connection.player_data.teleport_query.aetheryte_id = aetheryte_id as u16;
                                                }

                                                // inform the server of our trigger, it will handle sending it to other clients
                                                connection.handle.send(ToServer::ClientTrigger(connection.id, connection.player_data.actor_id, trigger.clone())).await;
                                            }
                                            ClientZoneIpcData::Unk2 { .. } => {
                                                tracing::info!("Recieved Unk2!");
                                            }
                                            ClientZoneIpcData::Unk3 { .. } => {
                                                tracing::info!("Recieved Unk3!");
                                            }
                                            ClientZoneIpcData::Unk4 { .. } => {
                                                tracing::info!("Recieved Unk4!");
                                            }
                                            ClientZoneIpcData::SetSearchInfoHandler { .. } => {
                                                tracing::info!("Recieved SetSearchInfoHandler!");
                                            }
                                            ClientZoneIpcData::Unk5 { .. } => {
                                                tracing::info!("Recieved Unk5!");
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
                                                {
                                                    let parts: Vec<&str> = chat_message.message.split(' ').collect();
                                                    let command_name = &parts[0][1..];

                                                    let lua = lua.lock().unwrap();
                                                    let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

                                                    if let Some(command_script) =
                                                        state.command_scripts.get(command_name)
                                                        {
                                                            handled = true;

                                                            lua.scope(|scope| {
                                                                let connection_data = scope
                                                                .create_userdata_ref_mut(&mut lua_player)
                                                                .unwrap();

                                                                let config = get_config();

                                                                let file_name = format!(
                                                                    "{}/{}",
                                                                    &config.world.scripts_location, command_script
                                                                );
                                                                lua.load(
                                                                    std::fs::read(&file_name)
                                                                    .expect("Failed to locate scripts directory!"),
                                                                )
                                                                .set_name("@".to_string() + &file_name)
                                                                .exec()
                                                                .unwrap();

                                                                let func: Function =
                                                                lua.globals().get("onCommand").unwrap();

                                                                tracing::info!("{}", &chat_message.message[command_name.len() + 2..]);

                                                                func.call::<()>((&chat_message.message[command_name.len() + 2..], connection_data))
                                                                .unwrap();

                                                                Ok(())
                                                            })
                                                            .unwrap();
                                                        }
                                                }

                                                if !handled {
                                                    ChatHandler::handle_chat_message(
                                                        &mut connection,
                                                        &mut lua_player,
                                                        chat_message,
                                                    )
                                                    .await;
                                                }
                                            }
                                            ClientZoneIpcData::GMCommand { command, arg0, arg1, .. } => {
                                                tracing::info!("Got a game master command!");

                                                match &command {
                                                    GameMasterCommandType::SetLevel => {
                                                        connection.player_data.level = *arg0 as u8;
                                                        connection.update_class_info().await;
                                                    }
                                                    GameMasterCommandType::ChangeWeather => {
                                                        connection.change_weather(*arg0 as u16).await
                                                    }
                                                    GameMasterCommandType::ChangeTerritory => {
                                                        connection.change_zone(*arg0 as u16).await
                                                    }
                                                    GameMasterCommandType::ToggleInvisibility => {
                                                        connection
                                                        .actor_control_self(ActorControlSelf {
                                                            category:
                                                            ActorControlCategory::ToggleInvisibility {
                                                                invisible: true,
                                                            },
                                                        })
                                                        .await
                                                    }
                                                    GameMasterCommandType::ToggleWireframe => connection
                                                    .actor_control_self(ActorControlSelf {
                                                        category:
                                                        ActorControlCategory::ToggleWireframeRendering(),
                                                    })
                                                    .await,
                                                    GameMasterCommandType::GiveItem => {
                                                        connection.player_data.inventory.add_in_next_free_slot(Item { id: *arg0, quantity: 1 });
                                                        connection.send_inventory(false).await;
                                                    }
                                                    GameMasterCommandType::Aetheryte => {
                                                        let on = *arg0 == 0;
                                                        let id = *arg1;

                                                        // id == 0 means "all"
                                                        if id == 0 {
                                                            for i in 1..239 {
                                                                connection.actor_control_self(ActorControlSelf {
                                                                    category: ActorControlCategory::LearnTeleport { id: i, unlocked: on } }).await;
                                                            }
                                                        } else {
                                                            connection.actor_control_self(ActorControlSelf {
                                                                category: ActorControlCategory::LearnTeleport { id, unlocked: on } }).await;
                                                        }
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
                                                    let new_zone = Zone::load(&mut game_data.game_data, exit_box.territory_type);
                                                    let (destination_object, _) = new_zone
                                                    .find_pop_range(exit_box.destination_instance_id)
                                                    .unwrap();

                                                    // set the exit position
                                                    connection.exit_position = Some(Position {
                                                        x: destination_object.transform.translation[0],
                                                        y: destination_object.transform.translation[1],
                                                        z: destination_object.transform.translation[2],
                                                    });
                                                    new_territory = exit_box.territory_type;
                                                }

                                                connection.change_zone(new_territory).await;
                                            }
                                            ClientZoneIpcData::ActionRequest(request) => {
                                                let mut effects_builder = None;

                                                // run action script
                                                {
                                                    let lua = lua.lock().unwrap();
                                                    let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

                                                    let key = request.action_key;
                                                    if let Some(action_script) =
                                                        state.action_scripts.get(&key)
                                                        {
                                                            lua.scope(|scope| {
                                                                let connection_data = scope
                                                                .create_userdata_ref_mut(&mut lua_player)
                                                                .unwrap();

                                                                let config = get_config();

                                                                let file_name = format!(
                                                                    "{}/{}",
                                                                    &config.world.scripts_location, action_script
                                                                );
                                                                lua.load(
                                                                    std::fs::read(&file_name)
                                                                    .expect("Failed to locate scripts directory!"),
                                                                )
                                                                .set_name("@".to_string() + &file_name)
                                                                .exec()
                                                                .unwrap();

                                                                let func: Function =
                                                                lua.globals().get("doAction").unwrap();

                                                                effects_builder = Some(
                                                                    func.call::<EffectsBuilder>(connection_data)
                                                                    .unwrap(),
                                                                );

                                                                Ok(())
                                                            })
                                                            .unwrap();
                                                        } else {
                                                            tracing::warn!("Action {key} isn't scripted yet! Ignoring...");
                                                        }
                                                }

                                                // tell them the action results
                                                if let Some(effects_builder) = effects_builder {
                                                    let mut effects = [ActionEffect::default(); 8];
                                                    effects[..effects_builder.effects.len()]
                                                    .copy_from_slice(&effects_builder.effects);

                                                    if let Some(actor) =
                                                        connection.get_actor_mut(request.target.object_id)
                                                        {
                                                            for effect in &effects_builder.effects {
                                                                match effect.kind {
                                                                    EffectKind::Damage { amount, .. } => {
                                                                        actor.hp = actor.hp.saturating_sub(amount as u32);
                                                                    }
                                                                    _ => todo!()
                                                                }
                                                            }

                                                            let actor = *actor;
                                                            connection.update_hp_mp(actor.id, actor.hp, 10000).await;
                                                        }

                                                        let ipc = ServerZoneIpcSegment {
                                                            op_code: ServerZoneIpcType::ActionResult,
                                                            timestamp: timestamp_secs(),
                                                            data: ServerZoneIpcData::ActionResult(ActionResult {
                                                                main_target: request.target,
                                                                target_id_again: request.target,
                                                                action_id: request.action_key,
                                                                animation_lock_time: 0.6,
                                                                rotation: connection.player_data.rotation,
                                                                action_animation_id: request.action_key as u16, // assuming action id == animation id
                                                                flag: 1,
                                                                effect_count: effects_builder.effects.len() as u8,
                                                                                                  effects,
                                                                                                  unk1: 2662353,
                                                                                                  unk2: 3758096384,
                                                                                                  hidden_animation: 1,
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

                                                        if let Some(actor) =
                                                            connection.get_actor(request.target.object_id)
                                                            {
                                                                if actor.hp == 0 {
                                                                    tracing::info!("Despawning {} because they died!", actor.id.0);
                                                                    // if the actor died, despawn them
                                                                    /*connection.handle
                                                                     *                                       .send(ToServer::ActorDespawned(connection.id, actor.id.0))
                                                                     *                                       .await;*/
                                                                }
                                                            }
                                                }
                                            }
                                            ClientZoneIpcData::Unk16 { .. } => {
                                                tracing::info!("Recieved Unk16!");
                                            }
                                            ClientZoneIpcData::Unk17 { .. } => {
                                                tracing::info!("Recieved Unk17!");
                                            }
                                            ClientZoneIpcData::Unk18 { .. } => {
                                                tracing::info!("Recieved Unk18!");
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
                                                tracing::info!("Recieved Unk19!");
                                            }
                                            ClientZoneIpcData::ItemOperation(action) => {
                                                tracing::info!("Client is modifying inventory! {action:#?}");

                                                connection.player_data.inventory.process_action(action);
                                                connection.send_inventory(true).await;
                                            }
                                            ClientZoneIpcData::StartTalkEvent { actor_id, event_id } => {
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

                                                let mut should_cancel = false;
                                                {
                                                    let lua = lua.lock().unwrap();
                                                    let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

                                                    if let Some(event_script) =
                                                        state.event_scripts.get(event_id)
                                                        {
                                                            connection.event = Some(Event::new(*event_id, &event_script));
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
                                                }
                                            }
                                            ClientZoneIpcData::EventHandlerReturn { handler_id, scene, error_code, num_results, results } => {
                                                tracing::info!("Finishing this event... {handler_id} {scene} {error_code} {num_results} {results:#?}");

                                                connection
                                                .event
                                                .as_mut()
                                                .unwrap()
                                                .finish(*scene, results, &mut lua_player);
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
                                        panic!("The server is recieving a response or unknown packet!")
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
                    FromServer::Message(msg) => connection.send_message(&msg).await,
                    FromServer::ActorSpawn(actor, common) => connection.spawn_actor(actor, common).await,
                    FromServer::ActorMove(actor_id, position, rotation) => connection.set_actor_position(actor_id, position, rotation).await,
                    FromServer::ActorDespawn(actor_id) => connection.remove_actor(actor_id).await,
                    FromServer::ActorControl(actor_id, actor_control) => connection.actor_control(actor_id, actor_control).await,
                    FromServer::ActorControlTarget(actor_id, actor_control) => connection.actor_control_target(actor_id, actor_control).await,
                    FromServer::SpawnNPC(npc) => connection.send_npc(npc).await,
                    FromServer::ActorControlSelf(actor_control) => connection.actor_control_self(actor_control).await,
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
        let lua = lua.lock().unwrap();

        let register_action_func = lua
            .create_function(|lua, (action_id, action_script): (u32, String)| {
                tracing::info!("Registering {action_id} with {action_script}!");
                let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
                let _ = state.action_scripts.insert(action_id, action_script);
                Ok(())
            })
            .unwrap();

        let register_event_func = lua
            .create_function(|lua, (event_id, event_script): (u32, String)| {
                tracing::info!("Registering {event_id} with {event_script}!");
                let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
                let _ = state.event_scripts.insert(event_id, event_script);
                Ok(())
            })
            .unwrap();

        let register_command_func = lua
            .create_function(|lua, (command_name, command_script): (String, String)| {
                tracing::info!("Registering {command_name} with {command_script}!");
                let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
                let _ = state.command_scripts.insert(command_name, command_script);
                Ok(())
            })
            .unwrap();

        lua.set_app_data(ExtraLuaState::default());
        lua.globals()
            .set("registerAction", register_action_func)
            .unwrap();
        lua.globals()
            .set("registerEvent", register_event_func)
            .unwrap();
        lua.globals()
            .set("registerCommand", register_command_func)
            .unwrap();

        let effectsbuilder_constructor = lua
            .create_function(|_, ()| Ok(EffectsBuilder::default()))
            .unwrap();
        lua.globals()
            .set("EffectsBuilder", effectsbuilder_constructor)
            .unwrap();

        let file_name = format!("{}/Global.lua", &config.world.scripts_location);
        lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
            .set_name("@".to_string() + &file_name)
            .exec()
            .unwrap();
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
                    gracefully_logged_out: false
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
