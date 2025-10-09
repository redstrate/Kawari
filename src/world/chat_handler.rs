use std::str::FromStr;

use crate::{
    ERR_INVENTORY_ADD_FAILED, ITEM_CONDITION_MAX,
    common::ItemInfoQuery,
    inventory::{Item, Storage},
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, ServerZoneIpcData, ServerZoneIpcSegment,
        client::SendChatMessage,
        server::{Condition, Conditions, GameMasterRank},
    },
    world::{EventFinishType, ToServer},
};

use super::ZoneConnection;

pub struct ChatHandler {}

impl ChatHandler {
    /// Returns true if the command is handled, otherwise false.
    pub async fn handle_chat_message(
        connection: &mut ZoneConnection,
        chat_message: &SendChatMessage,
    ) -> bool {
        if connection.player_data.gm_rank == GameMasterRank::NormalUser {
            tracing::info!("Rejecting debug command because the user is not GM!");
            return true;
        }

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
                true
            }
            "!spawnmonster" => {
                if let Some((_, id)) = chat_message.message.split_once(' ') {
                    if let Ok(id) = id.parse::<u32>() {
                        connection
                            .handle
                            .send(ToServer::DebugNewEnemy(
                                connection.id,
                                connection.player_data.actor_id,
                                id,
                            ))
                            .await;
                    }
                }
                true
            }
            "!spawnclone" => {
                connection
                    .handle
                    .send(ToServer::DebugSpawnClone(
                        connection.id,
                        connection.player_data.actor_id,
                    ))
                    .await;
                true
            }
            "!equip" => {
                if let Some((_, name)) = chat_message.message.split_once(' ') {
                    {
                        let mut gamedata = connection.gamedata.lock();

                        if let Some(item_info) =
                            gamedata.get_item_info(ItemInfoQuery::ByName(name.to_string()))
                        {
                            let slot = gamedata
                                .get_equipslot_category(item_info.equip_category)
                                .unwrap();

                            let slot = connection.player_data.inventory.equipped.get_slot_mut(slot);

                            slot.id = item_info.id;
                            slot.glamour_catalog_id = 0;
                            slot.quantity = 1;
                            slot.condition = ITEM_CONDITION_MAX;
                        }
                    }

                    connection.send_inventory(false).await;
                    connection.inform_equip().await;
                }

                true
            }
            "!item" => {
                if let Some((_, name)) = chat_message.message.split_once(' ') {
                    let mut result = None;
                    {
                        let mut gamedata = connection.gamedata.lock();

                        if let Some(item_info) =
                            gamedata.get_item_info(ItemInfoQuery::ByName(name.to_string()))
                        {
                            result = connection
                                .player_data
                                .inventory
                                .add_in_next_free_slot(Item::new(item_info, 1));
                        }
                    }

                    if result.is_some() {
                        connection.send_inventory(false).await;
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        connection.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                    }
                }

                true
            }
            "!reload" => {
                connection.reload_scripts();
                connection.send_notice("Scripts reloaded!").await;
                true
            }
            "!finishevent" => {
                if let Some(event) = &connection.event {
                    connection
                        .event_finish(event.id, 0, EventFinishType::Normal)
                        .await;
                    connection
                        .send_notice("Current event forcefully finished.")
                        .await;
                }
                true
            }
            "!replay" => {
                if let Some((_, path)) = chat_message.message.split_once(' ') {
                    connection.replay_packets(path).await;
                }

                true
            }
            "!condition" => {
                if let Some((_, condition_name)) = chat_message.message.split_once(' ') {
                    if let Ok(condition) = Condition::from_str(condition_name) {
                        connection.conditions.set_condition(condition);
                        connection.send_conditions().await;
                        connection
                            .send_notice(&format!("Condition {condition:?} set!"))
                            .await;
                    } else {
                        connection
                            .send_notice(&format!("Unknown condition {condition_name}"))
                            .await;
                    }
                }

                true
            }
            "!clearconditions" => {
                connection.conditions = Conditions::default();
                connection.send_conditions().await;
                connection.send_notice("Conditions cleared!").await;

                true
            }
            "!acs" => {
                let parts: Vec<&str> = chat_message.message.split(' ').collect();

                connection
                    .actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::Unknown {
                            category: parts.get(1).cloned().unwrap().parse().unwrap(),
                            param1: parts
                                .get(2)
                                .cloned()
                                .unwrap_or_default()
                                .parse()
                                .unwrap_or_default(),
                            param2: parts
                                .get(3)
                                .cloned()
                                .unwrap_or_default()
                                .parse()
                                .unwrap_or_default(),
                            param3: parts
                                .get(4)
                                .cloned()
                                .unwrap_or_default()
                                .parse()
                                .unwrap_or_default(),
                            param4: parts
                                .get(5)
                                .cloned()
                                .unwrap_or_default()
                                .parse()
                                .unwrap_or_default(),
                        },
                    })
                    .await;

                true
            }
            "!mount" => {
                if let Some((_, mount)) = chat_message.message.split_once(' ') {
                    let mount_id = mount.parse::<u16>().unwrap();
                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Mount {
                        id: mount_id,
                        unk1: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                    });
                    connection.send_ipc_self(ipc).await;
                }

                true
            }
            _ => false,
        }
    }
}
