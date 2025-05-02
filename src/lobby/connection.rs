use std::cmp::min;

use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::{
    RECEIVE_BUFFER_SIZE,
    blowfish::Blowfish,
    common::{
        custom_ipc::{CustomIpcData, CustomIpcSegment, CustomIpcType},
        timestamp_secs,
        workdefinitions::CharaMake,
    },
    config::get_config,
    oodle::OodleNetwork,
    opcodes::ServerLobbyIpcType,
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentData, SegmentType,
        generate_encryption_key, parse_packet, send_packet,
    },
};

use super::ipc::{
    CharacterDetails, LobbyCharacterAction, LobbyCharacterActionKind, LobbyCharacterList,
    LobbyServerList, LobbyServiceAccountList, Server, ServerLobbyIpcData, ServerLobbyIpcSegment,
    ServiceAccount,
};
use crate::lobby::ipc::ClientLobbyIpcSegment;

/// Represents a single connection between an instance of the client and the lobby server.
pub struct LobbyConnection {
    pub socket: TcpStream,

    pub session_id: Option<String>,

    pub state: PacketState,

    pub stored_character_creation_name: String,

    pub world_name: String,

    pub service_accounts: Vec<ServiceAccount>,

    pub selected_service_account: Option<u32>,
}

impl LobbyConnection {
    pub async fn parse_packet(
        &mut self,
        data: &[u8],
    ) -> (Vec<PacketSegment<ClientLobbyIpcSegment>>, ConnectionType) {
        parse_packet(data, &mut self.state).await
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
        self.state.client_key = Some(generate_encryption_key(key, phrase));

        let mut data = 0xE0003C2Au32.to_le_bytes().to_vec();
        data.resize(0x280, 0);

        let blowfish = Blowfish::new(&self.state.client_key.unwrap());
        blowfish.encrypt(&mut data);

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::SecurityInitialize,
            data: SegmentData::SecurityInitialize { data },
        })
        .await;
    }

    /// Send the service account list to the client.
    pub async fn send_account_list(&mut self) {
        let service_account_list = ServerLobbyIpcData::LoginReply(LobbyServiceAccountList {
            sequence: 0,
            num_service_accounts: self.service_accounts.len() as u8,
            unk1: 3,
            unk2: 0x99,
            service_accounts: self.service_accounts.to_vec(),
        });

        let ipc = ServerLobbyIpcSegment {
            unk1: 0,
            unk2: 0,
            op_code: ServerLobbyIpcType::LoginReply,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: service_account_list,
        };

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
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
                index: 0,
                flags: 0,
                icon: 0,
                name: self.world_name.clone(),
            }]
            .to_vec();
            // add any empty boys
            servers.resize(6, Server::default());

            let lobby_server_list = ServerLobbyIpcData::DistWorldInfo(LobbyServerList {
                sequence: 0,
                unk1: 1,
                offset: 0,
                num_servers: 1,
                servers,
            });

            let ipc = ServerLobbyIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: ServerLobbyIpcType::DistWorldInfo,
                server_id: 0,
                timestamp: timestamp_secs(),
                data: lobby_server_list,
            };

            let response_packet = PacketSegment {
                source_actor: 0,
                target_actor: 0,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            };
            packets.push(response_packet);
        }

        // send them the retainer list
        {
            let lobby_retainer_list = ServerLobbyIpcData::DistRetainerInfo { unk1: 1 };

            let ipc = ServerLobbyIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: ServerLobbyIpcType::DistRetainerInfo,
                server_id: 0,
                timestamp: timestamp_secs(),
                data: lobby_retainer_list,
            };

            let response_packet = PacketSegment {
                source_actor: 0,
                target_actor: 0,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
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
            let charlist_request = CustomIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: CustomIpcType::RequestCharacterList,
                server_id: 0,
                timestamp: 0,
                data: CustomIpcData::RequestCharacterList {
                    service_account_id: self.selected_service_account.unwrap(),
                },
            };

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
                    LobbyCharacterList {
                        sequence,
                        counter: (i * 4) + 1, // TODO: why the + 1 here?
                        num_in_packet: characters_in_packet.len() as u8,
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        unk4: 128,
                        unk5: [0; 7],
                        unk6: 0,
                        veteran_rank: 0,
                        unk7: 0,
                        days_subscribed: 30,
                        remaining_days: 30,
                        days_to_next_rank: 0,
                        unk8: 8,
                        max_characters_on_world: 8,
                        entitled_expansion: 5,
                        characters: characters_in_packet,
                    }
                } else {
                    LobbyCharacterList {
                        sequence,
                        counter: i * 4,
                        num_in_packet: characters_in_packet.len() as u8,
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                        unk4: 0,
                        unk5: [0; 7],
                        unk6: 0,
                        veteran_rank: 0,
                        unk7: 0,
                        days_subscribed: 0,
                        remaining_days: 0,
                        days_to_next_rank: 0,
                        max_characters_on_world: 0,
                        unk8: 0,
                        entitled_expansion: 0,
                        characters: characters_in_packet,
                    }
                };

                let ipc = ServerLobbyIpcSegment {
                    unk1: 0,
                    unk2: 0,
                    op_code: ServerLobbyIpcType::ServiceLoginReply,
                    server_id: 0,
                    timestamp: timestamp_secs(),
                    data: ServerLobbyIpcData::ServiceLoginReply(lobby_character_list),
                };

                self.send_segment(PacketSegment {
                    source_actor: 0,
                    target_actor: 0,
                    segment_type: SegmentType::Ipc,
                    data: SegmentData::Ipc { data: ipc },
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
            host: config.world.listen_address,
        };

        let ipc = ServerLobbyIpcSegment {
            unk1: 0,
            unk2: 0,
            op_code: ServerLobbyIpcType::GameLoginReply,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: enter_world,
        };

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    /// Send a lobby error to the client.
    pub async fn send_error(&mut self, sequence: u64, error: u32, exd_error: u16) {
        let lobby_error = ServerLobbyIpcData::NackReply {
            sequence,
            error,
            value: 0,
            exd_error_id: exd_error,
            unk1: 1,
        };

        let ipc = ServerLobbyIpcSegment {
            unk1: 0,
            unk2: 0,
            op_code: ServerLobbyIpcType::NackReply,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: lobby_error,
        };

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn handle_character_action(&mut self, character_action: &LobbyCharacterAction) {
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

                let name_response = send_custom_world_packet(name_request)
                    .await
                    .expect("Failed to get name request packet!");
                let CustomIpcData::NameIsAvailableResponse { free } = &name_response.data else {
                    panic!("Unexpedted custom IPC type!")
                };

                tracing::info!("Is name free? {free}");

                if *free {
                    self.stored_character_creation_name = character_action.name.clone();

                    let ipc = ServerLobbyIpcSegment {
                        unk1: 0,
                        unk2: 0,
                        op_code: ServerLobbyIpcType::CharaMakeReply,
                        server_id: 0,
                        timestamp: 0,
                        data: ServerLobbyIpcData::CharaMakeReply {
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
                        },
                    };

                    self.send_segment(PacketSegment {
                        source_actor: 0x0,
                        target_actor: 0x0,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc { data: ipc },
                    })
                    .await;
                } else {
                    let ipc = ServerLobbyIpcSegment {
                        unk1: 0,
                        unk2: 0,
                        op_code: ServerLobbyIpcType::NackReply,
                        server_id: 0,
                        timestamp: 0,
                        data: ServerLobbyIpcData::NackReply {
                            sequence: character_action.sequence,
                            error: 0x00000bdb,
                            exd_error_id: 0x32cc,
                            value: 0,
                            unk1: 0,
                        },
                    };

                    let response_packet = PacketSegment {
                        source_actor: 0x0,
                        target_actor: 0x0,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc { data: ipc },
                    };
                    self.send_segment(response_packet).await;
                }
            }
            LobbyCharacterActionKind::Create => {
                tracing::info!("Player is creating a new character!");

                let our_actor_id;
                let our_content_id;

                dbg!(CharaMake::from_json(&character_action.json));

                // tell the world server to create this character
                {
                    let ipc_segment = CustomIpcSegment {
                        unk1: 0,
                        unk2: 0,
                        op_code: CustomIpcType::RequestCreateCharacter,
                        server_id: 0,
                        timestamp: 0,
                        data: CustomIpcData::RequestCreateCharacter {
                            service_account_id: self.selected_service_account.unwrap(),
                            name: self.stored_character_creation_name.clone(), // TODO: worth double-checking, but AFAIK we have to store it this way?
                            chara_make_json: character_action.json.clone(),
                        },
                    };

                    let response_segment = send_custom_world_packet(ipc_segment).await.unwrap();
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
                }

                tracing::info!(
                    "Got new player info from world server: {our_content_id} {our_actor_id}"
                );

                // a slightly different character created packet now
                {
                    let ipc = ServerLobbyIpcSegment {
                        unk1: 0,
                        unk2: 0,
                        op_code: ServerLobbyIpcType::CharaMakeReply,
                        server_id: 0,
                        timestamp: 0,
                        data: ServerLobbyIpcData::CharaMakeReply {
                            sequence: character_action.sequence + 1,
                            unk1: 0x1,
                            unk2: 0x1,
                            action: LobbyCharacterActionKind::Create,
                            details: CharacterDetails {
                                actor_id: our_actor_id,
                                content_id: our_content_id,
                                character_name: character_action.name.clone(),
                                origin_server_name: self.world_name.clone(),
                                current_server_name: self.world_name.clone(),
                                ..Default::default()
                            },
                        },
                    };

                    self.send_segment(PacketSegment {
                        source_actor: 0x0,
                        target_actor: 0x0,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc { data: ipc },
                    })
                    .await;
                }
            }
            LobbyCharacterActionKind::Rename => todo!(),
            LobbyCharacterActionKind::Delete => {
                // tell the world server to yeet this guy
                {
                    let ipc_segment = CustomIpcSegment {
                        unk1: 0,
                        unk2: 0,
                        op_code: CustomIpcType::DeleteCharacter,
                        server_id: 0,
                        timestamp: 0,
                        data: CustomIpcData::DeleteCharacter {
                            content_id: character_action.content_id,
                        },
                    };

                    let _ = send_custom_world_packet(ipc_segment).await.unwrap();

                    // we intentionally don't care about the response right now, it's not expected to fail
                }

                // send a confirmation that the deletion was successful
                {
                    let ipc = ServerLobbyIpcSegment {
                        unk1: 0,
                        unk2: 0,
                        op_code: ServerLobbyIpcType::CharaMakeReply,
                        server_id: 0,
                        timestamp: 0,
                        data: ServerLobbyIpcData::CharaMakeReply {
                            sequence: character_action.sequence + 1,
                            unk1: 0x1,
                            unk2: 0x1,
                            action: LobbyCharacterActionKind::Delete,
                            details: CharacterDetails {
                                actor_id: 0, // TODO: fill maybe?
                                content_id: character_action.content_id,
                                character_name: character_action.name.clone(),
                                origin_server_name: self.world_name.clone(),
                                current_server_name: self.world_name.clone(),
                                ..Default::default()
                            },
                        },
                    };

                    self.send_segment(PacketSegment {
                        source_actor: 0x0,
                        target_actor: 0x0,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc { data: ipc },
                    })
                    .await;
                }
            }
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
}

/// Sends a custom IPC packet to the world server, meant for private server-to-server communication.
/// Returns the first custom IPC segment returned.
pub async fn send_custom_world_packet(segment: CustomIpcSegment) -> Option<CustomIpcSegment> {
    let config = get_config();

    let addr = config.world.get_socketaddr();

    let mut stream = TcpStream::connect(addr).await.unwrap();

    let mut packet_state = PacketState {
        client_key: None,
        serverbound_oodle: OodleNetwork::new(),
        clientbound_oodle: OodleNetwork::new(),
    };

    let segment: PacketSegment<CustomIpcSegment> = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::KawariIpc,
        data: SegmentData::KawariIpc { data: segment },
    };

    send_packet(
        &mut stream,
        &mut packet_state,
        ConnectionType::None,
        CompressionType::Uncompressed,
        &[segment],
    )
    .await;

    // read response
    let mut buf = vec![0; RECEIVE_BUFFER_SIZE];
    let n = stream.read(&mut buf).await.expect("Failed to read data!");
    if n != 0 {
        let (segments, _) = parse_packet::<CustomIpcSegment>(&buf[..n], &mut packet_state).await;

        return match &segments[0].data {
            SegmentData::KawariIpc { data } => Some(data.clone()),
            _ => None,
        };
    }

    None
}
