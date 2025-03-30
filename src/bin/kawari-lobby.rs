use kawari::common::custom_ipc::CustomIpcData;
use kawari::common::custom_ipc::CustomIpcSegment;
use kawari::common::custom_ipc::CustomIpcType;
use kawari::common::get_world_name;
use kawari::config::get_config;
use kawari::lobby::LobbyConnection;
use kawari::lobby::ipc::{ClientLobbyIpcData, ServerLobbyIpcSegment};
use kawari::lobby::send_custom_world_packet;
use kawari::oodle::OodleNetwork;
use kawari::packet::ConnectionType;
use kawari::packet::{PacketState, SegmentType, send_keep_alive};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.lobby.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    tracing::info!("Server started on {addr}");

    let world_name = get_world_name(config.world.world_id);

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
            world_name: world_name.clone(),
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
                                connection.initialize_encryption(phrase, key).await
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
                                    connection.send_lobby_info(*sequence).await
                                }
                                ClientLobbyIpcData::LobbyCharacterAction(character_action) => {
                                    connection.handle_character_action(character_action).await
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
