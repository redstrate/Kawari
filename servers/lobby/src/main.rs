use kawari::common::NETWORK_TIMEOUT;
use kawari::common::RECEIVE_BUFFER_SIZE;
use kawari::common::timestamp_secs;
use kawari::config::get_config;
use kawari::ipc::kawari::CustomIpcData;
use kawari::ipc::kawari::CustomIpcSegment;
use kawari::ipc::lobby::{ClientLobbyIpcData, ServerLobbyIpcSegment};
use kawari::packet::ConnectionType;
use kawari::packet::PacketSegment;
use kawari::packet::SegmentType;
use kawari::packet::send_custom_world_packet;
use kawari::packet::{ConnectionState, SegmentData, send_keep_alive};
use kawari_lobby::LobbyConnection;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = get_config();

    let addr = config.lobby.get_socketaddr();

    let listener = TcpListener::bind(addr).await.unwrap();

    tracing::info!("Server started on {addr}");

    loop {
        let (socket, _) = listener.accept().await.unwrap();

        let mut connection = LobbyConnection {
            socket,
            state: ConnectionState::None,
            session_id: None,
            stored_character_creation_name: String::new(),
            service_accounts: Vec::new(),
            selected_service_account: None,
            last_keep_alive: Instant::now(),
        };

        // as seen in retail, the server sends a KeepAliveRequest before doing *anything*
        {
            connection
                .send_segment(PacketSegment {
                    segment_type: SegmentType::KeepAliveRequest,
                    data: SegmentData::KeepAliveRequest {
                        id: 0xE0037603u32,
                        timestamp: timestamp_secs(),
                    },
                    ..Default::default()
                })
                .await;
        }

        tokio::spawn(async move {
            let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
            loop {
                match connection.socket.read(&mut buf).await {
                    Ok(n) => {
                        // if the last response was over >5 seconds, the client is probably gone
                        if n == 0 {
                            let now = Instant::now();
                            if now.duration_since(connection.last_keep_alive) > NETWORK_TIMEOUT {
                                tracing::info!("Connection was killed because of timeout");
                                break;
                            }
                        } else {
                            connection.last_keep_alive = Instant::now();

                            let segments = connection.parse_packet(&buf[..n]);
                            for segment in &segments {
                                match &segment.data {
                                    SegmentData::SecuritySetup { phrase, key } => {
                                        connection.initialize_encryption(phrase, key).await
                                    }
                                    SegmentData::Ipc(data) => match &data.data {
                                        ClientLobbyIpcData::LoginEx {
                                            sequence,
                                            session_id,
                                            version_info,
                                            ..
                                        } => {
                                            connection
                                                .login(*sequence, session_id, version_info)
                                                .await;
                                        }
                                        ClientLobbyIpcData::ServiceLogin {
                                            sequence,
                                            account_index,
                                            ..
                                        } => {
                                            connection.selected_service_account = Some(
                                                connection.service_accounts
                                                    [*account_index as usize]
                                                    .id,
                                            );
                                            connection.send_lobby_info(*sequence).await
                                        }
                                        ClientLobbyIpcData::CharaMake(character_action) => {
                                            connection
                                                .handle_character_action(character_action)
                                                .await
                                        }
                                        ClientLobbyIpcData::ShandaLogin {
                                            sequence,
                                            session_id,
                                            version_info,
                                            ..
                                        } => {
                                            connection
                                                .login(*sequence, session_id, version_info)
                                                .await;
                                        }
                                        ClientLobbyIpcData::GameLogin {
                                            sequence,
                                            content_id,
                                            ..
                                        } => {
                                            tracing::info!(
                                                "Client is joining the world with {content_id}"
                                            );

                                            let our_actor_id;

                                            // find the actor id for this content id
                                            // NOTE: This is NOT the ideal solution. I theorize the lobby server has it's own records with this information.
                                            {
                                                let ipc_segment = CustomIpcSegment::new(
                                                    CustomIpcData::GetActorId {
                                                        content_id: *content_id,
                                                    },
                                                );

                                                if let Some(response_segment) =
                                                    send_custom_world_packet(ipc_segment).await
                                                {
                                                    match &response_segment.data {
                                                        CustomIpcData::ActorIdFound {
                                                            actor_id,
                                                        } => {
                                                            our_actor_id = *actor_id;
                                                        }
                                                        _ => panic!(
                                                            "Unexpected custom IPC packet type here!"
                                                        ),
                                                    }

                                                    connection
                                                        .send_enter_world(
                                                            *sequence,
                                                            *content_id,
                                                            our_actor_id,
                                                        )
                                                        .await;
                                                } else {
                                                    // "The lobby server has encountered a problem."
                                                    connection
                                                        .send_error(*sequence, 2002, 13006)
                                                        .await;
                                                }
                                            }
                                        }
                                        ClientLobbyIpcData::Unknown { unk } => {
                                            tracing::warn!(
                                                "Unknown packet {:?} recieved ({} bytes), this should be handled!",
                                                data.header.op_code,
                                                unk.len()
                                            );
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
                                    SegmentData::KeepAliveResponse { .. } => {
                                        // Intentionally empty, we can throw this away.
                                    }
                                    _ => {
                                        panic!("The server is recieving a response packet!")
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {
                        tracing::info!("A connection was disconnected!");
                        break;
                    }
                }
            }
        });
    }
}
