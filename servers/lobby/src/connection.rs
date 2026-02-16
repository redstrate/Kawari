use std::{cmp::min, fs, path::MAIN_SEPARATOR_STR, time::Instant};

use physis::blowfish::LobbyBlowfish;
use tokio::net::TcpStream;

use kawari::{
    common::{GAME_SERVICE, WORLD_NAME},
    config::get_config,
    constants::SUPPORTED_EXPAC_VERSIONS,
    ipc::lobby::{DistRetainerInfo, NackReply},
    packet::{
        CompressionType, ConnectionState, ConnectionType, PacketSegment, SegmentData, SegmentType,
        generate_encryption_key, parse_packet, send_custom_world_packet, send_packet,
    },
};

use kawari::ipc::kawari::{CustomIpcData, CustomIpcSegment};
use kawari::ipc::lobby::{
    CharaMake, CharacterDetails, ClientLobbyIpcSegment, DistWorldInfo, LobbyCharacterActionKind,
    LoginReply, Server, ServerLobbyIpcData, ServerLobbyIpcSegment, ServiceAccount,
    ServiceLoginReply,
};

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

        for (client_version, expected_version) in client_version_data
            .expansion_pack_versions
            .iter()
            .zip(SUPPORTED_EXPAC_VERSIONS.iter())
        {
            // The client doesn't send a patch2 value in its expansion version strings, so we just pretend it doesn't exist on our side.
            let expected_version = &expected_version[..expected_version.len() - 5].to_string();
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

/// Represents a single connection between an instance of the client and the lobby server.
pub struct LobbyConnection {
    pub socket: TcpStream,
    pub session_id: Option<String>,
    pub state: ConnectionState,
    pub stored_character_creation_name: String,
    pub service_accounts: Vec<ServiceAccount>,
    pub selected_service_account: Option<u64>,
    pub last_keep_alive: Instant,
}

impl LobbyConnection {
    pub fn parse_packet(&mut self, data: &[u8]) -> Vec<PacketSegment<ClientLobbyIpcSegment>> {
        parse_packet(data, &mut self.state)
    }

    pub async fn send_segment(&mut self, segment: PacketSegment<ServerLobbyIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Lobby,
            CompressionType::Uncompressed,
            &[segment],
        )
        .await;
    }

    /// Send an acknowledgement to the client that we generated a valid encryption key.
    pub async fn initialize_encryption(&mut self, phrase: &str, key: &[u8; 4]) {
        // Generate an encryption key for this client
        let client_key = generate_encryption_key(key, phrase);

        let mut data = 0xE0003C2Au32.to_le_bytes().to_vec();
        data.resize(0x280, 0);

        let blowfish = LobbyBlowfish::new(&client_key);
        blowfish.encrypt(&mut data);

        self.state = ConnectionState::Lobby { blowfish };

        self.send_segment(PacketSegment {
            segment_type: SegmentType::SecurityInitialize,
            data: SegmentData::SecurityInitialize { data },
            ..Default::default()
        })
        .await;
    }

    /// Send the service account list to the client.
    pub async fn send_account_list(&mut self) {
        let service_account_list = ServerLobbyIpcData::LoginReply(LoginReply {
            sequence: 0,
            num_service_accounts: self.service_accounts.len() as u8,
            unk1: 3,
            unk2: 0x99,
            service_accounts: self.service_accounts.to_vec(),
        });

        let ipc = ServerLobbyIpcSegment::new(service_account_list);

        self.send_segment(PacketSegment {
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
            ..Default::default()
        })
        .await;
    }

    /// Send the world, retainer and character list to the client.
    pub async fn send_lobby_info(&mut self, sequence: u64) {
        let mut packets = Vec::new();
        // send them the server list
        {
            let config = get_config();

            let mut servers = [Server {
                id: config.world.world_id,
                name: WORLD_NAME.to_string(),
                restricted: !config.world.accept_new_characters,
                exp_bonus: config.world.exp_bonus,
                ..Default::default()
            }]
            .to_vec();
            // add any empty boys
            servers.resize(6, Server::default());

            let lobby_server_list = ServerLobbyIpcData::DistWorldInfo(DistWorldInfo {
                sequence,
                unk1: 1,
                num_servers: 1,
                servers,
                ..Default::default()
            });

            let ipc = ServerLobbyIpcSegment::new(lobby_server_list);

            let response_packet = PacketSegment {
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc(ipc),
                ..Default::default()
            };
            packets.push(response_packet);
        }

        // send them the retainer list
        {
            let lobby_retainer_list =
                ServerLobbyIpcData::DistRetainerInfo(DistRetainerInfo::default());

            let ipc = ServerLobbyIpcSegment::new(lobby_retainer_list);

            let response_packet = PacketSegment {
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc(ipc),
                ..Default::default()
            };
            packets.push(response_packet);
        }

        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Lobby,
            CompressionType::Uncompressed,
            &packets,
        )
        .await;

        // now send them the character list
        {
            let charlist_request = CustomIpcSegment::new(CustomIpcData::RequestCharacterList {
                service_account_id: self.selected_service_account.unwrap(),
            });

            let name_response = send_custom_world_packet(charlist_request)
                .await
                .expect("Failed to get name request packet!");
            let CustomIpcData::RequestCharacterListResponse { characters } = &name_response.data
            else {
                panic!("Unexpedted custom IPC type!")
            };

            let mut characters = characters.to_vec();

            let max_characters_per_world = 8;
            let can_create_character = characters.len() < max_characters_per_world;

            for i in 0..4 {
                let mut characters_in_packet = Vec::new();
                for _ in 0..min(characters.len(), 2) {
                    characters_in_packet.push(characters.swap_remove(0));
                }
                // add any empty boys
                characters_in_packet.resize(2, CharacterDetails::default());

                let lobby_character_list = if i == 3 {
                    // On the last packet, add the account-wide information
                    ServiceLoginReply {
                        sequence,
                        counter: (i * 4) + 1, // TODO: why the + 1 here?
                        num_in_packet: characters_in_packet.len() as u8,
                        unk3: 0,
                        unk4: 0,
                        unk6: 64,
                        days_subscribed: 30,
                        remaining_days: 30,
                        days_to_next_rank: 0,
                        unk8: if can_create_character { 1 } else { 0 },
                        max_characters_on_world: max_characters_per_world as u16,
                        entitled_expansion: 5, // TODO: use service account max expansion
                        characters: characters_in_packet,
                        ..Default::default()
                    }
                } else {
                    ServiceLoginReply {
                        sequence,
                        counter: i * 4,
                        num_in_packet: characters_in_packet.len() as u8,
                        characters: characters_in_packet,
                        ..Default::default()
                    }
                };

                let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::ServiceLoginReply(
                    lobby_character_list,
                ));

                self.send_segment(PacketSegment {
                    segment_type: SegmentType::Ipc,
                    data: SegmentData::Ipc(ipc),
                    ..Default::default()
                })
                .await;
            }
        }
    }

    /// Send the host information for the world server to the client.
    pub async fn send_enter_world(&mut self, sequence: u64, content_id: u64, actor_id: u32) {
        let config = get_config();

        let enter_world = ServerLobbyIpcData::GameLoginReply {
            sequence,
            actor_id,
            content_id,
            token: String::new(),
            port: config.world.port,
            host: config.world.server_name,
        };

        let ipc = ServerLobbyIpcSegment::new(enter_world);

        self.send_segment(PacketSegment {
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
            ..Default::default()
        })
        .await;
    }

    /// Send a lobby error to the client.
    pub async fn send_error(&mut self, sequence: u64, error: u32, exd_error: u16) {
        let lobby_error = ServerLobbyIpcData::NackReply(NackReply {
            sequence,
            error,
            exd_error_id: exd_error,
            ..Default::default()
        });

        let ipc = ServerLobbyIpcSegment::new(lobby_error);

        self.send_segment(PacketSegment {
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
            ..Default::default()
        })
        .await;
    }

    pub async fn handle_character_action(&mut self, character_action: &CharaMake) {
        match &character_action.action {
            LobbyCharacterActionKind::ReserveName => {
                tracing::info!(
                    "Player is requesting {} as a new character name!",
                    character_action.name
                );

                // check with the world server if the name is available
                let name_request = CustomIpcSegment::new(CustomIpcData::CheckNameIsAvailable {
                    name: character_action.name.clone(),
                });

                let is_free;
                if let Some(name_response) = send_custom_world_packet(name_request).await {
                    let CustomIpcData::NameIsAvailableResponse { free } = &name_response.data
                    else {
                        panic!("Unexpedted custom IPC type!")
                    };
                    is_free = *free;
                } else {
                    tracing::warn!("Failed to contact World server, assuming name isn't free.");
                    is_free = false;
                }

                if is_free {
                    self.stored_character_creation_name = character_action.name.clone();

                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::ReserveName,
                        details: CharacterDetails {
                            character_name: character_action.name.clone(),
                            origin_server_name: WORLD_NAME.to_string(),
                            current_server_name: WORLD_NAME.to_string(),
                            ..Default::default()
                        },
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                } else {
                    let ipc =
                        ServerLobbyIpcSegment::new(ServerLobbyIpcData::NackReply(NackReply {
                            sequence: character_action.sequence,
                            error: 0x00000bdb,
                            exd_error_id: 0x32cc,
                            ..Default::default()
                        }));

                    let response_packet = PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    };
                    self.send_segment(response_packet).await;
                }
            }
            LobbyCharacterActionKind::Create => {
                tracing::info!("Player is creating a new character!");

                let our_actor_id;
                let our_content_id;

                // tell the world server to create this character
                {
                    let ipc_segment =
                        CustomIpcSegment::new(CustomIpcData::RequestCreateCharacter {
                            service_account_id: self.selected_service_account.unwrap(),
                            name: self.stored_character_creation_name.clone(), // TODO: worth double-checking, but AFAIK we have to store it this way?
                            chara_make_json: character_action.json.clone(),
                        });

                    if let Some(response_segment) = send_custom_world_packet(ipc_segment).await {
                        match &response_segment.data {
                            CustomIpcData::CharacterCreated {
                                actor_id,
                                content_id,
                            } => {
                                our_actor_id = *actor_id;
                                our_content_id = *content_id;
                            }
                            _ => panic!("Unexpected custom IPC packet type here!"),
                        }
                    } else {
                        // "The lobby server has encountered a problem."
                        self.send_error(character_action.sequence, 2002, 13006)
                            .await;
                        return;
                    }
                }

                tracing::info!(
                    "Got new player info from world server: {our_content_id} {our_actor_id}"
                );

                // a slightly different character created packet now
                {
                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::Create,
                        details: CharacterDetails {
                            player_id: our_actor_id as u64, // TODO: not correct
                            content_id: our_content_id,
                            character_name: character_action.name.clone(),
                            origin_server_name: WORLD_NAME.to_string(),
                            current_server_name: WORLD_NAME.to_string(),
                            ..Default::default()
                        },
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                }
            }
            LobbyCharacterActionKind::Rename => todo!(),
            LobbyCharacterActionKind::Delete => {
                // tell the world server to yeet this guy
                {
                    let ipc_segment = CustomIpcSegment::new(CustomIpcData::DeleteCharacter {
                        content_id: character_action.content_id,
                    });

                    let _ = send_custom_world_packet(ipc_segment).await;

                    // we intentionally don't care about the response right now, it's not expected to fail
                }

                // send a confirmation that the deletion was successful
                {
                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::Delete,
                        details: CharacterDetails {
                            player_id: 0, // TODO: fill maybe?
                            content_id: character_action.content_id,
                            character_name: character_action.name.clone(),
                            origin_server_name: WORLD_NAME.to_string(),
                            current_server_name: WORLD_NAME.to_string(),
                            ..Default::default()
                        },
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                }
            }
            LobbyCharacterActionKind::Move => todo!(),
            LobbyCharacterActionKind::RemakeRetainer => todo!(),
            LobbyCharacterActionKind::RemakeChara => {
                // tell the world server to turn this guy into a catgirl
                {
                    let ipc_segment = CustomIpcSegment::new(CustomIpcData::RemakeCharacter {
                        content_id: character_action.content_id,
                        chara_make_json: character_action.json.clone(),
                    });

                    let _ = send_custom_world_packet(ipc_segment).await;

                    // we intentionally don't care about the response right now, it's not expected to fail
                }

                // send a confirmation that the remake was successful
                {
                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::RemakeChara,
                        details: CharacterDetails {
                            player_id: 0, // TODO: fill maybe?
                            content_id: character_action.content_id,
                            character_name: character_action.name.clone(),
                            origin_server_name: WORLD_NAME.to_string(),
                            current_server_name: WORLD_NAME.to_string(),
                            ..Default::default()
                        },
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                }
            }
            LobbyCharacterActionKind::SettingsUploadBegin => {
                // send a confirmation that the upload was successful
                {
                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::SettingsUploadBegin,
                        details: CharacterDetails::default(),
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                }
            }
            LobbyCharacterActionKind::SettingsUpload => todo!(),
            LobbyCharacterActionKind::WorldVisit => todo!(),
            LobbyCharacterActionKind::DataCenterToken => todo!(),
            LobbyCharacterActionKind::Request => todo!(),
            LobbyCharacterActionKind::UploadData => {
                // send a confirmation that the upload was successful
                {
                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::UploadData,
                        details: CharacterDetails::default(),
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                }
            }
            LobbyCharacterActionKind::SettingsUploadCompleted => {
                // send a confirmation that the upload was successful
                {
                    let ipc = ServerLobbyIpcSegment::new(ServerLobbyIpcData::CharaMakeReply {
                        sequence: character_action.sequence + 1,
                        unk1: 0x1,
                        unk2: 0x1,
                        action: LobbyCharacterActionKind::SettingsUploadCompleted,
                        details: CharacterDetails::default(),
                    });

                    self.send_segment(PacketSegment {
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc(ipc),
                        ..Default::default()
                    })
                    .await;
                }
            }
        }
    }

    pub async fn login(&mut self, sequence: u64, session_id: &str, version_info: &str) {
        tracing::info!("Client {session_id} ({version_info}) logging in!");

        let config = get_config();

        // The lobby server does its own version check as well, but it can be turned off if desired.
        if config.tweaks.enforce_validity_checks && !do_game_version_check(version_info) {
            // "A version update is required."
            self.send_error(sequence, 1012, 13101).await;
            return;
        }

        let Ok(mut login_reply) = ureq::get(format!(
            "{}/_private/service_accounts",
            config.login.server_name,
        ))
        .query("sid", session_id)
        .query("service", GAME_SERVICE)
        .call() else {
            tracing::warn!("Failed to contact login server, is it running?");
            // "The lobby server connection has encountered an error."
            self.send_error(sequence, 2002, 13001).await;
            return;
        };

        let Ok(body) = login_reply.body_mut().read_to_string() else {
            tracing::warn!("Failed to parse login server response, is it running?");
            // "The lobby server connection has encountered an error."
            self.send_error(sequence, 2002, 13001).await;
            return;
        };

        let service_accounts: Option<Vec<ServiceAccount>> = serde_json::from_str(&body).ok();
        if let Some(service_accounts) = service_accounts {
            if service_accounts.is_empty() {
                tracing::warn!(
                    "This account has no service accounts attached, how did this happen?"
                );

                /* "<the game> has not yet been registered on this platform or your service account's subscription has expired. Please close the application and complete the registration process. If you would like to add a platform to your service account or renew your subscription, please visit the <website>). To register another platform, you must purchase a license for the applicable platform or complete the registration process using the registration code included with your purchase." */
                self.send_error(sequence, 2002, 13209).await;
            } else {
                self.service_accounts = service_accounts;
                self.session_id = Some(session_id.to_string());
                self.send_account_list().await;
            }
        } else {
            tracing::warn!("Failed to parse service accounts from the login server!");

            // "The lobby server has encountered a problem."
            self.send_error(sequence, 2002, 13006).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_client_version_string;

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

        let hw_str: &str = &format!("{BASE_STR}+{EX1_STR}");
        let hw_stb_str: &str = &format!("{hw_str}+{EX2_STR}");
        let hw_stb_shb_str: &str = &format!("{hw_stb_str}+{REST_EX_STR}");
        let hw_stb_shb_ew_str: &str = &format!("{hw_stb_shb_str}+{REST_EX_STR}");
        let full_dt_str: &str = &format!("{hw_stb_shb_ew_str}+{REST_EX_STR}");

        // Test valid cases first, starting with HW + StB only, and adding one expansion per test.
        assert!(validate_client_version_string(hw_stb_str).is_some());
        assert!(validate_client_version_string(hw_stb_shb_str).is_some());
        assert!(validate_client_version_string(hw_stb_shb_ew_str).is_some());
        assert!(validate_client_version_string(full_dt_str).is_some());

        // Next, ensure cases that don't provide enough expansions, no expansions at all, or are otherwise obviously malformed in some way, fail.
        assert!(validate_client_version_string(BASE_STR).is_none());
        assert!(validate_client_version_string(hw_str).is_none());
        assert!(validate_client_version_string(INVALID_EXE_SIZE_STR).is_none());
        assert!(validate_client_version_string(INVALID_EXE_NAME_STR).is_none());
    }
}
