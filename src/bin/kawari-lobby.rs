use kawari::CONTENT_ID;
use kawari::lobby::chara_make::CharaMake;
use kawari::lobby::connection::LobbyConnection;
use kawari::lobby::ipc::{
    CharacterDetails, ClientLobbyIpcData, LobbyCharacterAction, ServerLobbyIpcData,
    ServerLobbyIpcSegment, ServerLobbyIpcType,
};
use kawari::oodle::FFXIVOodle;
use kawari::packet::{PacketSegment, PacketState, SegmentType, send_keep_alive};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();

    tracing::info!("Lobby server started on 127.0.0.1:7000");

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let state = PacketState {
            client_key: None,
            session_id: None,
            clientbound_oodle: FFXIVOodle::new(),
            serverbound_oodle: FFXIVOodle::new(),
        };

        let mut connection = LobbyConnection { socket, state };

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = connection
                    .socket
                    .read(&mut buf)
                    .await
                    .expect("Failed to read data!");

                if n != 0 {
                    tracing::info!("read {} bytes", n);

                    let (segments, _) = connection.parse_packet(&buf[..n]).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::InitializeEncryption { phrase, key } => {
                                connection.initialize_encryption(phrase, key).await;
                            }
                            SegmentType::Ipc { data } => match &data.data {
                                ClientLobbyIpcData::ClientVersionInfo {
                                    session_id,
                                    version_info,
                                    ..
                                } => {
                                    tracing::info!(
                                        "Client {session_id} ({version_info}) logging in!"
                                    );

                                    connection.state.session_id = Some(session_id.clone());

                                    connection.send_account_list().await;

                                    // request an update
                                    //connection.send_error(*sequence, 1012, 13101).await;
                                }
                                ClientLobbyIpcData::RequestCharacterList { sequence } => {
                                    tracing::info!("Client is requesting character list...");

                                    connection.send_lobby_info(*sequence).await;
                                }
                                ClientLobbyIpcData::LobbyCharacterAction {
                                    action,
                                    name,
                                    json,
                                    ..
                                } => {
                                    match action {
                                        LobbyCharacterAction::ReserveName => {
                                            tracing::info!(
                                                "Player is requesting {name} as a new character name!"
                                            );

                                            // reject
                                            /*{
                                                let ipc = IPCSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: IPCOpCode::InitializeChat, // wrong but technically right
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: ClientLobbyIpcType::NameRejection {
                                                        unk1: 0x03,
                                                        unk2: 0x0bdb,
                                                        unk3: 0x000132cc,
                                                    },
                                                };

                                                let response_packet = PacketSegment {
                                                    source_actor: 0x0,
                                                    target_actor: 0x0,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                };
                                                send_packet(
                                                    &mut write,
                                                    &[response_packet],
                                                    &mut state,
                                                    CompressionType::Uncompressed,
                                                )
                                                .await;
                                            }*/

                                            // accept
                                            {
                                                let ipc = ServerLobbyIpcSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: ServerLobbyIpcType::CharacterCreated,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: ServerLobbyIpcData::CharacterCreated {
                                                        unk1: 0x4,
                                                        unk2: 0x00010101,
                                                        details: CharacterDetails {
                                                            content_id: CONTENT_ID,
                                                            character_name: name.clone(),
                                                            origin_server_name: "KAWARI"
                                                                .to_string(),
                                                            current_server_name: "KAWARI"
                                                                .to_string(),
                                                            ..Default::default()
                                                        },
                                                    },
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: 0x0,
                                                        target_actor: 0x0,
                                                        segment_type: SegmentType::Ipc {
                                                            data: ipc,
                                                        },
                                                    })
                                                    .await;
                                            }
                                        }
                                        LobbyCharacterAction::Create => {
                                            tracing::info!("Player is creating a new character!");

                                            let chara_make = CharaMake::from_json(json);
                                            println!("charamake: {:#?}", chara_make);

                                            // a slightly different character created packet now
                                            {
                                                let ipc = ServerLobbyIpcSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: ServerLobbyIpcType::CharacterCreated,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: ServerLobbyIpcData::CharacterCreated {
                                                        unk1: 0x5,
                                                        unk2: 0x00020101,
                                                        details: CharacterDetails {
                                                            id: 0x07369f3a, // notice that we give them an id now
                                                            content_id: CONTENT_ID,
                                                            character_name: name.clone(),
                                                            origin_server_name: "KAWARI"
                                                                .to_string(),
                                                            current_server_name: "KAWARI"
                                                                .to_string(),
                                                            ..Default::default()
                                                        },
                                                    },
                                                };

                                                connection
                                                    .send_segment(PacketSegment {
                                                        source_actor: 0x0,
                                                        target_actor: 0x0,
                                                        segment_type: SegmentType::Ipc {
                                                            data: ipc,
                                                        },
                                                    })
                                                    .await;
                                            }
                                        }
                                        LobbyCharacterAction::Rename => todo!(),
                                        LobbyCharacterAction::Delete => todo!(),
                                        LobbyCharacterAction::Move => todo!(),
                                        LobbyCharacterAction::RemakeRetainer => todo!(),
                                        LobbyCharacterAction::RemakeChara => todo!(),
                                        LobbyCharacterAction::SettingsUploadBegin => todo!(),
                                        LobbyCharacterAction::SettingsUpload => todo!(),
                                        LobbyCharacterAction::WorldVisit => todo!(),
                                        LobbyCharacterAction::DataCenterToken => todo!(),
                                        LobbyCharacterAction::Request => todo!(),
                                    }
                                }
                                ClientLobbyIpcData::RequestEnterWorld {
                                    sequence,
                                    lookup_id,
                                } => {
                                    tracing::info!("Client is joining the world...");

                                    connection.send_enter_world(*sequence, *lookup_id).await;
                                }
                            },
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive::<ServerLobbyIpcSegment>(
                                    &mut connection.socket,
                                    &mut connection.state,
                                    *id,
                                    *timestamp,
                                )
                                .await
                            }
                            _ => {
                                panic!("The server is recieving a response packet!")
                            }
                        }
                    }
                }
            }
        });
    }
}
