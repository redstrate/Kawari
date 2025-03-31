use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use kawari::RECEIVE_BUFFER_SIZE;
use kawari::common::custom_ipc::{CustomIpcData, CustomIpcSegment, CustomIpcType};
use kawari::common::{GameData, ObjectId, timestamp_secs};
use kawari::common::{Position, determine_initial_starting_zone};
use kawari::config::get_config;
use kawari::lobby::CharaMake;
use kawari::oodle::OodleNetwork;
use kawari::opcodes::ServerZoneIpcType;
use kawari::packet::{
    CompressionType, ConnectionType, PacketSegment, PacketState, SegmentType, send_keep_alive,
    send_packet,
};
use kawari::world::ipc::{
    ActionEffect, ActionResult, ClientZoneIpcData, CommonSpawn, DisplayFlag, EffectKind,
    GameMasterCommandType, GameMasterRank, ObjectKind, OnlineStatus, PlayerSubKind,
    ServerZoneIpcData, ServerZoneIpcSegment, SocialListRequestType,
};
use kawari::world::{
    Actor, ClientHandle, ClientId, EffectsBuilder, FromServer, LuaPlayer, PlayerData, ServerHandle,
    StatusEffects, ToServer, WorldDatabase,
};
use kawari::world::{
    ChatHandler, Inventory, Zone, ZoneConnection,
    ipc::{
        ActorControlCategory, ActorControlSelf, PlayerEntry, PlayerSetup, PlayerSpawn, PlayerStats,
        SocialList,
    },
};

use mlua::{Function, Lua};
use tokio::io::AsyncReadExt;
use tokio::join;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{Receiver, UnboundedReceiver, UnboundedSender, channel, unbounded_channel};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

#[derive(Default)]
struct ExtraLuaState {
    action_scripts: HashMap<u32, String>,
}

#[derive(Default, Debug)]
struct Data {
    clients: HashMap<ClientId, ClientHandle>,
}

