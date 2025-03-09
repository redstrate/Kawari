use std::{
    cmp::min,
    fs::write,
    io::Cursor,
    time::{SystemTime, UNIX_EPOCH},
};

use binrw::{BinRead, BinWrite, binrw};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
};

use crate::{
    common::{read_bool_from, read_string, write_bool_as},
    encryption::{blowfish_encode, decrypt, encrypt, generate_encryption_key},
    ipc::{CharacterDetails, IPCOpCode, IPCSegment, IPCStructData, Server, ServiceAccount},
};

#[binrw]
#[brw(repr = u16)]
#[derive(Debug)]
enum ConnectionType {
    None = 0x0,
    Zone = 0x1,
    Chat = 0x2,
    Lobby = 0x3,
}

#[binrw]
#[brw(import(size: u32, encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
enum SegmentType {
    // Client->Server Packets
    #[brw(magic = 0x9u32)]
    InitializeEncryption {
        #[brw(pad_before = 36)] // empty
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        phrase: String,

        #[brw(pad_after = 512)] // empty
        key: [u8; 4],
    },
    #[brw(magic = 0x3u32)]
    Ipc {
        #[br(parse_with = decrypt, args(size, encryption_key))]
        #[bw(write_with = encrypt, args(size, encryption_key))]
        data: IPCSegment,
    },
    #[brw(magic = 0x7u32)]
    KeepAlive { id: u32, timestamp: u32 },

    // Server->Client Packets
    #[brw(magic = 0x0Au32)]
    InitializationEncryptionResponse {
        #[br(count = 0x280)]
        data: Vec<u8>,
    },
    #[brw(magic = 0x08u32)]
    KeepAliveResponse { id: u32, timestamp: u32 },
}

#[binrw]
#[derive(Debug)]
struct PacketHeader {
    unk1: u64,
    unk2: u64,
    timestamp: u64,
    size: u32,
    connection_type: ConnectionType,
    segment_count: u16,
    unk3: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    compressed: bool,
    unk4: u16,
    unk5: u32, // iolite says the size after oodle decompression
}

#[binrw]
#[brw(import(encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
struct PacketSegment {
    #[bw(calc = self.calc_size())]
    size: u32,
    source_actor: u32,
    target_actor: u32,
    #[brw(args(size, encryption_key))]
    segment_type: SegmentType,
}

impl PacketSegment {
    fn calc_size(&self) -> u32 {
        let header = std::mem::size_of::<u32>() * 4;
        header as u32
            + match &self.segment_type {
                SegmentType::InitializeEncryption { .. } => 616,
                SegmentType::InitializationEncryptionResponse { .. } => 640,
                SegmentType::Ipc { data } => data.calc_size(),
                SegmentType::KeepAlive { .. } => todo!(),
                SegmentType::KeepAliveResponse { .. } => 0x8,
            }
    }
}

#[binrw]
#[brw(import(encryption_key: Option<&[u8]>))]
#[derive(Debug)]
struct Packet {
    header: PacketHeader,
    #[br(count = header.segment_count, args { inner: (encryption_key,) })]
    #[bw(args(encryption_key))]
    segments: Vec<PacketSegment>,
}

fn dump(msg: &str, data: &[u8]) {
    write("packet.bin", data).unwrap();
    panic!("{msg} Dumped to packet.bin.");
}

