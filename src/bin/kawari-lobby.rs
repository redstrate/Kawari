use kawari::RECEIVE_BUFFER_SIZE;
use kawari::common::GameData;
use kawari::common::timestamp_secs;
use kawari::config::get_config;
use kawari::get_supported_expac_versions;
use kawari::ipc::kawari::CustomIpcData;
use kawari::ipc::kawari::CustomIpcSegment;
use kawari::ipc::kawari::CustomIpcType;
use kawari::ipc::lobby::ServiceAccount;
use kawari::ipc::lobby::{ClientLobbyIpcData, ServerLobbyIpcSegment};
use kawari::lobby::LobbyConnection;
use kawari::packet::ConnectionType;
use kawari::packet::PacketSegment;
use kawari::packet::SegmentType;
use kawari::packet::oodle::OodleNetwork;
use kawari::packet::send_custom_world_packet;
use kawari::packet::{PacketState, SegmentData, send_keep_alive};
use std::fs;
use std::path::MAIN_SEPARATOR_STR;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

const GAME_EXE_NAME: &str = "ffxiv_dx11.exe";

#[derive(Debug)]
struct VersionCheckData {
    game_exe_len: usize,
    game_exe_sha1_hash: String,
    expansion_pack_versions: Vec<String>,
}

/// Allows the lobby server to do a thorough client version check.
/// First, it checks the local game executable's file length against the client-specified size.
/// Second, it calculates a SHA1 hash against the locally stored game executable and compares it to the client-specified hash.
/// Finally, it compares expansion pack version strings provided by the client against locally stored information.
/// If, and only if, all of these checks pass, is the client permitted to continue.
fn do_game_version_check(client_version_str: &str) -> bool {
    let config = get_config();

    let game_exe_path = &format!(
        "{}{}{}",
        config.filesystem.game_path, MAIN_SEPARATOR_STR, GAME_EXE_NAME
    );

    if let Ok(game_md) = fs::metadata(game_exe_path) {
        let Some(client_version_data) = validate_client_version_string(client_version_str) else {
            return false;
        };

        let expected_exe_len = game_md.len() as usize;

        if client_version_data.game_exe_len != expected_exe_len {
            tracing::error!(
                "Client's game executable length is incorrect! Rejecting session! Got {}, expected {}",
                client_version_data.game_exe_len,
                expected_exe_len
            );
            return false;
        } else {
            tracing::info!("Client's game executable length is OK.")
        }

        match std::fs::read(game_exe_path) {
            Ok(game_exe_filebuffer) => {
                let expected_exe_hash = sha1_smol::Sha1::from(game_exe_filebuffer)
                    .digest()
                    .to_string();
                if client_version_data.game_exe_sha1_hash != expected_exe_hash {
                    tracing::error!(
                        "Client's game executable is corrupted! Rejecting session! Got {}, expected {}",
                        client_version_data.game_exe_sha1_hash,
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

        let supported_expansion_versions = get_supported_expac_versions();

        // We need these in order, and hashmaps don't guarantee this.
        let expected_versions = [
            &supported_expansion_versions["ex1"].0,
            &supported_expansion_versions["ex2"].0,
            &supported_expansion_versions["ex3"].0,
            &supported_expansion_versions["ex4"].0,
            &supported_expansion_versions["ex5"].0,
        ];

        for expansion in client_version_data
            .expansion_pack_versions
            .iter()
            .zip(expected_versions.iter())
        {
            // The client doesn't send a patch2 value in its expansion version strings, so we just pretend it doesn't exist on our side.
            let expected_version = &expansion.1[..expansion.1.len() - 5].to_string();
            let client_version = expansion.0;
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
    panic!("Our game executable doesn't exist! We can't do version checks! Stopping lobby server!");
}

/// Validates most of the information sent by the client before doing the actual versioning and sizing checks.
fn validate_client_version_string(client_version_str: &str) -> Option<VersionCheckData> {
    // We assume the client has at least ex1 & ex2.
    const VER_PARTS_MIN_LEN: usize = 3;

    // The exe section is divided into name, file size, and sha1 hash.
    const EXE_VERIFICATION_INFO_PARTS: usize = 3;

    // For now, client expansion substrings are 15 characters with a format of: YYYY.MM.DD.AAAA, YYYY = year, MM = month, DD = day, AAAA = patch1 value.
    const EXPANSION_VERSION_STR_LEN: usize = 15;

    let parts: Vec<&str> = client_version_str.split("+").collect();

    // If the client is claiming they don't even have ex1 & ex2, it's probably malformed, or an outdated client anyway?
    if parts.len() < VER_PARTS_MIN_LEN {
        tracing::error!(
            "Client's version string is malformed, it's reporting {} expansion(s), expected at least {}! Rejecting session!",
            parts.len() - 1,
            VER_PARTS_MIN_LEN - 1
        );
        return None;
    }

    let mut version_data = VersionCheckData {
        game_exe_len: 0,
        game_exe_sha1_hash: "".to_string(),
        expansion_pack_versions: Vec::new(),
    };

    if !parts[0].starts_with(GAME_EXE_NAME) {
        tracing::error!(
            "Client's version string is malformed, it doesn't declare the name of the game executable correctly! Got {}, expected it to start with {}! Rejecting session!",
            parts[0],
            GAME_EXE_NAME
        );
        return None;
    } else {
        let exe_verification_info: Vec<&str> = parts[0].split("/").collect();
        if exe_verification_info.len() != EXE_VERIFICATION_INFO_PARTS {
            tracing::error!(
                "Client's version string is malformed, the exe section doesn't contain enough parts! Rejecting session! Got {}, expected {}",
                exe_verification_info.len(),
                EXE_VERIFICATION_INFO_PARTS
            );
            return None;
        }

        match exe_verification_info[1].parse::<usize>() {
            Ok(client_exe_len) => {
                version_data.game_exe_len = client_exe_len;
            }
            Err(err) => {
                tracing::error!(
                    "Client's version string is malformed, unable to parse executable length field! Rejecting session! Got {}, further info: {}",
                    exe_verification_info[1],
                    err
                );
                return None;
            }
        }

        // We don't check this for validity (length or otherwise) here since it'll be verified later when the actual SHA-1 hashing is done.
        version_data.game_exe_sha1_hash = exe_verification_info[2].to_string();

        // The remaining parts between '+'s are expansion version strings
        for expansion_ver_str in parts[1..].iter() {
            if expansion_ver_str.len() != EXPANSION_VERSION_STR_LEN {
                tracing::error!(
                    "Client's version string is malformed, an expansion's version string is the incorrect length! Got {}, expected {}, string was {}",
                    expansion_ver_str.len(),
                    EXPANSION_VERSION_STR_LEN,
                    expansion_ver_str
                );
                return None;
            }

            version_data
                .expansion_pack_versions
                .push(expansion_ver_str.to_string());
        }
    }

    Some(version_data)
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
                                    ..
                                } => {
                                    tracing::info!(
                                        "Client {session_id} ({version_info}) logging in!"
                                    );

                                    let config = get_config();

                                    // The lobby server does its own version check as well, but it can be turned off if desired.
                                    if config.enforce_validity_checks
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
                                ClientLobbyIpcData::ServiceLogin {
                                    sequence,
                                    account_index,
                                    ..
                                } => {
                                    connection.selected_service_account = Some(
                                        connection.service_accounts[*account_index as usize].id
                                            as u32,
                                    );
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
                                    ..
                                } => {
                                    tracing::info!("Client is joining the world with {content_id}");

                                    let our_actor_id;

                                    // find the actor id for this content id
                                    // NOTE: This is NOT the ideal solution. I theorize the lobby server has it's own records with this information.
                                    {
                                        let ipc_segment = CustomIpcSegment {
                                            op_code: CustomIpcType::GetActorId,
                                            data: CustomIpcData::GetActorId {
                                                content_id: *content_id,
                                            },
                                            ..Default::default()
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
                            SegmentData::KeepAliveResponse { .. } => {
                                // we can throw this away
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

#[cfg(test)]
mod tests {
    use super::{validate_client_version_string};

    #[test]
    fn test_validate_client_version_string() {
        // Uncomment if interested in seeing output from the test!
        //tracing_subscriber::fmt::init();

        // These strings match 7.3h1 version checks, but they don't need to be updated for testing purposes.
        const BASE_STR: &str = "ffxiv_dx11.exe/50154824/60b27b131deebe9b2b7914012618293c85aff247";
        const EX1_STR: &str = "2025.07.25.0000";
        const EX2_STR: &str = "2025.07.23.0000";
        const REST_EX_STR: &str = "2025.08.07.0000"; // expansions past ex2 seem to always have the same version.
        const INVALID_EXE_NAME_STR: &str =
            "INVALID_EXE_NAME.exe/0/INVALID_HASH+INVALID_EX1+INVALID_EX2";
        const INVALID_EXE_SIZE_STR: &str =
            "ffxiv_dx11.exe/INVALID_SIZE/INVALID_HASH+INVALID_EX1+INVALID_EX2";

        let hw_str: &str = &format!("{}+{}", BASE_STR, EX1_STR);
        let hw_stb_str: &str = &format!("{}+{}", hw_str, EX2_STR);
        let hw_stb_shb_str: &str = &format!("{}+{}", hw_stb_str, REST_EX_STR);
        let hw_stb_shb_ew_str: &str = &format!("{}+{}", hw_stb_shb_str, REST_EX_STR);
        let full_dt_str: &str = &format!("{}+{}", hw_stb_shb_ew_str, REST_EX_STR);

        // Test valid cases first, starting with HW + StB only, and adding one expansion per test.
        assert_eq!(validate_client_version_string(hw_stb_str).is_some(), true);
        assert_eq!(
            validate_client_version_string(hw_stb_shb_str).is_some(),
            true
        );
        assert_eq!(
            validate_client_version_string(hw_stb_shb_ew_str).is_some(),
            true
        );
        assert_eq!(validate_client_version_string(full_dt_str).is_some(), true);

        // Next, ensure cases that don't provide enough expansions, no expansions at all, or are otherwise obviously malformed in some way, fail.
        assert_eq!(validate_client_version_string(BASE_STR).is_none(), true);
        assert_eq!(validate_client_version_string(hw_str).is_none(), true);
        assert_eq!(
            validate_client_version_string(INVALID_EXE_SIZE_STR).is_none(),
            true
        );
        assert_eq!(
            validate_client_version_string(INVALID_EXE_NAME_STR).is_none(),
            true
        );
    }
}
