use crate::{
    common::{CustomizeData, ObjectId, ObjectTypeId, Position, timestamp_secs},
    config::get_config,
    ipc::zone::{
        ActorControl, ActorControlCategory, BattleNpcSubKind, ChatMessage, CommonSpawn,
        DisplayFlag, EventStart, NpcSpawn, ObjectKind, OnlineStatus, PlayerSpawn, PlayerSubKind,
        ServerZoneIpcData, ServerZoneIpcSegment,
    },
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentData, SegmentType},
    world::{Actor, Event},
};

use super::{LuaPlayer, ZoneConnection};

pub const CUSTOMIZE_DATA: CustomizeData = CustomizeData {
    race: 4,
    gender: 1,
    age: 1,
    height: 50,
    subrace: 7,
    face: 3,
    hair: 5,
    enable_highlights: 0,
    skin_tone: 10,
    right_eye_color: 75,
    hair_tone: 50,
    highlights: 0,
    facial_features: 1,
    facial_feature_color: 19,
    eyebrows: 1,
    left_eye_color: 75,
    eyes: 1,
    nose: 0,
    jaw: 1,
    mouth: 1,
    lips_tone_fur_pattern: 169,
    race_feature_size: 100,
    race_feature_type: 1,
    bust: 100,
    face_paint: 0,
    face_paint_color: 167,
};

pub struct ChatHandler {}

