use crate::{
    common::{CustomizeData, INVALID_OBJECT_ID, ObjectId, ObjectTypeId, Position, timestamp_secs},
    config::get_config,
    packet::{PacketSegment, SegmentType},
    world::ipc::{
        ActorControl, ActorControlCategory, CommonSpawn, NpcSpawn, ObjectKind, PlayerSpawn,
        ServerZoneIpcData, ServerZoneIpcSegment, ServerZoneIpcType, StatusEffectList,
    },
};

use super::{ZoneConnection, ipc::ChatMessage};

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
    pub async fn handle_chat_message(connection: &mut ZoneConnection, chat_message: &ChatMessage) {
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
            "!spawnactor" => {
                tracing::info!("Spawning actor...");

                // status effect
                {
                    let ipc = ServerZoneIpcSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: ServerZoneIpcType::StatusEffectList,
                        server_id: 0,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::StatusEffectList(StatusEffectList {
                            classjob_id: 3,
                            level: 10,
                            unk1: 10,
                            curr_hp: 241,
                            max_hp: 241,
                            curr_mp: 10000,
                            max_mp: 10000,
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc { data: ipc },
                        })
                        .await;
                }

                let config = get_config();

                // send player spawn
                {
                    let ipc = ServerZoneIpcSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: ServerZoneIpcType::PlayerSpawn,
                        server_id: 0,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::PlayerSpawn(PlayerSpawn {
                            some_unique_id: 1,
                            content_id: 1,
                            common: CommonSpawn {
                                current_world_id: config.world.world_id,
                                home_world_id: config.world.world_id,
                                title: 1,
                                class_job: 35,
                                name: "Test Actor".to_string(),
                                hp_curr: 100,
                                hp_max: 100,
                                mp_curr: 100,
                                mp_max: 100,
                                object_kind: ObjectKind::Player,
                                gm_rank: 3,
                                spawn_index: connection.get_free_spawn_index(),
                                look: CUSTOMIZE_DATA,
                                fc_tag: "LOCAL".to_string(),
                                subtype: 4,
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
                                pos: connection.position,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc { data: ipc },
                        })
                        .await;
                }

                // zone in
                {
                    let ipc = ServerZoneIpcSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: ServerZoneIpcType::ActorControl,
                        server_id: 0,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::ActorControl(ActorControl {
                            category: ActorControlCategory::ZoneIn,
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc { data: ipc },
                        })
                        .await;
                }
            }
            "!spawnnpc" => {
                // spawn another one of us
                {
                    let ipc = ServerZoneIpcSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: ServerZoneIpcType::NpcSpawn,
                        server_id: 0,
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
                                object_kind: ObjectKind::BattleNpc,
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
                                pos: connection.position,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc { data: ipc },
                        })
                        .await;
                }
            }
            "!spawnmonster" => {
                // spawn a tiny mandragora
                {
                    let ipc = ServerZoneIpcSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: ServerZoneIpcType::NpcSpawn,
                        server_id: 0,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                            common: CommonSpawn {
                                hp_curr: 91,
                                hp_max: 91,
                                mp_curr: 100,
                                mp_max: 100,
                                spawn_index: connection.get_free_spawn_index(),
                                bnpc_base: 13498, // TODO: changing this prevents it from spawning...
                                bnpc_name: 405,
                                object_kind: ObjectKind::BattleNpc,
                                level: 1,
                                battalion: 4,
                                model_chara: 297,
                                pos: connection.position,
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_data.actor_id,
                            segment_type: SegmentType::Ipc { data: ipc },
                        })
                        .await;
                }
            }
            _ => tracing::info!("Unrecognized debug command!"),
        }
    }
}
