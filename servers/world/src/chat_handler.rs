use std::str::FromStr;

use crate::{
    Event, ItemInfoQuery, ToServer,
    event::EventHandler,
    inventory::{Item, Storage},
};
use kawari::{
    common::{ERR_INVENTORY_ADD_FAILED, FateState},
    ipc::zone::{
        ActorControlCategory, Condition, Conditions, GameMasterRank, SendChatMessage,
        ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

use super::ZoneConnection;

pub struct ChatHandler {}

impl ChatHandler {
    /// Returns true if the command is handled, otherwise false.
    pub async fn handle_chat_message(
        connection: &mut ZoneConnection,
        chat_message: &SendChatMessage,
        events: &mut Vec<(Box<dyn EventHandler>, Event)>,
    ) -> bool {
        if connection.player_data.character.gm_rank == GameMasterRank::NormalUser {
            tracing::info!("Rejecting debug command because the user is not GM!");
            return true;
        }

        let parts: Vec<&str> = chat_message.message.split(' ').collect();
        match parts[0] {
            "!spawnmonster" => {
                if let Some((_, id)) = chat_message.message.split_once(' ')
                    && let Ok(id) = id.parse::<u32>()
                {
                    connection
                        .handle
                        .send(ToServer::DebugNewEnemy(
                            connection.id,
                            connection.player_data.character.actor_id,
                            id,
                        ))
                        .await;
                }
                true
            }
            "!spawnclone" => {
                connection
                    .handle
                    .send(ToServer::DebugSpawnClone(
                        connection.id,
                        connection.player_data.character.actor_id,
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
                            let slot = connection
                                .player_data
                                .inventory
                                .equipped
                                .get_slot_mut(item_info.equip_category as u16);
                            *slot = Item::new(&item_info, 1);
                        }
                    }

                    connection.send_inventory().await;
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
                                .add_in_next_free_slot(Item::new(&item_info, 1));
                        }
                    }

                    if result.is_some() {
                        connection.send_inventory().await;
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        connection.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                    }
                }

                true
            }
            "!reload" => {
                connection.reload_scripts().await;
                connection.send_notice("Scripts reloaded!").await;
                true
            }
            "!finishevent" => {
                connection.event_finish(events).await;
                connection
                    .send_notice("Current event forcefully finished.")
                    .await;
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
                    .actor_control_self(ActorControlCategory::Unknown {
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
                        param5: parts
                            .get(6)
                            .cloned()
                            .unwrap_or_default()
                            .parse()
                            .unwrap_or_default(),
                    })
                    .await;

                true
            }
            "!mount" => {
                if let Some((_, mount)) = chat_message.message.split_once(' ') {
                    let mount_id = match mount.parse::<u16>() {
                        Ok(id) => id,
                        Err(_) => {
                            let mut gamedata = connection.gamedata.lock();
                            gamedata
                                .get_mount_id_from_name(mount.to_string())
                                .unwrap_or(1) // Fallback to a company chocobo otherwise
                        }
                    };

                    connection
                        .handle
                        .send(ToServer::DebugMount(
                            connection.id,
                            connection.player_data.character.actor_id,
                            mount_id,
                        ))
                        .await;
                }

                true
            }
            "!fate" => {
                if let Some((_, fate_id)) = chat_message.message.split_once(' ') {
                    let fate_id = fate_id.parse().unwrap();

                    connection
                        .actor_control_self(ActorControlCategory::CreateFateContext {
                            fate_id,
                            is_bonus: 0,
                        })
                        .await;

                    connection
                        .send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::UnkFate {
                            fate_id,
                            unk1: 0,
                            unk2: 1774393044,
                            unk3: 0,
                            unk4: 900,
                            unk5: 0,
                        }))
                        .await;

                    connection
                        .actor_control_self(ActorControlCategory::FateInit {
                            fate_id,
                            fate_state: FateState::Running,
                        })
                        .await;
                    connection
                        .actor_control_self(ActorControlCategory::UnkFate12 { fate_id })
                        .await;
                }

                true
            }
            _ => false,
        }
    }
}
