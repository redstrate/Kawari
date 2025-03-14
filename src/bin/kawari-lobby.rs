use std::cmp::min;
use std::time::{SystemTime, UNIX_EPOCH};

use kawari::blowfish::Blowfish;
use kawari::chara_make::CharaMake;
use kawari::client_select_data::{ClientCustomizeData, ClientSelectData};
use kawari::encryption::generate_encryption_key;
use kawari::ipc::{
    CharacterDetails, IPCOpCode, IPCSegment, IPCStructData, LobbyCharacterAction, Server,
    ServiceAccount,
};
use kawari::oodle::FFXIVOodle;
use kawari::packet::{
    CompressionType, PacketSegment, SegmentType, State, parse_packet, send_keep_alive, send_packet,
};
use kawari::{CONTENT_ID, WORLD_ID, WORLD_NAME, ZONE_ID};
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

        let mut state = State {
            client_key: None,
            session_id: None,
            clientbound_oodle: FFXIVOodle::new(),
            serverbound_oodle: FFXIVOodle::new(),
            player_id: None,
        };

        tokio::spawn(async move {
            let mut buf = [0; 2056];
            loop {
                let n = read.read(&mut buf).await.expect("Failed to read data!");

                if n != 0 {
                    tracing::info!("read {} bytes", n);

                    let (segments, _) = parse_packet(&buf[..n], &mut state).await;
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

                                    state.session_id = Some(session_id.clone());

                                    send_account_list(&mut write, &mut state).await;
                                }
                                IPCStructData::RequestCharacterList { sequence } => {
                                    tracing::info!("Client is requesting character list...");

                                    send_lobby_info(&mut write, &mut state, *sequence).await;
                                }
                                IPCStructData::LobbyCharacterAction {
                                    character_id,
                                    character_index,
                                    action,
                                    world_id,
                                    name,
                                    json,
                                    ..
                                } => {
                                    match action {
                                        LobbyCharacterAction::ReserveName => {
                                            tracing::info!(
                                                "Player is requesting {name} as a new character name!"
                                            );

                                            // reject
                                            /*{
                                                let ipc = IPCSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: IPCOpCode::InitializeChat, // wrong but technically right
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: IPCStructData::NameRejection {
                                                        unk1: 0x03,
                                                        unk2: 0x0bdb,
                                                        unk3: 0x000132cc,
                                                    },
                                                };

                                                let response_packet = PacketSegment {
                                                    source_actor: 0x0,
                                                    target_actor: 0x0,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                };
                                                send_packet(
                                                    &mut write,
                                                    &[response_packet],
                                                    &mut state,
                                                    CompressionType::Uncompressed,
                                                )
                                                .await;
                                            }*/

                                            // accept
                                            {
                                                let ipc = IPCSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: IPCOpCode::CharacterCreated,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: IPCStructData::CharacterCreated {
                                                        unk1: 0x4,
                                                        unk2: 0x00010101,
                                                        details: CharacterDetails {
                                                            content_id: CONTENT_ID,
                                                            character_name: name.clone(),
                                                            origin_server_name: "KAWARI"
                                                                .to_string(),
                                                            current_server_name: "KAWARI"
                                                                .to_string(),
                                                            ..Default::default()
                                                        },
                                                    },
                                                };

                                                let response_packet = PacketSegment {
                                                    source_actor: 0x0,
                                                    target_actor: 0x0,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                };
                                                send_packet(
                                                    &mut write,
                                                    &[response_packet],
                                                    &mut state,
                                                    CompressionType::Uncompressed,
                                                )
                                                .await;
                                            }
                                        }
                                        LobbyCharacterAction::Create => {
                                            tracing::info!("Player is creating a new character!");

                                            let chara_make = CharaMake::from_json(json);
                                            println!("charamake: {:#?}", chara_make);

                                            // a slightly different character created packet now
                                            {
                                                let ipc = IPCSegment {
                                                    unk1: 0,
                                                    unk2: 0,
                                                    op_code: IPCOpCode::CharacterCreated,
                                                    server_id: 0,
                                                    timestamp: 0,
                                                    data: IPCStructData::CharacterCreated {
                                                        unk1: 0x5,
                                                        unk2: 0x00020101,
                                                        details: CharacterDetails {
                                                            id: 0x07369f3a, // notice that we give them an id now
                                                            content_id: CONTENT_ID,
                                                            character_name: name.clone(),
                                                            origin_server_name: "KAWARI"
                                                                .to_string(),
                                                            current_server_name: "KAWARI"
                                                                .to_string(),
                                                            ..Default::default()
                                                        },
                                                    },
                                                };

                                                let response_packet = PacketSegment {
                                                    source_actor: 0x0,
                                                    target_actor: 0x0,
                                                    segment_type: SegmentType::Ipc { data: ipc },
                                                };
                                                send_packet(
                                                    &mut write,
                                                    &[response_packet],
                                                    &mut state,
                                                    CompressionType::Uncompressed,
                                                )
                                                .await;
                                            }
                                        }
                                        LobbyCharacterAction::Rename => todo!(),
                                        LobbyCharacterAction::Delete => todo!(),
                                        LobbyCharacterAction::Move => todo!(),
                                        LobbyCharacterAction::RemakeRetainer => todo!(),
                                        LobbyCharacterAction::RemakeChara => todo!(),
                                        LobbyCharacterAction::SettingsUploadBegin => todo!(),
                                        LobbyCharacterAction::SettingsUpload => todo!(),
                                        LobbyCharacterAction::WorldVisit => todo!(),
                                        LobbyCharacterAction::DataCenterToken => todo!(),
                                        LobbyCharacterAction::Request => todo!(),
                                    }
                                }
                                IPCStructData::RequestEnterWorld {
                                    sequence,
                                    lookup_id,
                                } => {
                                    tracing::info!("Client is joining the world...");

                                    send_enter_world(&mut write, &mut state, *sequence, *lookup_id)
                                        .await;
                                }
                                _ => {
                                    panic!("The server is recieving a IPC response packet!")
                                }
                            },
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive(&mut write, &mut state, *id, *timestamp).await
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

    let blowfish = Blowfish::new(&state.client_key.unwrap());
    blowfish.encrypt(&mut data);

    let response_packet = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::InitializationEncryptionResponse { data },
    };
    send_packet(
        socket,
        &[response_packet],
        state,
        CompressionType::Uncompressed,
    )
    .await;
}

