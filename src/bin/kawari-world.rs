use std::time::{SystemTime, UNIX_EPOCH};

use kawari::ipc::{IPCOpCode, IPCSegment, IPCStructData};
use kawari::oodle::FFXIVOodle;
use kawari::packet::{
    CompressionType, PacketSegment, SegmentType, State, parse_packet, send_keep_alive, send_packet,
};
use kawari::world::{
    ActorControlSelf, ActorControlType, InitZone, PlayerSetup, PlayerSpawn, PlayerStats,
    UpdateClassInfo,
};
use kawari::{CONTENT_ID, WORLD_ID, ZONE_ID};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7100").await.unwrap();

    tracing::info!("World server started on 7100");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let (mut read, mut write) = tokio::io::split(socket);

        let mut state = State {
            client_key: None,
            session_id: None,
            clientbound_oodle: FFXIVOodle::new(),
            serverbound_oodle: FFXIVOodle::new(),
            player_id: None,
        };

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = read.read(&mut buf).await.expect("Failed to read data!");

                if n != 0 {
                    println!("recieved {n} bytes...");
                    let (segments, connection_type) = parse_packet(&buf[..n], &mut state).await;
                    for segment in &segments {
                        let timestamp_secs = || {
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .expect("Failed to get UNIX timestamp!")
                                .as_secs()
                                .try_into()
                                .unwrap()
                        };

                        match &segment.segment_type {
                            SegmentType::InitializeSession { player_id } => {
                                state.player_id = Some(*player_id);

                                // We have send THEM a keep alive
                                {
                                    let timestamp: u32 = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .expect("Failed to get UNIX timestamp!")
                                        .as_secs()
                                        .try_into()
                                        .unwrap();

                                    let response_packet = PacketSegment {
                                        source_actor: 0,
                                        target_actor: 0,
                                        segment_type: SegmentType::KeepAlive {
                                            id: 0xE0037603u32,
                                            timestamp,
                                        },
                                    };
                                    send_packet(
                                        &mut write,
                                        &[response_packet],
                                        &mut state,
                                        CompressionType::Oodle,
                                    )
                                    .await;
                                }

                                match connection_type {
                                    kawari::packet::ConnectionType::Zone => {
                                        tracing::info!(
                                            "Client {player_id} is initializing zone session..."
                                        );

                                        let response_packet = PacketSegment {
                                            source_actor: 0,
                                            target_actor: 0,
                                            segment_type: SegmentType::ZoneInitialize {
                                                player_id: *player_id,
                                            },
                                        };
                                        send_packet(
                                            &mut write,
                                            &[response_packet],
                                            &mut state,
                                            CompressionType::Oodle,
                                        )
                                        .await;
                                    }
                                    kawari::packet::ConnectionType::Chat => {
                                        tracing::info!(
                                            "Client {player_id} is initializing chat session..."
                                        );

                                        {
                                            let response_packet = PacketSegment {
                                                source_actor: 0,
                                                target_actor: 0,
                                                segment_type: SegmentType::ZoneInitialize {
                                                    player_id: *player_id,
                                                },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitializeChat,
                                                server_id: 0,
                                                timestamp: 0,
                                                data: IPCStructData::InitializeChat { unk: [0; 8] },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: *player_id,
                                                target_actor: *player_id,
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
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
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitResponse,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitResponse {
                                                    unk1: 0,
                                                    character_id: state.player_id.unwrap(),
                                                    unk2: 0,
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Control Data
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::ActorControlSelf,
                                                server_id: 0,
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
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Stats
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::PlayerStats,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerStats(PlayerStats {
                                                    strength: 1,
                                                    hp: 100,
                                                    mp: 100,
                                                    ..Default::default()
                                                }),
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Player Setup
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::PlayerSetup,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerSetup(PlayerSetup {
                                                    content_id: CONTENT_ID,
                                                    exp: [10000; 32],
                                                    levels: [100; 32],
                                                    name: "KAWARI".to_string(),
                                                    ..Default::default()
                                                }),
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Player Class Info
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::UpdateClassInfo,
                                                server_id: 69, // lol
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::UpdateClassInfo(
                                                    UpdateClassInfo {
                                                        class_id: 35,
                                                        unknown: 1,
                                                        synced_level: 90,
                                                        class_level: 90,
                                                        ..Default::default()
                                                    },
                                                ),
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Init Zone
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitZone,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitZone(InitZone {
                                                    server_id: WORLD_ID,
                                                    zone_id: ZONE_ID,
                                                    ..Default::default()
                                                }),
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
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
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::PlayerSpawn,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerSpawn(PlayerSpawn {
                                                    hp_curr: 100,
                                                    hp_max: 100,
                                                    mp_curr: 100,
                                                    mp_max: 100,
                                                    ..Default::default()
                                                }),
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }
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
                                    IPCStructData::Unk6 { .. } => {
                                        tracing::info!("Recieved Unk6!");
                                    }
                                    IPCStructData::Unk7 { .. } => {
                                        tracing::info!("Recieved Unk7!");
                                    }
                                    IPCStructData::UpdatePositionHandler { .. } => {
                                        tracing::info!("Recieved UpdatePositionHandler!");
                                    }
                                    IPCStructData::LogOut { .. } => {
                                        tracing::info!("Recieved log out from client!");

                                        // tell the client to disconnect
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::LogOutComplete,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::LogOutComplete { unk: [0; 8] },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }
                                    }
                                    IPCStructData::Disconnected { .. } => {
                                        tracing::info!("Client disconnected!");
                                    }
                                    _ => panic!(
                                        "The server is recieving a IPC response or unknown packet!"
                                    ),
                                }
                            }
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive(&mut write, &mut state, *id, *timestamp).await
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
