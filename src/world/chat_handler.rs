use crate::{
    inventory::Storage,
    ipc::zone::{ActorControlCategory, ActorControlSelf, ChatMessage},
    world::ToServer,
};

use super::ZoneConnection;

pub struct ChatHandler {}

impl ChatHandler {
    pub async fn handle_chat_message(connection: &mut ZoneConnection, chat_message: &ChatMessage) {
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
            "!spawnclone" => {
                connection
                    .handle
                    .send(ToServer::DebugSpawnClone(
                        connection.id,
                        connection.player_data.actor_id,
                    ))
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
