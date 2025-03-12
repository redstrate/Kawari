use std::time::{SystemTime, UNIX_EPOCH};

use kawari::ipc::{ActorControlType, IPCOpCode, IPCSegment, IPCStructData, Position};
use kawari::oodle::FFXIVOodle;
use kawari::packet::{
    CompressionType, PacketSegment, SegmentType, State, parse_packet, send_keep_alive, send_packet,
};
use kawari::{CONTENT_ID, WORLD_ID, ZONE_ID};
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:7100").await.unwrap();

    tracing::info!("World server started on 7100");

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
                    println!("recieved {n} bytes...");
                    let (segments, connection_type) = parse_packet(&buf[..n], &mut state).await;
                    for segment in &segments {
                        match &segment.segment_type {
                            SegmentType::InitializeSession { player_id } => {
                                state.player_id = Some(*player_id);

                                // We have send THEM a keep alive
                                {
                                    let timestamp: u32 = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .expect("Failed to get UNIX timestamp!")
                                        .as_secs()
                                        .try_into()
                                        .unwrap();

                                    let response_packet = PacketSegment {
                                        source_actor: 0,
                                        target_actor: 0,
                                        segment_type: SegmentType::KeepAlive {
                                            id: 0xE0037603u32,
                                            timestamp,
                                        },
                                    };
                                    send_packet(
                                        &mut write,
                                        &[response_packet],
                                        &mut state,
                                        CompressionType::Oodle,
                                    )
                                    .await;
                                }

                                match connection_type {
                                    kawari::packet::ConnectionType::Zone => {
                                        tracing::info!(
                                            "Client {player_id} is initializing zone session..."
                                        );

                                        let response_packet = PacketSegment {
                                            source_actor: 0,
                                            target_actor: 0,
                                            segment_type: SegmentType::ZoneInitialize {
                                                player_id: *player_id,
                                            },
                                        };
                                        send_packet(
                                            &mut write,
                                            &[response_packet],
                                            &mut state,
                                            CompressionType::Oodle,
                                        )
                                        .await;
                                    }
                                    kawari::packet::ConnectionType::Chat => {
                                        tracing::info!(
                                            "Client {player_id} is initializing chat session..."
                                        );

                                        {
                                            let response_packet = PacketSegment {
                                                source_actor: 0,
                                                target_actor: 0,
                                                segment_type: SegmentType::ZoneInitialize {
                                                    player_id: *player_id,
                                                },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitializeChat,
                                                server_id: 0,
                                                timestamp: 0,
                                                data: IPCStructData::InitializeChat { unk: [0; 8] },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: *player_id,
                                                target_actor: *player_id,
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }
                                    }
                                    _ => panic!(
                                        "The client is trying to initialize the wrong connection?!"
                                    ),
                                }
                            }
                            SegmentType::Ipc { data } => {
                                match &data.data {
                                    IPCStructData::InitRequest { .. } => {
                                        tracing::info!(
                                            "Client is now requesting zone information. Sending!"
                                        );

                                        let timestamp_secs = || {
                                            SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .expect("Failed to get UNIX timestamp!")
                                                .as_secs()
                                                .try_into()
                                                .unwrap()
                                        };

                                        // IPC Init(?)
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitResponse,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitResponse {
                                                    unk1: 0,
                                                    character_id: state.player_id.unwrap(),
                                                    unk2: 0,
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Control Data
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::ActorControlSelf,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::ActorControlSelf {
                                                    category: ActorControlType::SetCharaGearParamUI,
                                                    param1: 1,
                                                    param2: 1,
                                                    param3: 0,
                                                    param4: 0,
                                                    param5: 0,
                                                    param6: 0,
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Stats
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::PlayerStats,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerStats {
                                                    strength: 1,
                                                    dexterity: 0,
                                                    vitality: 0,
                                                    intelligence: 0,
                                                    mind: 0,
                                                    piety: 0,
                                                    hp: 100,
                                                    mp: 100,
                                                    tp: 0,
                                                    gp: 0,
                                                    cp: 0,
                                                    delay: 0,
                                                    tenacity: 0,
                                                    attack_power: 0,
                                                    defense: 0,
                                                    direct_hit_rate: 0,
                                                    evasion: 0,
                                                    magic_defense: 0,
                                                    critical_hit: 0,
                                                    attack_magic_potency: 0,
                                                    healing_magic_potency: 0,
                                                    elemental_bonus: 0,
                                                    determination: 0,
                                                    skill_speed: 0,
                                                    spell_speed: 0,
                                                    haste: 0,
                                                    craftmanship: 0,
                                                    control: 0,
                                                    gathering: 0,
                                                    perception: 0,
                                                    unk1: [0; 26],
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Player Setup
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::PlayerSetup,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::PlayerSetup {
                                                    content_id: CONTENT_ID,
                                                    crest: 0,
                                                    unknown10: 0,
                                                    char_id: 0,
                                                    rested_exp: 0,
                                                    companion_current_exp: 0,
                                                    unknown1c: 0,
                                                    fish_caught: 0,
                                                    use_bait_catalog_id: 0,
                                                    unknown28: 0,
                                                    unknown_pvp2c: 0,
                                                    unknown2e: 0,
                                                    pvp_frontline_overall_campaigns: 0,
                                                    unknown_timestamp34: 0,
                                                    unknown_timestamp38: 0,
                                                    unknown3c: 0,
                                                    unknown40: 0,
                                                    unknown44: 0,
                                                    companion_time_passed: 0.0,
                                                    unknown4c: 0,
                                                    unknown50: 0,
                                                    unknown_pvp52: [0; 4],
                                                    pvp_series_exp: 0,
                                                    player_commendations: 0,
                                                    unknown64: [0; 8],
                                                    pvp_rival_wings_total_matches: 0,
                                                    pvp_rival_wings_total_victories: 0,
                                                    pvp_rival_wings_weekly_matches: 0,
                                                    pvp_rival_wings_weekly_victories: 0,
                                                    max_level: 0,
                                                    expansion: 0,
                                                    unknown76: 0,
                                                    unknown77: 0,
                                                    unknown78: 0,
                                                    race: 0,
                                                    tribe: 0,
                                                    gender: 0,
                                                    current_job: 0,
                                                    current_class: 0,
                                                    deity: 0,
                                                    nameday_month: 0,
                                                    nameday_day: 0,
                                                    city_state: 0,
                                                    homepoint: 0,
                                                    unknown8d: [0; 3],
                                                    companion_rank: 0,
                                                    companion_stars: 0,
                                                    companion_sp: 0,
                                                    companion_unk93: 0,
                                                    companion_color: 0,
                                                    companion_fav_feed: 0,
                                                    fav_aetheryte_count: 0,
                                                    unknown97: [0; 5],
                                                    sightseeing21_to_80_unlock: 0,
                                                    sightseeing_heavensward_unlock: 0,
                                                    unknown9e: [0; 26],
                                                    exp: [10000; 32],
                                                    pvp_total_exp: 0,
                                                    unknown_pvp124: 0,
                                                    pvp_exp: 0,
                                                    pvp_frontline_overall_ranks: [0; 3],
                                                    unknown138: 0,
                                                    levels: [100; 32],
                                                    unknown194: [0; 218],
                                                    companion_name: [0; 21],
                                                    companion_def_rank: 0,
                                                    companion_att_rank: 0,
                                                    companion_heal_rank: 0,
                                                    mount_guide_mask: [0; 33],
                                                    ornament_mask: [0; 4],
                                                    unknown281: [0; 23],
                                                    name: "KAWARI".to_string(),
                                                    unknown293: [0; 16],
                                                    unknown2a3: 0,
                                                    unlock_bitmask: [0; 64],
                                                    aetheryte: [0; 26],
                                                    favorite_aetheryte_ids: [0; 4],
                                                    free_aetheryte_id: 0,
                                                    ps_plus_free_aetheryte_id: 0,
                                                    discovery: [0; 480],
                                                    howto: [0; 36],
                                                    unknown554: [0; 4],
                                                    minions: [0; 60],
                                                    chocobo_taxi_mask: [0; 12],
                                                    watched_cutscenes: [0; 159],
                                                    companion_barding_mask: [0; 12],
                                                    companion_equipped_head: 0,
                                                    companion_equipped_body: 0,
                                                    companion_equipped_legs: 0,
                                                    unknown_mask: [0; 287],
                                                    pose: [0; 7],
                                                    unknown6df: [0; 3],
                                                    challenge_log_complete: [0; 13],
                                                    secret_recipe_book_mask: [0; 12],
                                                    unknown_mask6f7: [0; 29],
                                                    relic_completion: [0; 12],
                                                    sightseeing_mask: [0; 37],
                                                    hunting_mark_mask: [0; 102],
                                                    triple_triad_cards: [0; 45],
                                                    unknown895: 0,
                                                    unknown7d7: [0; 15],
                                                    unknown7d8: 0,
                                                    unknown7e6: [0; 49],
                                                    regional_folklore_mask: [0; 6],
                                                    orchestrion_mask: [0; 87],
                                                    hall_of_novice_completion: [0; 3],
                                                    anima_completion: [0; 11],
                                                    unknown85e: [0; 41],
                                                    unlocked_raids: [0; 28],
                                                    unlocked_dungeons: [0; 18],
                                                    unlocked_guildhests: [0; 10],
                                                    unlocked_trials: [0; 12],
                                                    unlocked_pvp: [0; 5],
                                                    cleared_raids: [0; 28],
                                                    cleared_dungeons: [0; 18],
                                                    cleared_guildhests: [0; 10],
                                                    cleared_trials: [0; 12],
                                                    cleared_pvp: [0; 5],
                                                    unknown948: [0; 15],
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Player Class Info
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::UpdateClassInfo,
                                                server_id: 69, // lol
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::UpdateClassInfo {
                                                    class_id: 35,
                                                    unknown: 1,
                                                    is_specialist: 0,
                                                    synced_level: 90,
                                                    class_level: 90,
                                                    role_actions: [0; 10],
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // Init Zone
                                        {
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitZone,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitZone {
                                                    server_id: WORLD_ID,
                                                    zone_id: ZONE_ID,
                                                    zone_index: 0,
                                                    content_finder_condition_id: 0,
                                                    layer_set_id: 0,
                                                    layout_id: 0,
                                                    weather_id: 1,
                                                    unk_bitmask1: 0x10,
                                                    unk_bitmask2: 0,
                                                    unk1: 0,
                                                    unk2: 0,
                                                    festival_id: 0,
                                                    additional_festival_id: 0,
                                                    unk3: 0,
                                                    unk4: 0,
                                                    unk5: 0,
                                                    unk6: [0; 4],
                                                    unk7: [0; 3],
                                                    position: Position {
                                                        x: 0.0,
                                                        y: 0.0,
                                                        z: 0.0,
                                                    },
                                                    unk8: [0; 4],
                                                    unk9: 0,
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(
                                                &mut write,
                                                &[response_packet],
                                                &mut state,
                                                CompressionType::Oodle,
                                            )
                                            .await;
                                        }

                                        // ?????
                                        /*{
                                            let ipc = IPCSegment {
                                                unk1: 0,
                                                unk2: 0,
                                                op_code: IPCOpCode::InitRequest,
                                                server_id: 0,
                                                timestamp: timestamp_secs(),
                                                data: IPCStructData::InitResponse {
                                                    unk1: 0,
                                                    character_id: state.player_id.unwrap(),
                                                    unk2: 0,
                                                },
                                            };

                                            let response_packet = PacketSegment {
                                                source_actor: state.player_id.unwrap(),
                                                target_actor: state.player_id.unwrap(),
                                                segment_type: SegmentType::Ipc { data: ipc },
                                            };
                                            send_packet(&mut write, &[response_packet], &mut state, CompressionType::Oodle)
                                                .await;
                                        }*/
                                    }
                                    _ => panic!(
                                        "The server is recieving a IPC response or unknown packet!"
                                    ),
                                }
                            }
                            SegmentType::KeepAlive { id, timestamp } => {
                                send_keep_alive(&mut write, &mut state, *id, *timestamp).await
                            }
                            SegmentType::KeepAliveResponse { .. } => {
                                tracing::info!("Got keep alive response from client... cool...");
                            }
                            _ => {
                                panic!("The server is recieving a response or unknown packet!")
                            }
                        }
                    }
                }
            }
        });
    }
}
