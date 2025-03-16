use std::time::{SystemTime, UNIX_EPOCH};

use kawari::ipc::{GameMasterCommandType, IPCOpCode, IPCSegment, IPCStructData};
use kawari::oodle::FFXIVOodle;
use kawari::packet::{PacketSegment, SegmentType, State, send_keep_alive};
use kawari::world::{
    ActorControlSelf, ActorControlType, ChatHandler, PlayerEntry, PlayerSetup, PlayerSpawn,
    PlayerStats, Position, SocialList, Zone, ZoneConnection,
};
use kawari::{
    CHAR_NAME, CITY_STATE, CONTENT_ID, CUSTOMIZE_DATA, DEITY, NAMEDAY_DAY, NAMEDAY_MONTH, WORLD_ID,
    ZONE_ID, common::timestamp_secs,
};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7100").await.unwrap();

    tracing::info!("World server started on 127.0.0.1:7100");

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let state = State {
            client_key: None,
            session_id: None,
            clientbound_oodle: FFXIVOodle::new(),
            serverbound_oodle: FFXIVOodle::new(),
        };

        let mut exit_position = None;

        let mut connection = ZoneConnection {
            socket,
            state,
            player_id: 0,
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
                    println!("recieved {n} bytes...");
                    let (segments, connection_type) = connection.parse_packet(&buf[..n]).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::InitializeSession { player_id } => {
                                connection.player_id = *player_id;

                                // We have send THEM a keep alive
                                {
                                    let timestamp: u32 = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .expect("Failed to get UNIX timestamp!")
                                        .as_secs()
                                        .try_into()
                                        .unwrap();

                                    connection
                                        .send_segment(PacketSegment {
                                            source_actor: 0,
                                            target_actor: 0,
                                            segment_type: SegmentType::KeepAlive {
                                                id: 0xE0037603u32,
                                                timestamp,
                                            },
                                        })
                                        .await;
                                }

                                match connection_type {
                                    kawari::packet::ConnectionType::Zone => {
                                        tracing::info!(
                                            "Client {player_id} is initializing zone session..."
                                        );

                                        connection
                                            .send_segment(PacketSegment {
                                                source_actor: 0,
                                                target_actor: 0,
                                                segment_type: SegmentType::ZoneInitialize {
                                                    player_id: *player_id,
                                                    timestamp: timestamp_secs(),
                                                },
                                            })
                                            .await;
                                    }
                                    kawari::packet::ConnectionType::Chat => {
                                        tracing::info!(
                                            "Client {player_id} is initializing chat session..."
                                        );

                                        {
                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: 0,
                                                    target_actor: 0,
                                                    segment_type: SegmentType::ZoneInitialize {
                                                        player_id: *player_id,
                                                        timestamp: timestamp_secs(),
                                                    },
                                                })
                                                .await;
                                        }

                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::InitializeChat,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitializeChat { unk: [0; 8] },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: *player_id,
                                                    target_actor: *player_id,
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
                                    IPCStructData::InitRequest { .. } => {
                                        tracing::info!(
                                            "Client is now requesting zone information. Sending!"
                                        );

                                        // IPC Init(?)
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::Unk5,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitResponse {
                                                    unk1: 0,
                                                    character_id: connection.player_id,
                                                    unk2: 0,
                                                },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // Control Data
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::ActorControlSelf,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::ActorControlSelf(
                                                    ActorControlSelf {
                                                        category:
                                                            ActorControlType::SetCharaGearParamUI,
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
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // Stats
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::PlayerStats,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerStats(PlayerStats {
                                                    strength: 1,
                                                    hp: 100,
                                                    mp: 100,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // Player Setup
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::PlayerSetup,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerSetup(PlayerSetup {
                                                    content_id: CONTENT_ID,
                                                    exp: [10000; 32],
                                                    levels: [100; 32],
                                                    name: CHAR_NAME.to_string(),
                                                    char_id: connection.player_id,
                                                    race: CUSTOMIZE_DATA.race,
                                                    gender: CUSTOMIZE_DATA.gender,
                                                    tribe: CUSTOMIZE_DATA.subrace,
                                                    city_state: CITY_STATE,
                                                    nameday_month: NAMEDAY_MONTH,
                                                    nameday_day: NAMEDAY_DAY,
                                                    deity: DEITY,
                                                    ..Default::default()
                                                }),
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        connection.change_zone(ZONE_ID).await;

                                        // send welcome message
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::ServerChatMessage,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::ServerChatMessage {
                                                    message: "Welcome to Kawari!".to_string(),
                                                    unk: 0,
                                                },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }
                                    }
                                    IPCStructData::FinishLoading { .. } => {
                                        tracing::info!(
                                            "Client has finished loading... spawning in!"
                                        );

                                        // send player spawn
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::PlayerSpawn,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerSpawn(PlayerSpawn {
                                                    content_id: CONTENT_ID,
                                                    current_world_id: WORLD_ID,
                                                    home_world_id: WORLD_ID,
                                                    title: 1,
                                                    class_job: 35,
                                                    name: CHAR_NAME.to_string(),
                                                    hp_curr: 100,
                                                    hp_max: 100,
                                                    mp_curr: 100,
                                                    mp_max: 100,
                                                    model_type: 1,
                                                    gm_rank: 3,
                                                    look: CUSTOMIZE_DATA,
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
                                                }),
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // fade in?
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::PrepareZoning,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PrepareZoning {
                                                    unk: [0, 0, 0, 0],
                                                },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // wipe any exit position so it isn't accidentally reused
                                        exit_position = None;
                                    }
                                    IPCStructData::Unk1 { .. } => {
                                        tracing::info!("Recieved Unk1!");
                                    }
                                    IPCStructData::Unk2 { .. } => {
                                        tracing::info!("Recieved Unk2!");
                                    }
                                    IPCStructData::Unk3 { .. } => {
                                        tracing::info!("Recieved Unk3!");
                                    }
                                    IPCStructData::Unk4 { .. } => {
                                        tracing::info!("Recieved Unk4!");
                                    }
                                    IPCStructData::SetSearchInfoHandler { .. } => {
                                        tracing::info!("Recieved SetSearchInfoHandler!");
                                    }
                                    IPCStructData::Unk5 { .. } => {
                                        tracing::info!("Recieved Unk5!");
                                    }
                                    IPCStructData::SocialListRequest(request) => {
                                        tracing::info!("Recieved social list request!");

                                        match &request.request_type {
                                            kawari::world::SocialListRequestType::Party => {
                                                let ipc = IPCSegment {
                                                    op_code: IPCOpCode::SocialList,
                                                    timestamp: timestamp_secs(),
                                                    data: IPCStructData::SocialList(SocialList {
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
                                                    }),
                                                    ..Default::default()
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_id,
                                                        target_actor: connection.player_id,
                                                        segment_type: SegmentType::Ipc {
                                                            data: ipc,
                                                        },
                                                    })
                                                    .await;
                                            }
                                            kawari::world::SocialListRequestType::Friends => {
                                                let ipc = IPCSegment {
                                                    op_code: IPCOpCode::SocialList,
                                                    timestamp: timestamp_secs(),
                                                    data: IPCStructData::SocialList(SocialList {
                                                        request_type: request.request_type,
                                                        sequence: request.count,
                                                        entries: Default::default(),
                                                    }),
                                                    ..Default::default()
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: connection.player_id,
                                                        target_actor: connection.player_id,
                                                        segment_type: SegmentType::Ipc {
                                                            data: ipc,
                                                        },
                                                    })
                                                    .await;
                                            }
                                        }
                                    }
                                    IPCStructData::Unk7 {
                                        timestamp, unk1, ..
                                    } => {
                                        tracing::info!("Recieved Unk7! {:#?}", unk1);

                                        // send unk11 in response
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::Unk11,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::Unk11 {
                                                    timestamp: *timestamp,
                                                    unk: 333,
                                                },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }
                                    }
                                    IPCStructData::UpdatePositionHandler { .. } => {
                                        tracing::info!("Recieved UpdatePositionHandler!");
                                    }
                                    IPCStructData::LogOut { .. } => {
                                        tracing::info!("Recieved log out from client!");

                                        // tell the client to disconnect
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::LogOutComplete,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::LogOutComplete { unk: [0; 8] },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }
                                    }
                                    IPCStructData::Disconnected { .. } => {
                                        tracing::info!("Client disconnected!");
                                    }
                                    IPCStructData::ChatMessage(chat_message) => {
                                        ChatHandler::handle_chat_message(
                                            &mut connection,
                                            chat_message,
                                        )
                                        .await
                                    }
                                    IPCStructData::GameMasterCommand { command, arg, .. } => {
                                        tracing::info!("Got a game master command!");

                                        match &command {
                                            GameMasterCommandType::ChangeTerritory => {
                                                connection.change_zone(*arg as u16).await
                                            }
                                        }
                                    }
                                    IPCStructData::Unk12 { .. } => {
                                        tracing::info!("Recieved Unk12!");
                                    }
                                    IPCStructData::EnterZoneLine {
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
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::PrepareZoning,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PrepareZoning {
                                                    unk: [0x01000000, 0, 0, 0],
                                                },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        // fade out? x2
                                        {
                                            let ipc = IPCSegment {
                                                op_code: IPCOpCode::PrepareZoning,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PrepareZoning {
                                                    unk: [0, 0x00000085, 0x00030000, 0x000008ff], // last thing is probably a float?
                                                },
                                                ..Default::default()
                                            };

                                            connection
                                                .send_segment(PacketSegment {
                                                    source_actor: connection.player_id,
                                                    target_actor: connection.player_id,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                })
                                                .await;
                                        }

                                        tracing::info!("sending them to {:#?}", new_territory);

                                        connection.change_zone(new_territory).await;
                                    }
                                    IPCStructData::Unk13 { .. } => {
                                        tracing::info!("Recieved Unk13!");
                                    }
                                    IPCStructData::Unk14 { .. } => {
                                        tracing::info!("Recieved Unk14!");
                                    }
                                    _ => panic!(
                                        "The server is recieving a IPC response or unknown packet!"
                                    ),
                                }
                            }
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive(
                                    &mut connection.socket,
                                    &mut connection.state,
                                    *id,
                                    *timestamp,
                                )
                                .await
                            }
                            SegmentType::KeepAliveResponse { .. } => {
                                tracing::info!("Got keep alive response from client... cool...");
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
