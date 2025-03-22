use kawari::common::custom_ipc::CustomIpcData;
use kawari::common::custom_ipc::CustomIpcSegment;
use kawari::common::custom_ipc::CustomIpcType;
use kawari::lobby::LobbyConnection;
use kawari::lobby::ipc::{
    CharacterDetails, ClientLobbyIpcData, LobbyCharacterActionKind, ServerLobbyIpcData,
    ServerLobbyIpcSegment, ServerLobbyIpcType,
};
use kawari::lobby::send_custom_world_packet;
use kawari::oodle::OodleNetwork;
use kawari::packet::ConnectionType;
use kawari::packet::{PacketSegment, PacketState, SegmentType, send_keep_alive};
use kawari::{CONTENT_ID, WORLD_NAME};
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
            clientbound_oodle: OodleNetwork::new(),
            serverbound_oodle: OodleNetwork::new(),
        };

        let mut connection = LobbyConnection {
            socket,
            state,
            session_id: None,
            stored_character_creation_name: String::new(),
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

                                    connection.session_id = Some(session_id.clone());

                                    connection.send_account_list().await;

                                    // request an update
                                    //connection.send_error(*sequence, 1012, 13101).await;
                                }
                                ClientLobbyIpcData::RequestCharacterList { sequence } => {
                                    tracing::info!("Client is requesting character list...");

                                    connection.send_lobby_info(*sequence).await;
                                }
                                ClientLobbyIpcData::LobbyCharacterAction(character_action) => {
                                    match &character_action.action {
                                        LobbyCharacterActionKind::ReserveName => {
                                            tracing::info!(
                                                "Player is requesting {} as a new character name!",
                                                character_action.name
                                            );

                                            // check with the world server if the name is available
                                            let name_request = CustomIpcSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: CustomIpcType::CheckNameIsAvailable,
                                                server_id: 0,
                                                timestamp: 0,
                                                data: CustomIpcData::CheckNameIsAvailable {
                                                    name: character_action.name.clone(),
                                                },
                                            };

                                            let name_response =
                                                send_custom_world_packet(name_request)
                                                    .await
                                                    .expect("Failed to get name request packet!");
                                            let CustomIpcData::NameIsAvailableResponse { free } =
                                                &name_response.data
                                            else {
                                                panic!("Unexpedted custom IPC type!")
                                            };

                                            tracing::info!("Is name free? {free}");

                                            // TODO: use read_bool_as
                                            let free: bool = *free == 1u8;

                                            if free {
                                                connection.stored_character_creation_name =
                                                    character_action.name.clone();

                                                let ipc = ServerLobbyIpcSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: ServerLobbyIpcType::CharacterCreated,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: ServerLobbyIpcData::CharacterCreated {
                                                        sequence: character_action.sequence + 1,
                                                        unk: 0x00010101,
                                                        details: CharacterDetails {
                                                            content_id: CONTENT_ID,
                                                            character_name: character_action
                                                                .name
                                                                .clone(),
                                                            origin_server_name: WORLD_NAME
                                                                .to_string(),
                                                            current_server_name: WORLD_NAME
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
                                            } else {
                                                let ipc = ServerLobbyIpcSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: ServerLobbyIpcType::LobbyError,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: ServerLobbyIpcData::LobbyError {
                                                        sequence: 0x03,
                                                        error: 0x0bdb, // TODO: I screwed this up when translating from the old struct to the new LobbyError
                                                        exd_error_id: 0,
                                                        value: 0,
                                                        unk1: 0,
                                                    },
                                                };

                                                let response_packet = PacketSegment {
                                                    source_actor: 0x0,
                                                    target_actor: 0x0,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                };
                                                connection.send_segment(response_packet).await;
                                            }
                                        }
                                        LobbyCharacterActionKind::Create => {
                                            tracing::info!("Player is creating a new character!");

                                            let our_actor_id;
                                            let our_content_id;

                                            // tell the world server to create this character
                                            {
                                                let ipc_segment = CustomIpcSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: CustomIpcType::RequestCreateCharacter,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: CustomIpcData::RequestCreateCharacter {
                                                        name: connection
                                                            .stored_character_creation_name
                                                            .clone(), // TODO: worth double-checking, but AFAIK we have to store it this way?
                                                        chara_make_json: character_action
                                                            .json
                                                            .clone(),
                                                    },
                                                };

                                                let response_segment =
                                                    send_custom_world_packet(ipc_segment)
                                                        .await
                                                        .unwrap();
                                                match &response_segment.data {
                                                    CustomIpcData::CharacterCreated {
                                                        actor_id,
                                                        content_id,
                                                    } => {
                                                        our_actor_id = *actor_id;
                                                        our_content_id = *content_id;
                                                    }
                                                    _ => panic!(
                                                        "Unexpected custom IPC packet type here!"
                                                    ),
                                                }
                                            }

                                            tracing::info!(
                                                "Got new player info from world server: {our_content_id} {our_actor_id}"
                                            );

                                            // a slightly different character created packet now
                                            {
                                                let ipc = ServerLobbyIpcSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: ServerLobbyIpcType::CharacterCreated,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: ServerLobbyIpcData::CharacterCreated {
                                                        sequence: character_action.sequence + 1,
                                                        unk: 0x00020101,
                                                        details: CharacterDetails {
                                                            actor_id: our_actor_id,
                                                            content_id: our_content_id,
                                                            character_name: character_action
                                                                .name
                                                                .clone(),
                                                            origin_server_name: WORLD_NAME
                                                                .to_string(),
                                                            current_server_name: WORLD_NAME
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
                                        LobbyCharacterActionKind::Rename => todo!(),
                                        LobbyCharacterActionKind::Delete => todo!(),
                                        LobbyCharacterActionKind::Move => todo!(),
                                        LobbyCharacterActionKind::RemakeRetainer => todo!(),
                                        LobbyCharacterActionKind::RemakeChara => todo!(),
                                        LobbyCharacterActionKind::SettingsUploadBegin => todo!(),
                                        LobbyCharacterActionKind::SettingsUpload => todo!(),
                                        LobbyCharacterActionKind::WorldVisit => todo!(),
                                        LobbyCharacterActionKind::DataCenterToken => todo!(),
                                        LobbyCharacterActionKind::Request => todo!(),
                                    }
                                }
                                ClientLobbyIpcData::RequestEnterWorld {
                                    sequence,
                                    content_id,
                                } => {
                                    tracing::info!("Client is joining the world with {content_id}");

                                    let our_actor_id;

                                    // find the actor id for this content id
                                    // NOTE: This is NOT the ideal solution. I theorize the lobby server has it's own records with this information.
                                    {
                                        let ipc_segment = CustomIpcSegment {
                                            unk1: 0,
                                            unk2: 0,
                                            op_code: CustomIpcType::GetActorId,
                                            server_id: 0,
                                            timestamp: 0,
                                            data: CustomIpcData::GetActorId {
                                                content_id: *content_id,
                                            },
                                        };

                                        let response_segment =
                                            send_custom_world_packet(ipc_segment).await.unwrap();

                                        match &response_segment.data {
                                            CustomIpcData::ActorIdFound { actor_id } => {
                                                our_actor_id = *actor_id;
                                            }
                                            _ => panic!("Unexpected custom IPC packet type here!"),
                                        }
                                    }

                                    connection
                                        .send_enter_world(*sequence, *content_id, our_actor_id)
                                        .await;
                                }
                            },
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive::<ServerLobbyIpcSegment>(
                                    &mut connection.socket,
                                    &mut connection.state,
                                    ConnectionType::Lobby,
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
