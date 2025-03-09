use std::cmp::min;
use std::time::{SystemTime, UNIX_EPOCH};

use kawari::encryption::{blowfish_encode, generate_encryption_key};
use kawari::ipc::{CharacterDetails, IPCOpCode, IPCSegment, IPCStructData, Server, ServiceAccount};
use kawari::packet::{
    PacketSegment, SegmentType, State, parse_packet, send_keep_alive, send_packet,
};
use tokio::io::{AsyncReadExt, WriteHalf};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();

    tracing::info!("Lobby server started on 7000");

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let (mut read, mut write) = tokio::io::split(socket);

        let mut state = State { client_key: None };

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = read.read(&mut buf).await.expect("Failed to read data!");

                if n != 0 {
                    let segments = parse_packet(&buf[..n], &mut state).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::InitializeEncryption { phrase, key } => {
                                initialize_encryption(&mut write, &mut state, phrase, key).await;
                            }
                            SegmentType::Ipc { data } => match &data.data {
                                IPCStructData::ClientVersionInfo {
                                    session_id,
                                    version_info,
                                } => {
                                    tracing::info!(
                                        "Client {session_id} ({version_info}) logging in!"
                                    );

                                    send_account_list(&mut write, &state).await;
                                }
                                IPCStructData::RequestCharacterList { sequence } => {
                                    tracing::info!("Client is requesting character list...");

                                    send_lobby_info(&mut write, &state, *sequence).await;
                                }
                                _ => {
                                    panic!("The server is recieving a IPC response packet!")
                                }
                            },
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive(&mut write, &state, *id, *timestamp).await
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

async fn initialize_encryption(
    socket: &mut WriteHalf<TcpStream>,
    state: &mut State,
    phrase: &str,
    key: &[u8; 4],
) {
    // Generate an encryption key for this client
    state.client_key = Some(generate_encryption_key(key, phrase));

    let mut data = 0xE0003C2Au32.to_le_bytes().to_vec();
    data.resize(0x280, 0);

    unsafe {
        let result = blowfish_encode(state.client_key.unwrap().as_ptr(), 16, data.as_ptr(), 0x280);
        data = std::slice::from_raw_parts(result, 0x280).to_vec();
    }

    let response_packet = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::InitializationEncryptionResponse { data },
    };
    send_packet(socket, &[response_packet], state).await;
}

async fn send_account_list(socket: &mut WriteHalf<TcpStream>, state: &State) {
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

async fn send_lobby_info(socket: &mut WriteHalf<TcpStream>, state: &State, sequence: u64) {
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
        let lobby_retainer_list = IPCStructData::LobbyRetainerList { unk1: 1 };

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