impl ChatHandler {
    pub async fn handle_chat_message(
        connection: &mut ZoneConnection,
        lua_player: &mut LuaPlayer,
        chat_message: &ChatMessage,
    ) {
        tracing::info!("Client sent chat message: {}!", chat_message.message);

        let parts: Vec<&str> = chat_message.message.split(' ').collect();
        match parts[0] {
            "!setpos" => {
                let pos_x = parts[1].parse::<f32>().unwrap();
                let pos_y = parts[2].parse::<f32>().unwrap();
                let pos_z = parts[3].parse::<f32>().unwrap();

                connection
                    .set_player_position(Position {
                        x: pos_x,
                        y: pos_y,
                        z: pos_z,
                    })
                    .await;
            }
            "!spawnplayer" => {
                let config = get_config();

                // send player spawn
                {
                    let ipc = ServerZoneIpcSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: ServerZoneIpcType::PlayerSpawn,
                        option: 0,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::PlayerSpawn(PlayerSpawn {
                            account_id: 1000000,
                            content_id: 1000000,
                            current_world_id: config.world.world_id,
                            home_world_id: config.world.world_id,
                            common: CommonSpawn {
                                class_job: 35,
                                name: "Test Actor".to_string(),
                                hp_curr: 250,
                                hp_max: 250,
                                mp_curr: 10000,
                                mp_max: 10000,
                                level: 5,
                                object_kind: ObjectKind::Player(PlayerSubKind::Player),
                                spawn_index: connection.get_free_spawn_index(),
                                look: CUSTOMIZE_DATA,
                                display_flags: DisplayFlag::INVISIBLE
                                    | DisplayFlag::HIDE_HEAD
                                    | DisplayFlag::UNK,
                                models: [
                                    0,  // head
                                    89, // body
                                    89, // hands
                                    89, // legs
                                    89, // feet
                                    0,  // ears
                                    0,  // neck
                                    0,  // wrists
                                    0,  // left finger
                                    0,  // right finger
                                ],
                                pos: connection.player_data.position,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc,
                            data: SegmentData::Ipc { data: ipc },
                        })
                        .await;
                }

                // zone in
                {
                    let ipc = ServerZoneIpcSegment {
                        op_code: ServerZoneIpcType::ActorControl,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::ActorControl(ActorControl {
                            category: ActorControlCategory::ZoneIn {
                                warp_finish_anim: 0x0,
                                raise_anim: 0x0,
                            },
                        }),
                        ..Default::default()
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc,
                            data: SegmentData::Ipc { data: ipc },
                        })
                        .await;
                }
            }
            "!spawnnpc" => {
                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::NpcSpawn,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                        common: CommonSpawn {
                            hp_curr: 100,
                            hp_max: 100,
                            mp_curr: 100,
                            mp_max: 100,
                            look: CUSTOMIZE_DATA,
                            spawn_index: connection.get_free_spawn_index(),
                            bnpc_base: 13498,
                            bnpc_name: 10261,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            target_id: ObjectTypeId {
                                object_id: ObjectId(connection.player_data.actor_id),
                                object_type: 0,
                            }, // target the player
                            level: 1,
                            models: [
                                0,  // head
                                89, // body
                                89, // hands
                                89, // legs
                                89, // feet
                                0,  // ears
                                0,  // neck
                                0,  // wrists
                                0,  // left finger
                                0,  // right finger
                            ],
                            pos: connection.player_data.position,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                connection
                    .send_segment(PacketSegment {
                        source_actor: 0x106ad804,
                        target_actor: connection.player_data.actor_id,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc { data: ipc },
                    })
                    .await;
            }
            "!spawnmonster" => {
                let spawn_index = connection.get_free_spawn_index();

                // spawn a tiny mandragora
                {
                    let ipc = ServerZoneIpcSegment {
                        op_code: ServerZoneIpcType::NpcSpawn,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                            aggression_mode: 1,
                            common: CommonSpawn {
                                hp_curr: 91,
                                hp_max: 91,
                                mp_curr: 100,
                                mp_max: 100,
                                spawn_index,
                                bnpc_base: 13498, // TODO: changing this prevents it from spawning...
                                bnpc_name: 405,
                                object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                                level: 1,
                                battalion: 4,
                                model_chara: 297,
                                pos: connection.player_data.position,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                        ..Default::default()
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc,
                            data: SegmentData::Ipc { data: ipc },
                        })
                        .await;
                }

                connection.actors.push(Actor {
                    id: ObjectId(0x106ad804),
                    hp: 91,
                    spawn_index: spawn_index as u32,
                });
            }
            "!playscene" => {
                let parts: Vec<&str> = chat_message.message.split(' ').collect();
                let event_id = parts[1].parse::<u32>().unwrap();

                // Load the game script for this event on the client
                {
                    let ipc = ServerZoneIpcSegment {
                        op_code: ServerZoneIpcType::EventStart,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::EventStart(EventStart {
                            target_id: ObjectTypeId {
                                object_id: ObjectId(connection.player_data.actor_id),
                                object_type: 0,
                            },
                            event_type: 15,
                            event_id,
                            flags: 0,
                            event_arg: 182, // zone?
                        }),
                        ..Default::default()
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: connection.player_data.actor_id,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc,
                            data: SegmentData::Ipc { data: ipc },
                        })
                        .await;
                }

                // set our status icon to viewing cutscene
                {
                    let ipc = ServerZoneIpcSegment {
                        op_code: ServerZoneIpcType::ActorControl,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::ActorControl(ActorControl {
                            category: ActorControlCategory::SetStatusIcon {
                                icon: OnlineStatus::ViewingCutscene,
                            },
                        }),
                        ..Default::default()
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: connection.player_data.actor_id,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc,
                            data: SegmentData::Ipc { data: ipc },
                        })
                        .await;
                }

                let event = match event_id {
                    1245185 => Event::new("opening/OpeningLimsaLominsa.lua"),
                    1245186 => Event::new("opening/OpeningGridania.lua"),
                    1245187 => Event::new("opening/OpeningUldah.lua"),
                    _ => panic!("Unsupported event!"),
                };

                connection.event = Some(event);
                connection
                    .event
                    .as_mut()
                    .unwrap()
                    .enter_territory(lua_player, connection.zone.as_ref().unwrap());
            }
            "!spawnclone" => {
                // spawn another one of us
                let player = &connection.player_data;

                let mut common = connection
                    .get_player_common_spawn(Some(player.position), Some(player.rotation));
                common.spawn_index = connection.get_free_spawn_index();

                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::NpcSpawn,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                        common,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                connection
                    .send_segment(PacketSegment {
                        source_actor: 0x106ad804,
                        target_actor: connection.player_data.actor_id,
                        segment_type: SegmentType::Ipc,
                        data: SegmentData::Ipc { data: ipc },
                    })
                    .await;
            }
            "!classjob" => {
                let parts: Vec<&str> = chat_message.message.split(' ').collect();

                connection.player_data.classjob_id = parts[1].parse::<u8>().unwrap();
                connection.update_class_info().await;
            }
            _ => tracing::info!("Unrecognized debug command!"),
        }
    }
}
