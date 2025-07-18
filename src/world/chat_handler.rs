use crate::{
    ERR_INVENTORY_ADD_FAILED, ITEM_CONDITION_MAX,
    common::ItemInfoQuery,
    inventory::{Item, Storage},
    ipc::zone::{ChatMessage, GameMasterRank},
    world::ToServer,
};

use super::ZoneConnection;

pub struct ChatHandler {}

impl ChatHandler {
    /// Returns true if the command is handled, otherwise false.
    pub async fn handle_chat_message(
        connection: &mut ZoneConnection,
        chat_message: &ChatMessage,
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
                connection
                    .handle
                    .send(ToServer::DebugNewEnemy(
                        connection.id,
                        connection.player_data.actor_id,
                    ))
                    .await;
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
                let (_, name) = chat_message.message.split_once(' ').unwrap();

                {
                    let mut gamedata = connection.gamedata.lock().unwrap();

                    if let Some(item_info) =
                        gamedata.get_item_info(ItemInfoQuery::ByName(name.to_string()))
                    {
                        let slot = gamedata
                            .get_equipslot_category(item_info.equip_category)
                            .unwrap();

                        let slot = connection.player_data.inventory.equipped.get_slot_mut(slot);

                        slot.id = item_info.id;
                        slot.quantity = 1;
                        slot.condition = ITEM_CONDITION_MAX;
                    }
                }

                connection.send_inventory(false).await;
                connection.inform_equip().await;
                true
            }
            "!item" => {
                let (_, name) = chat_message.message.split_once(' ').unwrap();
                let mut result = None;
                {
                    let mut gamedata = connection.gamedata.lock().unwrap();

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
                    connection.send_message(ERR_INVENTORY_ADD_FAILED).await;
                }

                true
            }
            "!reload" => {
                connection.reload_scripts();
                connection.send_message("Scripts reloaded!").await;
                true
            }
            "!finishevent" => {
                if let Some(event) = &connection.event {
                    connection.event_finish(event.id).await;
                    connection
                        .send_message("Current event forcefully finished.")
                        .await;
                }
                true
            }
            "!replay" => {
                let (_, path) = chat_message.message.split_once(' ').unwrap();
                connection.replay_packets(path).await;

                true
            }
            _ => false,
        }
    }
}