async fn main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let mut data = Data::default();
    let mut to_remove = Vec::new();

    while let Some(msg) = recv.recv().await {
        match msg {
            ToServer::NewClient(handle) => {
                data.clients.insert(handle.id, handle);
            }
            ToServer::Message(from_id, msg) => {
                for (id, handle) in &mut data.clients {
                    let id = *id;

                    if id == from_id {
                        continue;
                    }

                    let msg = FromServer::Message(msg.clone());

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::ActorSpawned(from_id, actor) => {
                for (id, handle) in &mut data.clients {
                    let id = *id;

                    if id == from_id {
                        continue;
                    }

                    let msg = FromServer::ActorSpawn(actor);

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::ActorMoved(from_id, actor_id, position) => {
                for (id, handle) in &mut data.clients {
                    let id = *id;

                    if id == from_id {
                        continue;
                    }

                    let msg = FromServer::ActorMove(actor_id, position);

                    if handle.send(msg).is_err() {
                        to_remove.push(id);
                    }
                }
            }
            ToServer::FatalError(err) => return Err(err),
        }
    }
    // Remove any clients that errored out
    for id in to_remove {
        data.clients.remove(&id);
    }
    Ok(())
}

fn spawn_main_loop() -> (ServerHandle, JoinHandle<()>) {
    let (send, recv) = channel(64);

    let handle = ServerHandle {
        chan: send,
        next_id: Default::default(),
    };

    let join = tokio::spawn(async move {
        let res = main_loop(recv).await;
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
    id: ClientId,
    handle: ServerHandle,
    /// Socket for data recieved from the global server
    recv: Receiver<FromServer>,
    connection: ZoneConnection,
}

/// Spawn a new client actor.
pub fn spawn_client(connection: ZoneConnection) {
    let (send, recv) = channel(64);

    let id = &connection.id.clone();
    let ip = &connection.ip.clone();

    let data = ClientData {
        id: connection.id,
        handle: connection.handle.clone(),
        recv,
        connection,
    };

    // Spawn a new client task
    let (my_send, my_recv) = oneshot::channel();
    let kill = tokio::spawn(start_client(my_recv, data));

    // Send client information to said task
    let handle = ClientHandle {
        id: *id,
        ip: *ip,
        channel: send,
        //kill,
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
    loop {
        match data.recv().await {
            Some(msg) => internal_send.send(msg).unwrap(),
            None => break,
        }
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

    let mut exit_position = None;
    let mut exit_rotation = None;

    let mut lua_player = LuaPlayer::default();

    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    loop {
        tokio::select! {
            biased; // client data should always be prioritized
            Ok(n) = connection.socket.read(&mut buf) => {
                if n > 0 {
                    let (segments, connection_type) = connection.parse_packet(&buf[..n]).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::InitializeSession { actor_id } => {
                                // for some reason they send a string representation
                                let actor_id = actor_id.parse::<u32>().unwrap();

                                // initialize player data if it doesn't exist'
                                if connection.player_data.actor_id == 0 {
                                    connection.player_data = database.find_player_data(actor_id);
                                }

                                // collect actor data
                                connection.initialize(&connection_type, actor_id).await;

                                if connection_type == ConnectionType::Zone {
                                    exit_position = Some(connection.player_data.position);
                                    exit_rotation = Some(connection.player_data.rotation);

                                    // tell the server we exist, now that we confirmed we are a legitimate connection
                                    connection.handle.send(ToServer::NewClient(client_handle.clone())).await;
                                }
                            }
                            SegmentType::Ipc { data } => {
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
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        let chara_details =
                                            database.find_chara_make(connection.player_data.content_id);

                                        // fill inventory
                                        connection.inventory.equip_racial_items(
                                            chara_details.chara_make.customize.race,
                                            chara_details.chara_make.customize.gender,
                                        );

                                        // Send inventory
                                        connection.send_inventory().await;

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
                                        {
                                            let attributes;
                                            {
                                                let mut game_data = game_data.lock().unwrap();

                                                attributes = game_data.get_racial_base_attributes(
                                                    chara_details.chara_make.customize.subrace,
                                                );
                                            }

                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::PlayerStats,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::PlayerStats(PlayerStats {
                                                    strength: attributes.strength,
                                                    dexterity: attributes.dexterity,
                                                    vitality: attributes.vitality,
                                                    intelligence: attributes.intelligence,
                                                    mind: attributes.mind,
                                                    hp: connection.player_data.max_hp,
                                                    mp: connection.player_data.max_mp as u32,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // Player Setup
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::PlayerSetup,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::PlayerSetup(PlayerSetup {
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
                                                    segment_type: SegmentType::Ipc { data: ipc },
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
                                        let chara_details =
                                            database.find_chara_make(connection.player_data.content_id);

                                        // send player spawn
                                        {
                                            let ipc;
                                            {
                                                let mut game_data = game_data.lock().unwrap();
                                                let equipped = &connection.inventory.equipped;

                                                ipc = ServerZoneIpcSegment {
                                                    op_code: ServerZoneIpcType::PlayerSpawn,
                                                    timestamp: timestamp_secs(),
                                                    data: ServerZoneIpcData::PlayerSpawn(PlayerSpawn {
                                                        account_id: connection.player_data.account_id,
                                                        content_id: connection.player_data.content_id,
                                                        current_world_id: config.world.world_id,
                                                        home_world_id: config.world.world_id,
                                                        gm_rank: GameMasterRank::Debug,
                                                        online_status: OnlineStatus::GameMasterBlue,
                                                        common: CommonSpawn {
                                                            class_job: connection.player_data.classjob_id,
                                                            name: chara_details.name,
                                                            hp_curr: connection.player_data.curr_hp,
                                                            hp_max: connection.player_data.max_hp,
                                                            mp_curr: connection.player_data.curr_mp,
                                                            mp_max: connection.player_data.max_mp,
                                                            object_kind: ObjectKind::Player(
                                                                PlayerSubKind::Player,
                                                            ),
                                                            look: chara_details.chara_make.customize,
                                                            fc_tag: "LOCAL".to_string(),
                                                            display_flags: DisplayFlag::UNK,
                                                            models: [
                                                                game_data
                                                                    .get_primary_model_id(equipped.head.id)
                                                                    as u32,
                                                                game_data
                                                                    .get_primary_model_id(equipped.body.id)
                                                                    as u32,
                                                                game_data
                                                                    .get_primary_model_id(equipped.hands.id)
                                                                    as u32,
                                                                game_data
                                                                    .get_primary_model_id(equipped.legs.id)
                                                                    as u32,
                                                                game_data
                                                                    .get_primary_model_id(equipped.feet.id)
                                                                    as u32,
                                                                game_data
                                                                    .get_primary_model_id(equipped.ears.id)
                                                                    as u32,
                                                                game_data
                                                                    .get_primary_model_id(equipped.neck.id)
                                                                    as u32,
                                                                game_data.get_primary_model_id(
                                                                    equipped.wrists.id,
                                                                )
                                                                    as u32,
                                                                game_data.get_primary_model_id(
                                                                    equipped.left_ring.id,
                                                                )
                                                                    as u32,
                                                                game_data.get_primary_model_id(
                                                                    equipped.right_ring.id,
                                                                )
                                                                    as u32,
                                                            ],
                                                            pos: exit_position
                                                                .unwrap_or(Position::default()),
                                                            rotation: exit_rotation.unwrap_or(0.0),
                                                            ..Default::default()
                                                        },
                                                        ..Default::default()
                                                    }),
                                                    ..Default::default()
                                                };
                                            }

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
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
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // wipe any exit position so it isn't accidentally reused
                                        exit_position = None;
                                        exit_rotation = None;

                                        // tell the other players we're here
                                        connection.handle.send(ToServer::ActorSpawned(connection.id, Actor { id: ObjectId(connection.player_data.actor_id), hp: 100 })).await;
                                    }
                                    ClientZoneIpcData::Unk1 {
                                        category, param1, ..
                                    } => {
                                        tracing::info!("Recieved Unk1! {category:#?}");

                                        match category {
                                            3 => {
                                                // set target
                                                tracing::info!("Targeting actor {param1}");
                                            }
                                            _ => {}
                                        }
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
                                                            fc_tag: "LOCAL".to_string(),
                                                            ..Default::default()
                                                        }],
                                                    }),
                                                    ..Default::default()
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_data.actor_id,
                                                        target_actor: connection.player_data.actor_id,
                                                        segment_type: SegmentType::Ipc { data: ipc },
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
                                                        segment_type: SegmentType::Ipc { data: ipc },
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                    ClientZoneIpcData::UpdatePositionHandler { position, rotation } => {
                                        tracing::info!(
                                            "Character moved to {position:#?} {}",
                                            rotation.to_degrees()
                                        );

                                        connection.player_data.rotation = *rotation;
                                        connection.player_data.position = *position;

                                        connection.handle.send(ToServer::ActorMoved(connection.id, connection.player_data.actor_id, *position)).await;
                                    }
                                    ClientZoneIpcData::LogOut { .. } => {
                                        tracing::info!("Recieved log out from client!");

                                        // write the player back to the database
                                        database.commit_player_data(&connection.player_data);

                                        // tell the client to disconnect
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::LogOutComplete,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::LogOutComplete { unk: [0; 8] },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }
                                    }
                                    ClientZoneIpcData::Disconnected { .. } => {
                                        tracing::info!("Client disconnected!");
                                    }
                                    ClientZoneIpcData::ChatMessage(chat_message) => {
                                        connection.handle.send(ToServer::Message(connection.id, chat_message.message.clone())).await;

                                        ChatHandler::handle_chat_message(
                                            &mut connection,
                                            &mut lua_player,
                                            chat_message,
                                        )
                                        .await
                                    }
                                    ClientZoneIpcData::GameMasterCommand { command, arg, .. } => {
                                        tracing::info!("Got a game master command!");

                                        match &command {
                                            GameMasterCommandType::ChangeWeather => {
                                                connection.change_weather(*arg as u16).await
                                            }
                                            GameMasterCommandType::ChangeTerritory => {
                                                connection.change_zone(*arg as u16).await
                                            }
                                            GameMasterCommandType::ToggleInvisibility => {
                                                connection
                                                    .actor_control_self(ActorControlSelf {
                                                        category:
                                                            ActorControlCategory::ToggleInvisibility {
                                                                invisible: 1,
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
                                        }
                                    }
                                    ClientZoneIpcData::EnterZoneLine {
                                        exit_box_id,
                                        position,
                                        ..
                                    } => {
                                        tracing::info!(
                                            "Character entered {exit_box_id} with a position of {position:#?}!"
                                        );

                                        // find the exit box id
                                        let new_territory;
                                        {
                                            let (_, exit_box) = connection
                                                .zone
                                                .as_ref()
                                                .unwrap()
                                                .find_exit_box(*exit_box_id)
                                                .unwrap();

                                            // find the pop range on the other side
                                            let mut game_data = game_data.lock().unwrap();
                                            let new_zone = Zone::load(&mut game_data.game_data, exit_box.territory_type);
                                            let (destination_object, _) = new_zone
                                                .find_pop_range(exit_box.destination_instance_id)
                                                .unwrap();

                                            // set the exit position
                                            exit_position = Some(Position {
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

                                            if let Some(action_script) =
                                                state.action_scripts.get(&request.action_id)
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
                                            }
                                        }

                                        // tell them the action results
                                        if let Some(effects_builder) = effects_builder {
                                            let mut effects = [ActionEffect::default(); 8];
                                            effects[..effects_builder.effects.len()]
                                                .copy_from_slice(&effects_builder.effects);

                                            if let Some(actor) =
                                                connection.get_actor(request.target.object_id)
                                            {
                                                for effect in &effects_builder.effects {
                                                    match effect.kind {
                                                        EffectKind::Damage => {
                                                            actor.hp -= effect.value as u32;
                                                        }
                                                        _ => todo!()
                                                    }
                                                }

                                                let actor = actor.clone();
                                                connection.update_hp_mp(actor.id, actor.hp, 10000).await;
                                            }

                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::ActionResult,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::ActionResult(ActionResult {
                                                    main_target: request.target,
                                                    target_id_again: request.target,
                                                    action_id: request.action_id,
                                                    animation_lock_time: 0.6,
                                                    rotation: connection.player_data.rotation,
                                                    action_animation_id: request.action_id as u16, // assuming action id == animation id
                                                    flag: 1,
                                                    effect_count: effects_builder.effects.len() as u8,
                                                    effects,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_data.actor_id,
                                                    target_actor: connection.player_data.actor_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
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
                                }
                            }
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive::<ServerZoneIpcSegment>(
                                    &mut connection.socket,
                                    &mut connection.state,
                                    ConnectionType::Zone,
                                    *id,
                                    *timestamp,
                                )
                                .await
                            }
                            SegmentType::KeepAliveResponse { .. } => {
                                tracing::info!("Got keep alive response from client... cool...");
                            }
                            SegmentType::CustomIpc { data } => {
                                match &data.data {
                                    CustomIpcData::RequestCreateCharacter {
                                        name,
                                        chara_make_json,
                                    } => {
                                        tracing::info!("creating character from: {name} {chara_make_json}");

                                        let chara_make = CharaMake::from_json(chara_make_json);

                                        let city_state;
                                        {
                                            let mut game_data = game_data.lock().unwrap();

                                            city_state =
                                                game_data.get_citystate(chara_make.classjob_id as u16);
                                        }

                                        let (content_id, actor_id) = database.create_player_data(
                                            name,
                                            chara_make_json,
                                            city_state,
                                            determine_initial_starting_zone(city_state),
                                        );

                                        tracing::info!("Created new player: {content_id} {actor_id}");

                                        // send them the new actor and content id
                                        {
                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: 0,
                                                    target_actor: 0,
                                                    segment_type: SegmentType::CustomIpc {
                                                        data: CustomIpcSegment {
                                                            unk1: 0,
                                                            unk2: 0,
                                                            op_code: CustomIpcType::CharacterCreated,
                                                            server_id: 0,
                                                            timestamp: 0,
                                                            data: CustomIpcData::CharacterCreated {
                                                                actor_id,
                                                                content_id,
                                                            },
                                                        },
                                                    },
                                                })
                                                .await;
                                        }
                                    }
                                    CustomIpcData::GetActorId { content_id } => {
                                        let actor_id = database.find_actor_id(*content_id);

                                        tracing::info!("We found an actor id: {actor_id}");

                                        // send them the actor id
                                        {
                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: 0,
                                                    target_actor: 0,
                                                    segment_type: SegmentType::CustomIpc {
                                                        data: CustomIpcSegment {
                                                            unk1: 0,
                                                            unk2: 0,
                                                            op_code: CustomIpcType::ActorIdFound,
                                                            server_id: 0,
                                                            timestamp: 0,
                                                            data: CustomIpcData::ActorIdFound { actor_id },
                                                        },
                                                    },
                                                })
                                                .await;
                                        }
                                    }
                                    CustomIpcData::CheckNameIsAvailable { name } => {
                                        let is_name_free = database.check_is_name_free(name);
                                        let is_name_free = if is_name_free { 1 } else { 0 };

                                        // send response
                                        {
                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: 0,
                                                    target_actor: 0,
                                                    segment_type: SegmentType::CustomIpc {
                                                        data: CustomIpcSegment {
                                                            unk1: 0,
                                                            unk2: 0,
                                                            op_code: CustomIpcType::NameIsAvailableResponse,
                                                            server_id: 0,
                                                            timestamp: 0,
                                                            data: CustomIpcData::NameIsAvailableResponse {
                                                                free: is_name_free,
                                                            },
                                                        },
                                                    },
                                                })
                                                .await;
                                        }
                                    }
                                    CustomIpcData::RequestCharacterList { service_account_id } => {
                                        let config = get_config();

                                        let world_name;
                                        {
                                            let mut game_data = game_data.lock().unwrap();
                                            world_name = game_data.get_world_name(config.world.world_id);
                                        }

                                        let characters = database.get_character_list(
                                            *service_account_id,
                                            config.world.world_id,
                                            &world_name,
                                        );

                                        // send response
                                        {
                                            send_packet::<CustomIpcSegment>(
                                                        &mut connection.socket,
                                                        &mut connection.state,
                                                        ConnectionType::None,
                                                        CompressionType::Uncompressed,
                                                        &[PacketSegment {
                                                            source_actor: 0,
                                                            target_actor: 0,
                                                            segment_type: SegmentType::CustomIpc {
                                                                data: CustomIpcSegment {
                                                                    unk1: 0,
                                                                    unk2: 0,
                                                                    op_code: CustomIpcType::RequestCharacterListRepsonse,
                                                                    server_id: 0,
                                                                    timestamp: 0,
                                                                    data: CustomIpcData::RequestCharacterListRepsonse {
                                                                        characters
                                                                    },
                                                                },
                                                            },
                                                        }],
                                                    )
                                                    .await;
                                        }
                                    }
                                    CustomIpcData::DeleteCharacter { content_id } => {
                                        database.delete_character(*content_id);

                                        // send response
                                        {
                                            send_packet::<CustomIpcSegment>(
                                                &mut connection.socket,
                                                &mut connection.state,
                                                ConnectionType::None,
                                                CompressionType::Uncompressed,
                                                &[PacketSegment {
                                                    source_actor: 0,
                                                    target_actor: 0,
                                                    segment_type: SegmentType::CustomIpc {
                                                        data: CustomIpcSegment {
                                                            unk1: 0,
                                                            unk2: 0,
                                                            op_code: CustomIpcType::CharacterDeleted,
                                                            server_id: 0,
                                                            timestamp: 0,
                                                            data: CustomIpcData::CharacterDeleted {
                                                                deleted: 1,
                                                            },
                                                        },
                                                    },
                                                }],
                                            )
                                            .await;
                                        }
                                    }
                                    _ => {
                                        panic!("The server is recieving a response or unknown custom IPC!")
                                    }
                                }
                            }
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
                    lua_player.player_data = connection.player_data;
                    lua_player.status_effects = connection.status_effects.clone();
                }
            }
            msg = internal_recv.recv() => match msg {
                Some(msg) => match msg {
                    FromServer::Message(msg)=>connection.send_message(&msg).await,
                    FromServer::ActorSpawn(actor) => {
                        connection.spawn_actor(actor).await
                    },
                    FromServer::ActorMove(actor_id, position) => connection.set_actor_position(actor_id, position).await,
                },
                None => break,
            }
        }
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
        let lua = lua.lock().unwrap();

        let register_action_func = lua
            .create_function(|lua, (action_id, action_script): (u32, String)| {
                tracing::info!("Registering {action_id} with {action_script}!");
                let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
                let _ = state.action_scripts.insert(action_id, action_script);
                Ok(())
            })
            .unwrap();

        lua.set_app_data(ExtraLuaState::default());
        lua.globals()
            .set("registerAction", register_action_func)
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
        let (socket, ip) = listener.accept().await.unwrap();
        let id = handle.next_id();

        let state = PacketState {
            client_key: None,
            clientbound_oodle: OodleNetwork::new(),
            serverbound_oodle: OodleNetwork::new(),
        };

        spawn_client(ZoneConnection {
            socket,
            state,
            player_data: PlayerData::default(),
            spawn_index: 0,
            zone: None,
            inventory: Inventory::new(),
            status_effects: StatusEffects::default(),
            event: None,
            actors: Vec::new(),
            ip,
            id,
            handle: handle.clone(),
            database: database.clone(),
            lua: lua.clone(),
            gamedata: game_data.clone(),
        });
    }
}