async fn send_packet(socket: &mut WriteHalf<TcpStream>, segments: &[PacketSegment], state: &State) {
    let timestamp: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_millis()
        .try_into()
        .unwrap();

    let mut total_segment_size = 0;
    for segment in segments {
        total_segment_size += segment.calc_size();
    }

    let header = PacketHeader {
        unk1: 0xE2465DFF41A05252, // wtf?
        unk2: 0x75C4997B4D642A7F, // wtf? x2
        timestamp,
        size: std::mem::size_of::<PacketHeader>() as u32 + total_segment_size,
        connection_type: ConnectionType::Lobby,
        segment_count: segments.len() as u16,
        unk3: 0,
        compressed: false,
        unk4: 0,
        unk5: 0,
    };

    let packet = Packet {
        header,
        segments: segments.to_vec(),
    };

    let mut cursor = Cursor::new(Vec::new());
    packet
        .write_le_args(
            &mut cursor,
            (state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),),
        )
        .unwrap();

    let buffer = cursor.into_inner();

    tracing::info!("Wrote response packet to outpacket.bin");
    write("outpacket.bin", &buffer).unwrap();

    socket
        .write_all(&buffer)
        .await
        .expect("Failed to write packet!");
}

// temporary
pub struct State {
    pub client_key: Option<[u8; 16]>,
}

