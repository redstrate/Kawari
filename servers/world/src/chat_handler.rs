use bstr::BString;
use std::str::FromStr;

use crate::{
    Event, ItemInfoQuery, ToServer,
    event::EventHandler,
    inventory::{Item, Storage},
};
use kawari::{
    common::{
        DirectorEvent, ERR_INVENTORY_ADD_FAILED, FateState, HandlerId, HandlerType, ObjectTypeId,
    },
    ipc::zone::{
        ActorControlCategory, Condition, Conditions, GameMasterRank, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};
use physis::equipment::EquipSlot;

use super::ZoneConnection;

pub struct ChatHandler {}

impl ChatHandler {
    /// Returns true if the command is handled, otherwise false.
    pub async fn handle_chat_message(
        connection: &mut ZoneConnection,
        chat_message: &BString, // TODO: Replace this with an SEString
        events: &mut Vec<(Box<dyn EventHandler>, Event)>,
    ) -> bool {
        if connection.player_data.character.gm_rank == GameMasterRank::NormalUser {
            tracing::info!("Rejecting debug command because the user is not GM!");
            return true;
        }

        // TODO: Ensure the message has no SEString macros (e.g. auto-translate phrases)?
        let chat_message = chat_message.to_string();

        let parts: Vec<&str> = chat_message.split(' ').collect();

        match parts[0] {
            "!spawnmonster" => {
                if let Some((_, id)) = chat_message.split_once(' ')
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
                if let Some((_, name)) = chat_message.split_once(' ') {
                    let item_info;
                    {
                        let mut gamedata = connection.gamedata.lock();
                        item_info = gamedata.get_item_info(ItemInfoQuery::ByName(name.to_string()));
                    }
                    if let Some(item_info) = item_info {
                        // Reject belts that may have forgotten to be changed to 0 and items that aren't equipment.
                        let belts_and_misc = [0, 6, 24]; // Invalid, belts, invalid respectively
                        if belts_and_misc.contains(&item_info.equip_category) {
                            connection.send_notice(&format!("[equip] The found item, {:#?}, isn't equipment! If the wrong item was found, try being more specific with its name, or consider using //gm item instead if you can't get the desired item from this command.", item_info.name).to_string()).await;
                            return true;
                        }

                        // EquipSlotCategory rows belong to these slot types. There's currently no physis mechanism for this, so we'll hardcode it here for the time being.
                        let main_hand = [1, 13, 14];
                        let legs = [7, 18];
                        let body = [4, 15, 16, 19, 20, 21, 22, 23];
                        let soul_crystal = 17;

                        // First, assume nothing special is going on. For most equipment you can just use the category minus one to get the correct slot.
                        let mut slot = if let Some(the_slot) =
                            EquipSlot::from_repr((item_info.equip_category - 1) as u16)
                        {
                            the_slot
                        } else {
                            EquipSlot::Waist // This should should never slip through since stuff like Herklaedi (category 16) or the Star Pilot Suit (category 23) will be caught below
                        };

                        // Convert more complicated EquipSlotCategory rows into proper slots.
                        if main_hand.contains(&item_info.equip_category) {
                            slot = EquipSlot::MainHand;
                        } else if legs.contains(&item_info.equip_category) {
                            slot = EquipSlot::Legs;
                        } else if body.contains(&item_info.equip_category) {
                            slot = EquipSlot::Body;
                        } else if item_info.equip_category == soul_crystal {
                            slot = EquipSlot::SoulCrystal;
                        }

                        assert!(slot != EquipSlot::Waist);

                        let slot = connection
                            .player_data
                            .inventory
                            .equipped
                            .get_slot_mut(slot as u16);
                        *slot = Item::new(&item_info, 1);

                        connection.send_inventory().await;
                        connection.inform_equip().await;
                    } else {
                        connection.send_notice(&format!("[equip] No items named {name:#?} were found! If you know its item id instead, consider using //gm item.").to_string()).await;
                    }
                } else {
                    connection
                        .send_notice("[equip] Usage: !equip <item name>")
                        .await;
                }

                true
            }
            "!item" => {
                if let Some((_, name)) = chat_message.split_once(' ') {
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
                if let Some((_, condition_name)) = chat_message.split_once(' ') {
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
                let parts: Vec<&str> = chat_message.split(' ').collect();

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
                if let Some((_, mount)) = chat_message.split_once(' ') {
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
                if let Some((_, fate_id)) = chat_message.split_once(' ') {
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
            "!yell" => {
                if let Some((_, npc_yell_id)) = chat_message.split_once(' ') {
                    let npc_yell_id = npc_yell_id.parse().unwrap();

                    let name_id;
                    {
                        let mut game_data = connection.gamedata.lock();
                        name_id = game_data
                            .get_npc_yell_name_id(npc_yell_id)
                            .unwrap_or_default();
                    }

                    connection
                        .send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::NpcYell {
                            object_id: ObjectTypeId::default(),
                            name_id,
                            npc_yell_id,
                            param1: 0,
                            param2: 0,
                            param3: 0,
                            param4: 0,
                        }))
                        .await;
                }

                true
            }
            "!gate" => {
                connection
                    .send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::NpcYell {
                        object_id: ObjectTypeId::default(),
                        name_id: 1010448,
                        npc_yell_id: 2450,
                        param1: 3,
                        param2: 7,
                        param3: 0,
                        param4: 0,
                    }))
                    .await;

                connection
                    .actor_control_self(ActorControlCategory::InitDirector {
                        handler_id: HandlerId::new(HandlerType::GoldSaucer, 1319),
                        content_id: 10,
                        flags: 0,
                    })
                    .await;

                connection
                    .actor_control_self(ActorControlCategory::DirectorEvent {
                        handler_id: HandlerId::new(HandlerType::GoldSaucer, 1319),
                        event: DirectorEvent::Unknown(11),
                        arg1: 3,
                        arg2: 0,
                    })
                    .await;

                true
            }
            "!ofbg" => {
                let parts: Vec<&str> = chat_message.split(' ').collect();

                // TODO: Should we reject doing this if it's attempted from other types of content?
                // Ocean fishing uses "festival" ids of 101 & 102, set when entering the zone, but it's more convenient to set it in this command for the time being
                connection
                    .actor_control_self(ActorControlCategory::SetFestival {
                        festival1: 101,
                        festival2: 102,
                        festival3: 0,
                        festival4: 0,
                    })
                    .await;
                // The director sends this with a background arg and a "phase" arg when the scenery needs to change. See the IKDSpot sheet for arg1 values (the row number should be increased by 1, so Kugane Coast would be 10, not 9).
                connection
                    .actor_control_self(ActorControlCategory::DirectorEvent {
                        handler_id: connection.content_handler_id,
                        event: DirectorEvent::Unknown(2),
                        arg1: parts
                            .get(1)
                            .cloned()
                            .unwrap_or_default()
                            .parse()
                            .unwrap_or_default(),
                        arg2: parts
                            .get(2)
                            .cloned()
                            .unwrap_or_default()
                            .parse()
                            .unwrap_or_default(),
                    })
                    .await;
                true
            }
            "!settime" => {
                // TODO: Figure out how UTC is converted to Eorzean time and make this friendly by allowing for strings such as "6:30PM" or "18:30"
                // TODO: Write the GM command equivalent which would just set the time offset directly as an i64/u64 (whichever this actually is)
                let val = chat_message.split_once(' ').unwrap();
                let val = val.1.parse::<i64>().unwrap();
                connection.set_eorzean_time(val).await;

                true
            }
            _ => false,
        }
    }
}
