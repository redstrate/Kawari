use std::{cmp::min, time::Instant};

use tokio::net::TcpStream;

use crate::{
    blowfish::Blowfish,
    config::get_config,
    ipc::lobby::{DistRetainerInfo, NackReply},
    packet::{
        CompressionType, ConnectionState, ConnectionType, PacketSegment, SegmentData, SegmentType,
        generate_encryption_key, parse_packet, send_custom_world_packet, send_packet,
    },
};

use crate::ipc::kawari::{CustomIpcData, CustomIpcSegment};
use crate::ipc::lobby::{
    CharaMake, CharacterDetails, ClientLobbyIpcSegment, DistWorldInfo, LobbyCharacterActionKind,
    LoginReply, Server, ServerLobbyIpcData, ServerLobbyIpcSegment, ServiceAccount,
    ServiceLoginReply,
};

/// Represents a single connection between an instance of the client and the lobby server.
pub struct LobbyConnection {
    pub socket: TcpStream,

    pub session_id: Option<String>,

    pub state: ConnectionState,

    pub stored_character_creation_name: String,

    pub world_name: String,

    pub service_accounts: Vec<ServiceAccount>,

    pub selected_service_account: Option<u32>,

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

        let blowfish = Blowfish::new(&client_key);
        blowfish.encrypt(&mut data);

        self.state = ConnectionState::Lobby { client_key };

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
                name: self.world_name.clone(),
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
            let CustomIpcData::RequestCharacterListRepsonse { characters } = &name_response.data
            else {
                panic!("Unexpedted custom IPC type!")
            };

            let mut characters = characters.to_vec();

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
                        unk3: 192,
                        unk6: 192,
                        days_subscribed: 30,
                        remaining_days: 30,
                        days_to_next_rank: 0,
                        unk8: 520,
                        max_characters_on_world: 8,
                        entitled_expansion: 5,
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
                            origin_server_name: self.world_name.clone(),
                            current_server_name: self.world_name.clone(),
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
                            origin_server_name: self.world_name.clone(),
                            current_server_name: self.world_name.clone(),
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
                            origin_server_name: self.world_name.clone(),
                            current_server_name: self.world_name.clone(),
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

                // send a confirmation that the remakewas successful
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
                            origin_server_name: self.world_name.clone(),
                            current_server_name: self.world_name.clone(),
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
            LobbyCharacterActionKind::SettingsUploadBegin => todo!(),
            LobbyCharacterActionKind::SettingsUpload => todo!(),
            LobbyCharacterActionKind::WorldVisit => todo!(),
            LobbyCharacterActionKind::DataCenterToken => todo!(),
            LobbyCharacterActionKind::Request => todo!(),
            LobbyCharacterActionKind::UploadData => todo!(),
        }
    }
}