pub async fn parse_packet(socket: &mut WriteHalf<TcpStream>, data: &[u8], state: &mut State) {
    let mut cursor = Cursor::new(data);

    match Packet::read_le_args(
        &mut cursor,
        (state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),),
    ) {
        Ok(packet) => {
            println!("{:#?}", packet);

            if packet.header.size as usize != data.len() {
                dump(
                    "Packet size mismatch between what we're given and the header!",
                    data,
                );
            }

            for segment in &packet.segments {
                match &segment.segment_type {
                    SegmentType::InitializeEncryption { phrase, key } => {
                        // Generate an encryption key for this client
                        state.client_key = Some(generate_encryption_key(key, phrase));

                        let mut data = 0xE0003C2Au32.to_le_bytes().to_vec();
                        data.resize(0x280, 0);

                        unsafe {
                            let result = blowfish_encode(
                                state.client_key.unwrap().as_ptr(),
                                16,
                                data.as_ptr(),
                                0x280,
                            );
                            data = std::slice::from_raw_parts(result, 0x280).to_vec();
                        }

                        let response_packet = PacketSegment {
                            source_actor: 0,
                            target_actor: 0,
                            segment_type: SegmentType::InitializationEncryptionResponse { data },
                        };
                        send_packet(socket, &[response_packet], state).await;
                    }
                    SegmentType::Ipc { data } => {
                        match &data.data {
                            IPCStructData::ClientVersionInfo {
                                session_id,
                                version_info,
                            } => {
                                tracing::info!("Client {session_id} ({version_info}) logging in!");

                                let timestamp: u32 = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("Failed to get UNIX timestamp!")
                                    .as_secs()
                                    .try_into()
                                    .unwrap();

                                // send the client the service account list
                                let mut service_accounts = [ServiceAccount {
                                    id: 0x002E4A2B,
                                    unk1: 0,
                                    index: 0,
                                    name: "FINAL FANTASY XIV".to_string(),
                                }]
                                .to_vec();
                                // add any empty boys
                                service_accounts.resize(8, ServiceAccount::default());

                                let service_account_list = IPCStructData::LobbyServiceAccountList {
                                    sequence: 0,
                                    num_service_accounts: service_accounts.len() as u8,
                                    unk1: 3,
                                    unk2: 0x99,
                                    service_accounts: service_accounts.to_vec(),
                                };

                                let ipc = IPCSegment {
                                    unk1: 0,
                                    unk2: 0,
                                    op_code: IPCOpCode::LobbyServiceAccountList,
                                    server_id: 0,
                                    timestamp,
                                    data: service_account_list,
                                };

                                let response_packet = PacketSegment {
                                    source_actor: 0,
                                    target_actor: 0,
                                    segment_type: SegmentType::Ipc { data: ipc },
                                };
                                send_packet(socket, &[response_packet], state).await;
                            }
                            IPCStructData::RequestCharacterList { sequence } => {
                                tracing::info!("Client is requesting character list...");

                                let timestamp: u32 = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("Failed to get UNIX timestamp!")
                                    .as_secs()
                                    .try_into()
                                    .unwrap();

                                let mut packets = Vec::new();
                                // send them the character list
                                {
                                    let mut servers = [Server {
                                        id: 21,
                                        index: 0,
                                        flags: 0,
                                        icon: 0,
                                        name: "KAWARI".to_string(),
                                    }]
                                    .to_vec();
                                    // add any empty boys
                                    servers.resize(6, Server::default());

                                    let lobby_server_list = IPCStructData::LobbyServerList {
                                        sequence: 0,
                                        unk1: 1,
                                        offset: 0,
                                        num_servers: 1,
                                        servers,
                                    };

                                    let ipc = IPCSegment {
                                        unk1: 0,
                                        unk2: 0,
                                        op_code: IPCOpCode::LobbyServerList,
                                        server_id: 0,
                                        timestamp,
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
                                    let lobby_retainer_list =
                                        IPCStructData::LobbyRetainerList { unk1: 1 };

                                    let ipc = IPCSegment {
                                        unk1: 0,
                                        unk2: 0,
                                        op_code: IPCOpCode::LobbyRetainerList,
                                        server_id: 0,
                                        timestamp,
                                        data: lobby_retainer_list,
                                    };

                                    let response_packet = PacketSegment {
                                        source_actor: 0,
                                        target_actor: 0,
                                        segment_type: SegmentType::Ipc { data: ipc },
                                    };
                                    packets.push(response_packet);
                                }

                                send_packet(socket, &packets, state).await;

                                // now send them the character list
                                {
                                    let mut characters = vec![CharacterDetails {
                                        id: 0,
                                        content_id: 11111111111111111,
                                        index: 0,
                                        server_id: 21,
                                        server_id1: 21,
                                        unk1: [0; 16],
                                        character_name: "test".to_string(),
                                        character_server_name: "test".to_string(),
                                        character_server_name1: "test".to_string(),
                                        character_detail_json: "test".to_string(),
                                        unk2: [0; 20],
                                    }];
                                    // add any empty boys
                                    characters.resize(2, CharacterDetails::default());

                                    for i in 0..4 {
                                        let mut characters_in_packet = Vec::new();
                                        for _ in 0..min(characters.len(), 2) {
                                            characters_in_packet.push(characters.swap_remove(0));
                                        }

                                        let lobby_character_list = if i == 3 {
                                            // On the last packet, add the account-wide information
                                            IPCStructData::LobbyCharacterList {
                                                sequence: *sequence,
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
                                                days_subscribed: 0,
                                                remaining_days: 0,
                                                days_to_next_rank: 0,
                                                max_characters_on_world: 20,
                                                unk8: 8,
                                                entitled_expansion: 4,
                                                characters: characters_in_packet,
                                            }
                                        } else {
                                            IPCStructData::LobbyCharacterList {
                                                sequence: *sequence,
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

                                        let ipc = IPCSegment {
                                            unk1: 0,
                                            unk2: 0,
                                            op_code: IPCOpCode::LobbyCharacterList,
                                            server_id: 0,
                                            timestamp,
                                            data: lobby_character_list,
                                        };

                                        let response_packet = PacketSegment {
                                            source_actor: 0,
                                            target_actor: 0,
                                            segment_type: SegmentType::Ipc { data: ipc },
                                        };
                                        send_packet(socket, &[response_packet], state).await;
                                    }
                                }
                            }
                            _ => {
                                panic!("The server is recieving a IPC response packet!")
                            }
                        }
                    }
                    SegmentType::KeepAlive { id, timestamp } => {
                        let response_packet = PacketSegment {
                            source_actor: 0,
                            target_actor: 0,
                            segment_type: SegmentType::KeepAliveResponse {
                                id: *id,
                                timestamp: *timestamp,
                            },
                        };
                        send_packet(socket, &[response_packet], state).await;
                    }
                    _ => {
                        panic!("The server is recieving a response packet!")
                    }
                }
            }
        }
        Err(err) => {
            println!("{err}");
            dump("Failed to parse packet!", data);
        }
    }
}
