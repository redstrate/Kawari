use kawari::RECEIVE_BUFFER_SIZE;
use kawari::common::GameData;
use kawari::config::get_config;
use kawari::get_supported_expac_versions;
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
use std::fs;
use std::path::MAIN_SEPARATOR_STR;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

/// Allows the lobby server to do a thorough client version check.
/// First, it checks the local game executable's file length against the client-specified size.
/// Second, it calculates a SHA1 hash against the locally stored game executable and compares it to the client-specified hash.
/// Finally, it compares expansion pack version strings provided by the client against locally stored information.
/// If, and only if, all of these checks pass, does the client get allowed in.
fn do_game_version_check(client_version_str: &str) -> bool {
    let config = get_config();
    const VERSION_STR_LEN: usize = 145;

    if client_version_str.len() != VERSION_STR_LEN {
        tracing::error!(
            "Version string sent by client is invalid or malformed, its length is {}! Rejecting session!",
            client_version_str.len()
        );
        return false;
    }

    let game_exe_path = [
        config.game_location,
        MAIN_SEPARATOR_STR.to_string(),
        "ffxiv_dx11.exe".to_string(),
    ]
    .join("");
    if let Ok(game_md) = fs::metadata(&game_exe_path) {
        let expected_exe_len = game_md.len();

        let parts: Vec<&str> = client_version_str.split("+").collect();
        if parts[0].starts_with("ffxiv_dx11.exe") {
            let exe_parts: Vec<&str> = parts[0].split("/").collect();
            match exe_parts[1].parse::<u64>() {
                Ok(client_exe_len) => {
                    if client_exe_len != expected_exe_len {
                        tracing::error!(
                            "Client's game executable length is incorrect! Rejecting session! Got {}, expected {}",
                            client_exe_len,
                            expected_exe_len
                        );
                        return false;
                    } else {
                        tracing::info!("Client's game executable length is OK.");
                    }
                }
                Err(err) => {
                    tracing::error!(
                        "Client's version string is malformed, unable to parse executable length field! Rejecting session! Got {}, further info: {}",
                        exe_parts[1],
                        err
                    );
                    return false;
                }
            }

            let client_exe_hash = exe_parts[2];

            match std::fs::read(&game_exe_path) {
                Ok(game_exe_filebuffer) => {
                    let expected_exe_hash = sha1_smol::Sha1::from(game_exe_filebuffer)
                        .digest()
                        .to_string();
                    if client_exe_hash != expected_exe_hash {
                        tracing::error!(
                            "Client's game executable is corrupted! Rejecting session! Got {}, expected {}",
                            client_exe_hash,
                            expected_exe_hash
                        );
                        return false;
                    } else {
                        tracing::info!("Client's game executable hash is OK.");
                    }
                }
                Err(err) => {
                    panic!(
                        "Unable to read our game executable file! Stopping lobby server! Further information: {err}",
                    );
                }
            }

            let client_expansion_versions = &parts[1..];

            let supported_expansion_versions = get_supported_expac_versions();
            if client_expansion_versions.len() != supported_expansion_versions.len() {
                tracing::error!(
                    "Client sent a malformed version string! It is missing one or more expansion versions! Rejecting session!"
                );
                return false;
            }

            // We need these in order, and hashmaps don't guarantee this.
            let expected_versions = [
                &supported_expansion_versions["ex1"].0,
                &supported_expansion_versions["ex2"].0,
                &supported_expansion_versions["ex3"].0,
                &supported_expansion_versions["ex4"].0,
                &supported_expansion_versions["ex5"].0,
            ];

            for expansion in client_expansion_versions
                .iter()
                .zip(expected_versions.iter())
            {
                // The client doesn't send a patch2 value in its expansion version strings, so we just pretend it doesn't exist on our side.
                let expected_version = &expansion.1[..expansion.1.len() - 5].to_string();
                let client_version = *expansion.0;
                if client_version != expected_version {
                    tracing::error!(
                        "One of the client's expansion versions does not match! Rejecting session! Got {}, expected {}",
                        client_version,
                        expected_version
                    );
                    return false;
                }
            }
            tracing::info!("All client version checks succeeded! Allowing session!");
            return true;
        }
        tracing::error!(
            "Client sent a malformed version string! It doesn't declare the name of the game executable correctly! Rejecting session!"
        );
        return false;
    }
    panic!("Our game executable doesn't exist! We can't do version checks!");
}

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
                    let (segments, _) = connection.parse_packet(&buf[..n]);
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

                                    // The lobby server does its own version check as well, but it can be turned off if desired.
                                    if config.lobby.do_version_checks
                                        && !do_game_version_check(version_info)
                                    {
                                        // "A version update is required."
                                        connection.send_error(*sequence, 1012, 13101).await;
                                        break;
                                    }

                                    let Ok(login_reply) = reqwest::get(format!(
                                        "http://{}/_private/service_accounts?sid={}",
                                        config.login.server_name, session_id
                                    ))
                                    .await
                                    else {
                                        tracing::warn!(
                                            "Failed to contact login server, is it running?"
                                        );
                                        // "The lobby server connection has encountered an error."
                                        connection.send_error(*sequence, 2002, 13001).await;
                                        break;
                                    };

                                    let Ok(body) = login_reply.text().await else {
                                        tracing::warn!(
                                            "Failed to contact login server, is it running?"
                                        );
                                        // "The lobby server connection has encountered an error."
                                        connection.send_error(*sequence, 2002, 13001).await;
                                        break;
                                    };

                                    let service_accounts: Option<Vec<ServiceAccount>> =
                                        serde_json::from_str(&body).ok();
                                    if let Some(service_accounts) = service_accounts {
                                        if service_accounts.is_empty() {
                                            tracing::warn!(
                                                "This account has no service accounts attached, how did this happen?"
                                            );

                                            /* "<the game> has not yet been registered on this platform or your service account's subscription has expired. Please close the application and complete the registration process. If you would like to add a platform to your service account or renew your subscription, please visit the <website>). To register another platform, you must purchase a license for the applicable platform or complete the registration process using the registration code included with your purchase." */
                                            connection.send_error(*sequence, 2002, 13209).await;
                                        } else {
                                            connection.service_accounts = service_accounts;
                                            connection.session_id = Some(session_id.clone());
                                            connection.send_account_list().await;
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Failed to parse service accounts from the login server!"
                                        );

                                        // "The lobby server has encountered a problem."
                                        connection.send_error(*sequence, 2002, 13006).await;
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
                                ClientLobbyIpcData::ShandaLogin { .. } => {
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
                                _ => {}
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
