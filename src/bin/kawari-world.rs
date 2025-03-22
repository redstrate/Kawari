use std::sync::Arc;

use kawari::common::custom_ipc::{CustomIpcData, CustomIpcSegment, CustomIpcType};
use kawari::config::get_config;
use kawari::oodle::OodleNetwork;
use kawari::packet::{
    CompressionType, ConnectionType, PacketSegment, PacketState, SegmentType, send_keep_alive,
    send_packet,
};
use kawari::world::ipc::{
    ClientZoneIpcData, CommonSpawn, GameMasterCommandType, ObjectKind, ServerZoneIpcData,
    ServerZoneIpcSegment, ServerZoneIpcType, SocialListRequestType, StatusEffect,
};
use kawari::world::{
    ChatHandler, Zone, ZoneConnection,
    ipc::{
        ActorControlCategory, ActorControlSelf, PlayerEntry, PlayerSetup, PlayerSpawn, PlayerStats,
        Position, SocialList,
    },
};
use kawari::world::{PlayerData, WorldDatabase};
use kawari::{CHAR_NAME, CITY_STATE, CONTENT_ID, WORLD_ID, ZONE_ID, common::timestamp_secs};
use physis::common::{Language, Platform};
use physis::gamedata::GameData;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7100").await.unwrap();

    tracing::info!("World server started on 127.0.0.1:7100");

    let database = Arc::new(WorldDatabase::new());

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let database = database.clone();

        let state = PacketState {
            client_key: None,
            clientbound_oodle: OodleNetwork::new(),
            serverbound_oodle: OodleNetwork::new(),
        };

        let mut exit_position = None;

        let mut connection = ZoneConnection {
            socket,
            state,
            player_data: PlayerData::default(),
            spawn_index: 0,
            zone: Zone::load(ZONE_ID),
        };

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = connection
                    .socket
                    .read(&mut buf)
                    .await
                    .expect("Failed to read data!");

                if n != 0 {
                    let (segments, connection_type) = connection.parse_packet(&buf[..n]).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::InitializeSession { actor_id } => {
                                tracing::info!("actor id to parse: {actor_id}");

                                // collect actor data
                                connection.player_data =
                                    database.find_player_data(actor_id.parse::<u32>().unwrap());

                                println!("player data: {:#?}", connection.player_data);

                                // We have send THEM a keep alive
                                {
                                    connection
                                        .send_segment(PacketSegment {
                                            source_actor: 0,
                                            target_actor: 0,
                                            segment_type: SegmentType::KeepAlive {
                                                id: 0xE0037603u32,
                                                timestamp: timestamp_secs(),
                                            },
                                        })
                                        .await;
                                }

                                match connection_type {
                                    kawari::packet::ConnectionType::Zone => {
                                        tracing::info!(
                                            "Client {actor_id} is initializing zone session..."
                                        );

                                        connection
                                            .send_segment(PacketSegment {
                                                source_actor: 0,
                                                target_actor: 0,
                                                segment_type: SegmentType::ZoneInitialize {
                                                    player_id: connection.player_data.actor_id,
                                                    timestamp: timestamp_secs(),
                                                },
                                            })
                                            .await;
                                    }
                                    kawari::packet::ConnectionType::Chat => {
                                        tracing::info!(
                                            "Client {actor_id} is initializing chat session..."
                                        );

                                        {
                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: 0,
                                                    target_actor: 0,
                                                    segment_type: SegmentType::ZoneInitialize {
                                                        player_id: connection.player_data.actor_id,
                                                        timestamp: timestamp_secs(),
                                                    },
                                                })
                                                .await;
                                        }

                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::InitializeChat,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::InitializeChat {
                                                    unk: [0; 8],
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
                                    }
                                    _ => panic!(
                                        "The client is trying to initialize the wrong connection?!"
                                    ),
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

                                        // Control Data
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::ActorControlSelf,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::ActorControlSelf(
                                                    ActorControlSelf {
                                                        category:
                                                            ActorControlCategory::SetCharaGearParamUI,
                                                        param1: 1,
                                                        param2: 1,
                                                        param3: 0,
                                                        param4: 0,
                                                        param5: 0,
                                                        param6: 0,
                                                    },
                                                ),
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

                                        // Stats
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::PlayerStats,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::PlayerStats(PlayerStats {
                                                    strength: 1,
                                                    hp: 100,
                                                    mp: 100,
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
                                            let chara_details = database
                                                .find_chara_make(connection.player_data.content_id);

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
                                                    gender: chara_details
                                                        .chara_make
                                                        .customize
                                                        .gender,
                                                    tribe: chara_details
                                                        .chara_make
                                                        .customize
                                                        .subrace,
                                                    city_state: CITY_STATE,
                                                    nameday_month: chara_details
                                                        .chara_make
                                                        .birth_month
                                                        as u8,
                                                    nameday_day: chara_details.chara_make.birth_day
                                                        as u8,
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

                                        connection.change_zone(ZONE_ID).await;

                                        // send welcome message
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::ServerChatMessage,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::ServerChatMessage {
                                                    message: "Welcome to Kawari!".to_string(),
                                                    unk: 0,
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
                                    }
                                    ClientZoneIpcData::FinishLoading { .. } => {
                                        tracing::info!(
                                            "Client has finished loading... spawning in!"
                                        );

                                        let chara_details = database
                                            .find_chara_make(connection.player_data.content_id);

                                        // send player spawn
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::PlayerSpawn,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::PlayerSpawn(PlayerSpawn {
                                                    content_id: CONTENT_ID,
                                                    common: CommonSpawn {
                                                        current_world_id: WORLD_ID,
                                                        home_world_id: WORLD_ID,
                                                        title: 1,
                                                        class_job: 35,
                                                        name: chara_details.name,
                                                        hp_curr: 100,
                                                        hp_max: 100,
                                                        mp_curr: 100,
                                                        mp_max: 100,
                                                        object_kind: ObjectKind::Player,
                                                        gm_rank: 3,
                                                        look: chara_details.chara_make.customize,
                                                        fc_tag: "LOCAL".to_string(),
                                                        subtype: 4,
                                                        models: [
                                                            0,  // head
                                                            89, // body
                                                            89, // hands
                                                            89, // legs
                                                            89, // feet
                                                            0,  // ears
                                                            0,  // neck
                                                            0,  // wrists
                                                            0,  // left finger
                                                            0,  // right finger
                                                        ],
                                                        pos: exit_position
                                                            .unwrap_or(Position::default()),
                                                        ..Default::default()
                                                    },
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
                                    }
                                    ClientZoneIpcData::Unk1 { .. } => {
                                        tracing::info!("Recieved Unk1!");
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
                                                    data: ServerZoneIpcData::SocialList(
                                                        SocialList {
                                                            request_type: request.request_type,
                                                            sequence: request.count,
                                                            entries: vec![PlayerEntry {
                                                                content_id: CONTENT_ID,
                                                                zone_id: connection.zone.id,
                                                                zone_id1: 0x0100,
                                                                class_job: 36,
                                                                level: 100,
                                                                one: 1,
                                                                name: CHAR_NAME.to_string(),
                                                                fc_tag: "LOCAL".to_string(),
                                                                ..Default::default()
                                                            }],
                                                        },
                                                    ),
                                                    ..Default::default()
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection
                                                            .player_data
                                                            .actor_id,
                                                        target_actor: connection
                                                            .player_data
                                                            .actor_id,
                                                        segment_type: SegmentType::Ipc {
                                                            data: ipc,
                                                        },
                                                    })
                                                    .await;
                                            }
                                            SocialListRequestType::Friends => {
                                                let ipc = ServerZoneIpcSegment {
                                                    op_code: ServerZoneIpcType::SocialList,
                                                    timestamp: timestamp_secs(),
                                                    data: ServerZoneIpcData::SocialList(
                                                        SocialList {
                                                            request_type: request.request_type,
                                                            sequence: request.count,
                                                            entries: Default::default(),
                                                        },
                                                    ),
                                                    ..Default::default()
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection
                                                            .player_data
                                                            .actor_id,
                                                        target_actor: connection
                                                            .player_data
                                                            .actor_id,
                                                        segment_type: SegmentType::Ipc {
                                                            data: ipc,
                                                        },
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                    ClientZoneIpcData::Unk7 {
                                        timestamp, unk1, ..
                                    } => {
                                        tracing::info!("Recieved Unk7! {:#?}", unk1);

                                        // send unk11 in response
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::Unk11,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::Unk11 {
                                                    timestamp: *timestamp,
                                                    unk: 333,
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
                                    }
                                    ClientZoneIpcData::UpdatePositionHandler { .. } => {
                                        tracing::info!("Recieved UpdatePositionHandler!");
                                    }
                                    ClientZoneIpcData::LogOut { .. } => {
                                        tracing::info!("Recieved log out from client!");

                                        // tell the client to disconnect
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::LogOutComplete,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::LogOutComplete {
                                                    unk: [0; 8],
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
                                    }
                                    ClientZoneIpcData::Disconnected { .. } => {
                                        tracing::info!("Client disconnected!");
                                    }
                                    ClientZoneIpcData::ChatMessage(chat_message) => {
                                        ChatHandler::handle_chat_message(
                                            &mut connection,
                                            chat_message,
                                        )
                                        .await
                                    }
                                    ClientZoneIpcData::GameMasterCommand {
                                        command, arg, ..
                                    } => {
                                        tracing::info!("Got a game master command!");

                                        match &command {
                                            GameMasterCommandType::ChangeWeather => {
                                                connection.change_weather(*arg as u16).await
                                            }
                                            GameMasterCommandType::ChangeTerritory => {
                                                connection.change_zone(*arg as u16).await
                                            }
                                        }
                                    }
                                    ClientZoneIpcData::Unk12 { .. } => {
                                        tracing::info!("Recieved Unk12!");
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
                                                .find_exit_box(*exit_box_id)
                                                .unwrap();
                                            tracing::info!("exit box: {:#?}", exit_box);

                                            // find the pop range on the other side
                                            let new_zone = Zone::load(exit_box.territory_type);
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

                                        // fade out?
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::PrepareZoning,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::PrepareZoning {
                                                    unk: [0x01000000, 0, 0, 0],
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

                                        // fade out? x2
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::PrepareZoning,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::PrepareZoning {
                                                    unk: [0, 0x00000085, 0x00030000, 0x000008ff], // last thing is probably a float?
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

                                        tracing::info!("sending them to {:#?}", new_territory);

                                        connection.change_zone(new_territory).await;
                                    }
                                    ClientZoneIpcData::Unk13 { .. } => {
                                        tracing::info!("Recieved Unk13!");
                                    }
                                    ClientZoneIpcData::Unk14 { .. } => {
                                        tracing::info!("Recieved Unk14!");
                                    }
                                    ClientZoneIpcData::ActionRequest(request) => {
                                        tracing::info!("Recieved action request: {:#?}!", request);

                                        let config = get_config();

                                        let mut game_data = GameData::from_existing(
                                            Platform::Win32,
                                            &config.game_location,
                                        )
                                        .unwrap();

                                        let exh =
                                            game_data.read_excel_sheet_header("Action").unwrap();
                                        let exd = game_data
                                            .read_excel_sheet("Action", &exh, Language::English, 0)
                                            .unwrap();

                                        let action_row =
                                            &exd.read_row(&exh, request.action_id).unwrap()[0];

                                        println!("Found action: {:#?}", action_row);

                                        // send new status list
                                        {
                                            let ipc = ServerZoneIpcSegment {
                                                op_code: ServerZoneIpcType::StatusEffectList,
                                                timestamp: timestamp_secs(),
                                                data: ServerZoneIpcData::StatusEffectList(
                                                    kawari::world::ipc::StatusEffectList {
                                                        statues: [StatusEffect {
                                                            effect_id: 50,
                                                            param: 0,
                                                            duration: 50.0,
                                                            source_actor_id: connection
                                                                .player_data
                                                                .actor_id,
                                                        };
                                                            30],
                                                        ..Default::default()
                                                    },
                                                ),
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
                                        tracing::info!(
                                            "creating character from: {name} {chara_make_json}"
                                        );

                                        let (content_id, actor_id) =
                                            database.create_player_data(name, chara_make_json);

                                        tracing::info!(
                                            "Created new player: {content_id} {actor_id}"
                                        );

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
                                                            op_code:
                                                                CustomIpcType::CharacterCreated,
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
                                                            data: CustomIpcData::ActorIdFound {
                                                                actor_id,
                                                            },
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
                                        let characters =
                                            database.get_character_list(*service_account_id);

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
                                    _ => panic!(
                                        "The server is recieving a response or unknown custom IPC!"
                                    ),
                                }
                            }
                            _ => {
                                panic!("The server is recieving a response or unknown packet!")
                            }
                        }
                    }
                }
            }
        });
    }
}
