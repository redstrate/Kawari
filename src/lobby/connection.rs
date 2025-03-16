use std::cmp::min;

use tokio::net::TcpStream;

use crate::{
    CHAR_NAME, CONTENT_ID, CUSTOMIZE_DATA, DEITY, NAMEDAY_DAY, NAMEDAY_MONTH, WORLD_ID, WORLD_NAME,
    ZONE_ID,
    blowfish::Blowfish,
    common::timestamp_secs,
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentType,
        generate_encryption_key, parse_packet, send_packet,
    },
};

use super::{
    client_select_data::ClientSelectData,
    ipc::{
        CharacterDetails, LobbyCharacterList, LobbyServerList, LobbyServiceAccountList, Server,
        ServerLobbyIpcData, ServerLobbyIpcSegment, ServerLobbyIpcType, ServiceAccount,
    },
};
use crate::lobby::ipc::ClientLobbyIpcSegment;

pub struct LobbyConnection {
    pub socket: TcpStream,

    pub state: PacketState,
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
            &[segment],
            &mut self.state,
            CompressionType::Uncompressed,
        )
        .await;
    }

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
            &packets,
            &mut self.state,
            CompressionType::Uncompressed,
        )
        .await;

        // now send them the character list
        {
            let select_data = ClientSelectData {
                game_name_unk: "Final Fantasy".to_string(),
                current_class: 2,
                class_levels: [5; 30],
                race: CUSTOMIZE_DATA.race as i32,
                subrace: CUSTOMIZE_DATA.subrace as i32,
                gender: CUSTOMIZE_DATA.gender as i32,
                birth_month: NAMEDAY_MONTH as i32,
                birth_day: NAMEDAY_DAY as i32,
                guardian: DEITY as i32,
                unk8: 0,
                unk9: 0,
                zone_id: ZONE_ID as i32,
                unk11: 0,
                customize: CUSTOMIZE_DATA,
                unk12: 0,
                unk13: 0,
                unk14: [0; 10],
                unk15: 0,
                unk16: 0,
                legacy_character: 0,
                unk18: 0,
                unk19: 0,
                unk20: 0,
                unk21: String::new(),
                unk22: 0,
                unk23: 0,
            };

            let mut characters = vec![CharacterDetails {
                id: 0,
                content_id: CONTENT_ID,
                index: 0,
                unk1: [0; 16],
                origin_server_id: WORLD_ID,
                current_server_id: WORLD_ID,
                character_name: CHAR_NAME.to_string(),
                origin_server_name: WORLD_NAME.to_string(),
                current_server_name: WORLD_NAME.to_string(),
                character_detail_json: select_data.to_json(),
                unk2: [0; 20],
            }];

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
                        days_subscribed: 5,
                        remaining_days: 5,
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

    pub async fn send_enter_world(&mut self, sequence: u64, lookup_id: u64) {
        let Some(session_id) = &self.state.session_id else {
            panic!("Missing session id!");
        };

        let enter_world = ServerLobbyIpcData::LobbyEnterWorld {
            sequence,
            character_id: 0,
            content_id: lookup_id, // TODO: shouldn't these be named the same then?
            session_id: session_id.clone(),
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
