use crate::{
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

                    if let Some((equip_category, id)) = gamedata.get_item_by_name(name) {
                        let slot = gamedata.get_equipslot_category(equip_category).unwrap();

                        connection
                            .player_data
                            .inventory
                            .equipped
                            .get_slot_mut(slot)
                            .id = id;
                        connection
                            .player_data
                            .inventory
                            .equipped
                            .get_slot_mut(slot)
                            .quantity = 1;
                    }
                }

                connection.send_inventory(true).await;
                true
            }
            "!item" => {
                let (_, name) = chat_message.message.split_once(' ').unwrap();

                {
                    let mut gamedata = connection.gamedata.lock().unwrap();

                    if let Some((_, id)) = gamedata.get_item_by_name(name) {
                        connection
                            .player_data
                            .inventory
                            .add_in_next_free_slot(Item::new(1, id));
                    }
                }

                connection.send_inventory(false).await;
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
            _ => false,
        }
    }
}
