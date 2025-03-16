use std::io::Cursor;

use binrw::BinRead;

use crate::{
    CHAR_NAME, CUSTOMIZE_DATA, WORLD_ID,
    ipc::{IPCOpCode, IPCSegment, IPCStructData},
    packet::{PacketSegment, SegmentType},
    timestamp_secs,
    world::PlayerSpawn,
};

use super::{ChatMessage, Position, ZoneConnection};

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
                println!("Spawning actor...");

                // send player spawn
                {
                    let ipc = IPCSegment {
                        unk1: 20,
                        unk2: 0,
                        op_code: IPCOpCode::PlayerSpawn,
                        server_id: 0,
                        timestamp: timestamp_secs(),
                        data: IPCStructData::PlayerSpawn(PlayerSpawn {
                            some_unique_id: 1,
                            content_id: 1,
                            current_world_id: WORLD_ID,
                            home_world_id: WORLD_ID,
                            title: 1,
                            class_job: 35,
                            name: CHAR_NAME.to_string(),
                            hp_curr: 100,
                            hp_max: 100,
                            mp_curr: 100,
                            mp_max: 100,
                            model_type: 1,
                            state: 1,
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
