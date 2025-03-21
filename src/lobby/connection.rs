use std::cmp::min;

use tokio::{io::AsyncReadExt, net::TcpStream};

use crate::{
    WORLD_ID, WORLD_NAME,
    blowfish::Blowfish,
    common::{
        custom_ipc::{CustomIpcData, CustomIpcSegment, CustomIpcType},
        timestamp_secs,
    },
    oodle::OodleNetwork,
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentType,
        generate_encryption_key, parse_packet, send_packet,
    },
};

use super::ipc::{
    CharacterDetails, LobbyCharacterList, LobbyServerList, LobbyServiceAccountList, Server,
    ServerLobbyIpcData, ServerLobbyIpcSegment, ServerLobbyIpcType, ServiceAccount,
};
use crate::lobby::ipc::ClientLobbyIpcSegment;

/// Represents a single connection between an instance of the client and the lobby server.
pub struct LobbyConnection {
    pub socket: TcpStream,

    pub session_id: Option<String>,

    pub state: PacketState,

    pub stored_character_creation_name: String,
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
            segment_type: SegmentType::InitializationEncryptionResponse { data },
        })
        .await;
    }

    /// Send the service account list to the client.
    pub async fn send_account_list(&mut self) {
        // send the client the service account list
        let service_accounts = [ServiceAccount {
            id: 0x002E4A2B,
            unk1: 0,
            index: 0,
            name: "FINAL FANTASY XIV".to_string(),
        }]
        .to_vec();

        let service_account_list =
            ServerLobbyIpcData::LobbyServiceAccountList(LobbyServiceAccountList {
                sequence: 0,
                num_service_accounts: service_accounts.len() as u8,
                unk1: 3,
                unk2: 0x99,
                service_accounts: service_accounts.to_vec(),
            });

        let ipc = ServerLobbyIpcSegment {
            unk1: 0,
            unk2: 0,
            op_code: ServerLobbyIpcType::LobbyServiceAccountList,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: service_account_list,
        };

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    /// Send the world, retainer and character list to the client.
    pub async fn send_lobby_info(&mut self, sequence: u64) {
        let mut packets = Vec::new();
        // send them the server list
        {
            let mut servers = [Server {
                id: WORLD_ID,
                index: 0,
                flags: 0,
                icon: 0,
                name: WORLD_NAME.to_string(),
            }]
            .to_vec();
            // add any empty boys
            servers.resize(6, Server::default());

            let lobby_server_list = ServerLobbyIpcData::LobbyServerList(LobbyServerList {
                sequence: 0,
                unk1: 1,
                offset: 0,
                num_servers: 1,
                servers,
            });

            let ipc = ServerLobbyIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: ServerLobbyIpcType::LobbyServerList,
                server_id: 0,
                timestamp: timestamp_secs(),
                data: lobby_server_list,
            };

            let response_packet = PacketSegment {
                source_actor: 0,
                target_actor: 0,
                segment_type: SegmentType::Ipc { data: ipc },
            };
            packets.push(response_packet);
        }

        // send them the retainer list
        {
            let lobby_retainer_list = ServerLobbyIpcData::LobbyRetainerList { unk1: 1 };

            let ipc = ServerLobbyIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: ServerLobbyIpcType::LobbyRetainerList,
                server_id: 0,
                timestamp: timestamp_secs(),
                data: lobby_retainer_list,
            };

            let response_packet = PacketSegment {
                source_actor: 0,
                target_actor: 0,
                segment_type: SegmentType::Ipc { data: ipc },
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
                    service_account_id: 0x1, // TODO: placeholder
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

            dbg!(&characters);

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
                        max_characters_on_world: 2,
                        entitled_expansion: 4,
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
                    op_code: ServerLobbyIpcType::LobbyCharacterList,
                    server_id: 0,
                    timestamp: timestamp_secs(),
                    data: ServerLobbyIpcData::LobbyCharacterList(lobby_character_list),
                };

                self.send_segment(PacketSegment {
                    source_actor: 0,
                    target_actor: 0,
                    segment_type: SegmentType::Ipc { data: ipc },
                })
                .await;
            }
        }
    }

    /// Send the host information for the world server to the client.
    pub async fn send_enter_world(&mut self, sequence: u64, content_id: u64, actor_id: u32) {
        let Some(session_id) = &self.session_id else {
            panic!("Missing session id!");
        };

        let enter_world = ServerLobbyIpcData::LobbyEnterWorld {
            sequence,
            actor_id,
            content_id,
            token: String::new(),
            port: 7100,
            host: "127.0.0.1".to_string(),
        };

        let ipc = ServerLobbyIpcSegment {
            unk1: 0,
            unk2: 0,
            op_code: ServerLobbyIpcType::LobbyEnterWorld,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: enter_world,
        };

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    /// Send a lobby error to the client.
    pub async fn send_error(&mut self, sequence: u64, error: u32, exd_error: u16) {
        let lobby_error = ServerLobbyIpcData::LobbyError {
            sequence,
            error,
            value: 0,
            exd_error_id: exd_error,
            unk1: 1,
        };

        let ipc = ServerLobbyIpcSegment {
            unk1: 0,
            unk2: 0,
            op_code: ServerLobbyIpcType::LobbyError,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: lobby_error,
        };

        self.send_segment(PacketSegment {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }
}

/// Sends a custom IPC packet to the world server, meant for private server-to-server communication.
/// Returns the first custom IPC segment returned.
pub async fn send_custom_world_packet(segment: CustomIpcSegment) -> Option<CustomIpcSegment> {
    let mut stream = TcpStream::connect("127.0.0.1:7100").await.unwrap();

    let mut packet_state = PacketState {
        client_key: None,
        serverbound_oodle: OodleNetwork::new(),
        clientbound_oodle: OodleNetwork::new(),
    };

    let segment: PacketSegment<CustomIpcSegment> = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::CustomIpc { data: segment },
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
    let mut buf = [0; 10024]; // TODO: this large buffer is just working around these packets not being compressed, but they really should be!
    let n = stream.read(&mut buf).await.expect("Failed to read data!");

    println!("Got {n} bytes of response!");

    let (segments, _) = parse_packet::<CustomIpcSegment>(&buf[..n], &mut packet_state).await;

    match &segments[0].segment_type {
        SegmentType::CustomIpc { data } => Some(data.clone()),
        _ => None,
    }
}
