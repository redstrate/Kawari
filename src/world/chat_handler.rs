use crate::{
    common::{ObjectId, ObjectTypeId, timestamp_secs},
    inventory::Storage,
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ChatMessage, EventStart, NpcSpawn,
        OnlineStatus, ServerZoneIpcData, ServerZoneIpcSegment,
    },
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentData, SegmentType},
    world::{Event, ToServer},
};

use super::{LuaPlayer, ZoneConnection};

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
            "!spawnnpc" => {
                connection
                    .handle
                    .send(ToServer::DebugNewNpc(
                        connection.id,
                        connection.player_data.actor_id,
                    ))
                    .await;
            }
            "!spawnmonster" => {
                connection
                    .handle
                    .send(ToServer::DebugNewEnemy(
                        connection.id,
                        connection.player_data.actor_id,
                    ))
                    .await;
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
                    1245185 => Event::new(1245185, "opening/OpeningLimsaLominsa.lua"),
                    1245186 => Event::new(1245186, "opening/OpeningGridania.lua"),
                    1245187 => Event::new(1245187, "opening/OpeningUldah.lua"),
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
            "!unlockaction" => {
                let parts: Vec<&str> = chat_message.message.split(' ').collect();
                let id = parts[1].parse::<u32>().unwrap();

                connection
                    .actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::ToggleActionUnlock { id, unlocked: true },
                    })
                    .await;
            }
            "!equip" => {
                let (_, name) = chat_message.message.split_once(' ').unwrap();

                {
                    let mut gamedata = connection.gamedata.lock().unwrap();

                    if let Some((equip_category, id)) = gamedata.get_item_by_name(name) {
                        let slot = gamedata.get_equipslot_category(equip_category).unwrap();

                        connection
                            .player_data
                            .inventory
                            .equipped
                            .get_slot_mut(slot as u16)
                            .id = id;
                        connection
                            .player_data
                            .inventory
                            .equipped
                            .get_slot_mut(slot as u16)
                            .quantity = 1;
                    }
                }

                connection.send_inventory(true).await;
            }
            _ => {}
        }
    }
}
