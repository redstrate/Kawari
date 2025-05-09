use kawari::RECEIVE_BUFFER_SIZE;
use kawari::common::GameData;
use kawari::config::get_config;
use kawari::ipc::kawari::CustomIpcData;
use kawari::ipc::kawari::CustomIpcSegment;
use kawari::ipc::kawari::CustomIpcType;
use kawari::ipc::lobby::ServiceAccount;
use kawari::ipc::lobby::{ClientLobbyIpcData, ServerLobbyIpcSegment};
use kawari::lobby::LobbyConnection;
use kawari::lobby::send_custom_world_packet;
use kawari::packet::ConnectionType;
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::{PacketState, SegmentData, send_keep_alive};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.lobby.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    tracing::info!("Server started on {addr}");

    let mut game_data = GameData::new();
    let world_name = game_data
        .get_world_name(config.world.world_id)
        .expect("Unknown world name");

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
            service_accounts: Vec::new(),
            selected_service_account: None,
        };

        tokio::spawn(async move {
            let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
            loop {
                let n = connection
                    .socket
                    .read(&mut buf)
                    .await
                    .expect("Failed to read data!");

                if n != 0 {
                    let (segments, _) = connection.parse_packet(&buf[..n]).await;
                    for segment in &segments {
                        match &segment.data {
                            SegmentData::SecuritySetup { phrase, key } => {
                                connection.initialize_encryption(phrase, key).await
                            }
                            SegmentData::Ipc { data } => match &data.data {
                                ClientLobbyIpcData::LoginEx {
                                    sequence,
                                    session_id,
                                    version_info,
                                } => {
                                    tracing::info!(
                                        "Client {session_id} ({version_info}) logging in!"
                                    );

                                    let config = get_config();

                                    let body = reqwest::get(format!(
                                        "http://{}/_private/service_accounts?sid={}",
                                        config.login.get_socketaddr(),
                                        session_id
                                    ))
                                    .await
                                    .unwrap()
                                    .text()
                                    .await
                                    .unwrap();

                                    let service_accounts: Option<Vec<ServiceAccount>> =
                                        serde_json::from_str(&body).ok();
                                    if let Some(service_accounts) = service_accounts {
                                        if service_accounts.is_empty() {
                                            // request an update, wrong error message lol
                                            connection.send_error(*sequence, 1012, 13101).await;
                                        } else {
                                            connection.service_accounts = service_accounts;
                                            connection.session_id = Some(session_id.clone());
                                            connection.send_account_list().await;
                                        }
                                    } else {
                                        // request an update, wrong error message lol
                                        connection.send_error(*sequence, 1012, 13101).await;
                                    }
                                }
                                ClientLobbyIpcData::ServiceLogin { sequence } => {
                                    // TODO: support selecting a service account
                                    connection.selected_service_account =
                                        Some(connection.service_accounts[0].id);
                                    connection.send_lobby_info(*sequence).await
                                }
                                ClientLobbyIpcData::CharaMake(character_action) => {
                                    connection.handle_character_action(character_action).await
                                }
                                ClientLobbyIpcData::ShandaLogin { unk } => {
                                    dbg!(unk);

                                    connection.send_account_list().await;
                                }
                                ClientLobbyIpcData::GameLogin {
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
                                            option: 0,
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
                            SegmentData::KeepAliveRequest { id, timestamp } => {
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