async fn send_account_list(socket: &mut WriteHalf<TcpStream>, state: &mut State) {
    let timestamp: u32 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_secs()
        .try_into()
        .unwrap();

    // send the client the service account list
    let service_accounts = [ServiceAccount {
        id: 0x002E4A2B,
        unk1: 0,
        index: 0,
        name: "FINAL FANTASY XIV".to_string(),
    }]
    .to_vec();

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
    send_packet(
        socket,
        &[response_packet],
        state,
        CompressionType::Uncompressed,
    )
    .await;
}

async fn send_lobby_info(socket: &mut WriteHalf<TcpStream>, state: &mut State, sequence: u64) {
    let timestamp: u32 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_secs()
        .try_into()
        .unwrap();

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

    send_packet(socket, &packets, state, CompressionType::Uncompressed).await;

    // now send them the character list
    {
        let select_data = ClientSelectData {
            game_name_unk: "Final Fantasy".to_string(),
            current_class: 2,
            class_levels: [5; 30],
            race: 0,
            subrace: 0,
            gender: 0,
            birth_month: 5,
            birth_day: 5,
            guardian: 2,
            unk8: 0,
            unk9: 0,
            zone_id: ZONE_ID as i32,
            unk11: 0,
            customize: ClientCustomizeData {
                race: 3,
                gender: 1,
                height: 0,
                subrace: 0,
                face: 1,
                hair: 1,
                enable_highlights: 1,
                skin_tone: 1,
                right_eye_color: 1,
                hair_tone: 1,
                highlights: 1,
                facial_features: 1,
                facial_feature_color: 1,
                eyebrows: 1,
                left_eye_color: 1,
                eyes: 1,
                nose: 1,
                jaw: 1,
                mouth: 1,
                lips_tone_fur_pattern: 1,
                race_feature_size: 1,
                race_feature_type: 1,
                bust: 0,
                face_paint: 1,
                face_paint_color: 0,
            },
            unk12: 0,
            unk13: 0,
            unk14: [0; 10],
            unk15: 0,
            unk16: 0,
            legacy_character: 0,
            unk18: 0,
            unk19: 0,
            unk20: 0,
            unk21: "hello".to_string(),
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
            character_name: "test".to_string(),
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
                    days_subscribed: 5,
                    remaining_days: 5,
                    days_to_next_rank: 0,
                    unk8: 8,
                    max_characters_on_world: 2,
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
            send_packet(
                socket,
                &[response_packet],
                state,
                CompressionType::Uncompressed,
            )
            .await;
        }
    }
}

async fn send_enter_world(
    socket: &mut WriteHalf<TcpStream>,
    state: &mut State,
    sequence: u64,
    lookup_id: u64,
) {
    let Some(session_id) = &state.session_id else {
        panic!("Missing session id!");
    };

    let timestamp: u32 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_secs()
        .try_into()
        .unwrap();

    let enter_world = IPCStructData::LobbyEnterWorld {
        sequence,
        character_id: 0,
        content_id: lookup_id, // TODO: shouldn't these be named the same then?
        session_id: session_id.clone(),
        port: 7100,
        host: "127.0.0.1".to_string(),
    };

    let ipc = IPCSegment {
        unk1: 0,
        unk2: 0,
        op_code: IPCOpCode::LobbyEnterWorld,
        server_id: 0,
        timestamp,
        data: enter_world,
    };

    let response_packet = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::Ipc { data: ipc },
    };
    send_packet(
        socket,
        &[response_packet],
        state,
        CompressionType::Uncompressed,
    )
    .await;
}
