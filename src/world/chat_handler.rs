use crate::{
    CHAR_NAME, CUSTOMIZE_DATA, INVALID_OBJECT_ID, WORLD_ID,
    common::timestamp_secs,
    packet::{PacketSegment, SegmentType},
    world::ipc::{
        CommonSpawn, NpcSpawn, ObjectKind, PlayerSpawn, ServerZoneIpcData, ServerZoneIpcSegment,
        ServerZoneIpcType,
    },
};

use super::{
    ZoneConnection,
    ipc::{ChatMessage, Position},
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
                                current_world_id: WORLD_ID,
                                home_world_id: WORLD_ID,
                                title: 1,
                                class_job: 35,
                                name: CHAR_NAME.to_string(),
                                hp_curr: 100,
                                hp_max: 100,
                                mp_curr: 100,
                                mp_max: 100,
                                object_kind: ObjectKind::Player,
                                gm_rank: 3,
                                spawn_index: connection.get_free_spawn_index(),
                                look: CUSTOMIZE_DATA,
                                fc_tag: "LOCAL".to_string(),
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
                                pos: Position::default(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_id,
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
                                spawner_id: connection.player_id,
                                parent_actor_id: INVALID_OBJECT_ID, // TODO: make default?
                                object_kind: ObjectKind::BattleNpc,
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
                                pos: Position::default(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }),
                    };

                    connection
                        .send_segment(PacketSegment {
                            source_actor: 0x106ad804,
                            target_actor: connection.player_id,
                            segment_type: SegmentType::Ipc { data: ipc },
                        })
                        .await;
                }
            }
            _ => tracing::info!("Unrecognized debug command!"),
        }
    }
}
