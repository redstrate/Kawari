//! Chat and command handling.

use std::str::FromStr;

use bstr::BString;
use mlua::Function;
use physis::equipment::EquipSlot;

use crate::{
    Event, EventHandler, ItemInfoQuery, MessageInfo, ZoneConnection,
    inventory::{Item, Storage},
    lua::{KawariLuaState, LuaPlayer},
};
use kawari::{
    common::{
        DirectorEvent, ERR_INVENTORY_ADD_FAILED, FateState, HandlerId, HandlerType, ObjectTypeId,
    },
    config::FilesystemConfig,
    ipc::zone::{
        ActorControlCategory, ChatMessage, Condition, Conditions, GameMasterRank,
        ServerNoticeFlags, ServerNoticeMessage, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

impl ZoneConnection {
    pub async fn send_message(&mut self, message: MessageInfo) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ChatMessage(ChatMessage {
            sender_account_id: message.sender_account_id,
            sender_actor_id: message.sender_actor_id,
            sender_world_id: message.sender_world_id,
            sender_name: message.sender_name,
            channel: message.channel,
            message: message.message,
            ..Default::default()
        }));
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_notice(&mut self, message: &str) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
            ServerNoticeMessage {
                flags: ServerNoticeFlags::CHAT_LOG,
                message: message.to_string(),
            },
        ));

        self.send_ipc_self(ipc).await;
    }

    pub async fn run_gm_command(
        &mut self,
        command: u32,
        arg0: i32,
        arg1: i32,
        arg2: i32,
        arg3: i32,
        name: String,
        lua_player: &mut LuaPlayer,
    ) {
        let lua = self.lua.lock();
        let state = lua.0.app_data_ref::<KawariLuaState>().unwrap();

        if let Some(command_script) = state.gm_command_scripts.get(&command) {
            let file_name = FilesystemConfig::locate_script_file(command_script);

            let run_script = || -> mlua::Result<()> {
                lua.0.scope(|scope| {
                    let connection_data = scope
                    .create_userdata_ref_mut(lua_player)?;
                    /* TODO: Instead of panicking we ought to send a message to the player
                     * and the console log, and abandon execution. */
                    lua.0.load(
                        std::fs::read(&file_name).unwrap_or_else(|_| panic!("Failed to load script file {}!", &file_name)),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()?;

                    let required_rank = lua.0.globals().get("required_rank");
                    if let Err(error) = required_rank {
                        tracing::info!("Script is missing required_rank! Unable to run command, sending error to user. Additional information: {}", error);
                        let func: Function =
                        lua.0.globals().get("onCommandRequiredRankMissingError")?;
                        func.call::<()>((error.to_string(), connection_data))?;
                        return Ok(());
                    }

                    /* Reset state for future commands. Without this it'll stay set to the last value
                     * and allow other commands that omit required_rank to run, which is undesirable. */
                    lua.0.globals().set("required_rank", mlua::Value::Nil)?;

                    if self.player_data.character.gm_rank as u8 >= required_rank? {
                        let func: Function =
                        lua.0.globals().get("onCommand")?;
                        func.call::<()>((connection_data, [arg0, arg1, arg2, arg3], name))?;

                        /* `command_sender` is an optional variable scripts can define to identify themselves in print messages.
                         * It's okay if this global isn't set. We also don't care what its value is, just that it exists.
                         * This is reset -after- running the command intentionally. Resetting beforehand will never display the command's identifier.
                         */
                        let command_sender: Result<mlua::prelude::LuaValue, mlua::prelude::LuaError> = lua.0.globals().get("command_sender");
                        if command_sender.is_ok() {
                            lua.0.globals().set("command_sender", mlua::Value::Nil)?;
                        }
                        Ok(())
                    } else {
                        tracing::info!("User with account_id {} tried to invoke GM command {} with insufficient privileges!",
                                       self.player_data.character.service_account_id, command);
                        let func: Function =
                        lua.0.globals().get("onCommandRequiredRankInsufficientError")?;
                        func.call::<()>(connection_data)?;
                        Ok(())
                    }
                })
            };

            if let Err(err) = run_script() {
                tracing::warn!("Lua error in {file_name}: {:?}", err);
            }
        } else {
            tracing::warn!(
                "Received unknown GM command {command} with args: arg0: {arg0} arg1: {arg1} arg2: {arg2} arg3: {arg3}!"
            );
        }
    }

    /// Returns true if the debug command is handled, otherwise false.
    pub async fn process_debug_commands(
        &mut self,
        chat_message: &BString, // TODO: Replace this with an SEString
        events: &mut Vec<(Box<dyn EventHandler>, Event)>,
    ) -> bool {
        if self.player_data.character.gm_rank == GameMasterRank::NormalUser {
            tracing::info!("Rejecting debug command because the user is not GM!");
            return true;
        }

        // TODO: Ensure the message has no SEString macros (e.g. auto-translate phrases)?
        let chat_message = chat_message.to_string();

        let parts: Vec<&str> = chat_message.split(' ').collect();

        match parts[0] {
            "!equip" => {
                if let Some((_, name)) = chat_message.split_once(' ') {
                    let item_info;
                    {
                        let mut gamedata = self.gamedata.lock();
                        item_info = gamedata.get_item_info(ItemInfoQuery::ByName(name.to_string()));
                    }
                    if let Some(item_info) = item_info {
                        let equip_slot = EquipSlot::from(&item_info.equip_category);

                        // EquipSlot::from returns EquipSlot::Waist if the item isn't equipment, since belts are no longer part of the game.
                        if equip_slot == EquipSlot::Waist {
                            self.send_notice(&format!("[equip] The found item, {:#?}, isn't equipment! If the wrong item was found, try being more specific with its name, or consider using //gm item instead if you can't get the desired item from this command.", item_info.name).to_string()).await;
                            return true;
                        };

                        let slot = self
                            .player_data
                            .inventory
                            .equipped
                            .get_slot_mut(equip_slot as u16);
                        *slot = Item::new(&item_info, 1);

                        self.send_inventory().await;
                        self.inform_equip().await;
                    } else {
                        self.send_notice(&format!("[equip] No items named {name:#?} were found! If you know its item id instead, consider using //gm item.").to_string()).await;
                    }
                } else {
                    self.send_notice("[equip] Usage: !equip <item name>").await;
                }

                true
            }
            "!item" => {
                if let Some((_, name)) = chat_message.split_once(' ') {
                    let mut result = None;
                    {
                        let mut gamedata = self.gamedata.lock();

                        if let Some(item_info) =
                            gamedata.get_item_info(ItemInfoQuery::ByName(name.to_string()))
                        {
                            result = self
                                .player_data
                                .inventory
                                .add_in_next_free_slot(Item::new(&item_info, 1));
                        }
                    }

                    if result.is_some() {
                        self.send_inventory().await;
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        self.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                    }
                }

                true
            }
            "!reload" => {
                self.reload_scripts().await;
                self.send_notice("Scripts reloaded!").await;
                true
            }
            "!finishevent" => {
                self.event_finish(events).await;
                self.send_notice("Current event forcefully finished.").await;
                true
            }
            "!condition" => {
                if let Some((_, condition_name)) = chat_message.split_once(' ') {
                    if let Ok(condition) = Condition::from_str(condition_name) {
                        self.conditions.set_condition(condition);
                        self.send_conditions().await;
                        self.send_notice(&format!("Condition {condition:?} set!"))
                            .await;
                    } else {
                        self.send_notice(&format!("Unknown condition {condition_name}"))
                            .await;
                    }
                }

                true
            }
            "!clearconditions" => {
                self.conditions = Conditions::default();
                self.send_conditions().await;
                self.send_notice("Conditions cleared!").await;

                true
            }
            "!acs" => {
                let parts: Vec<&str> = chat_message.split(' ').collect();

                self.actor_control_self(ActorControlCategory::Unknown {
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
            "!fate" => {
                if let Some((_, fate_id)) = chat_message.split_once(' ') {
                    let fate_id = fate_id.parse().unwrap();

                    self.actor_control_self(ActorControlCategory::CreateFateContext {
                        fate_id,
                        is_bonus: 0,
                    })
                    .await;

                    self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::UnkFate {
                        fate_id,
                        unk1: 0,
                        unk2: 1774393044,
                        unk3: 0,
                        unk4: 900,
                        unk5: 0,
                    }))
                    .await;

                    self.actor_control_self(ActorControlCategory::FateInit {
                        fate_id,
                        fate_state: FateState::Running,
                    })
                    .await;
                    self.actor_control_self(ActorControlCategory::UnkFate12 { fate_id })
                        .await;
                }

                true
            }
            "!yell" => {
                if let Some((_, npc_yell_id)) = chat_message.split_once(' ') {
                    let npc_yell_id = npc_yell_id.parse().unwrap();

                    let name_id;
                    {
                        let mut game_data = self.gamedata.lock();
                        name_id = game_data
                            .get_npc_yell_name_id(npc_yell_id)
                            .unwrap_or_default();
                    }

                    self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::NpcYell {
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
                self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::NpcYell {
                    object_id: ObjectTypeId::default(),
                    name_id: 1010448,
                    npc_yell_id: 2450,
                    param1: 3,
                    param2: 7,
                    param3: 0,
                    param4: 0,
                }))
                .await;

                self.actor_control_self(ActorControlCategory::InitDirector {
                    handler_id: HandlerId::new(HandlerType::GoldSaucer, 1319),
                    content_id: 10,
                    flags: 0,
                })
                .await;

                self.actor_control_self(ActorControlCategory::DirectorEvent {
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
                self.actor_control_self(ActorControlCategory::SetFestival {
                    festival1: 101,
                    festival2: 102,
                    festival3: 0,
                    festival4: 0,
                })
                .await;
                // The director sends this with a background arg and a "phase" arg when the scenery needs to change. See the IKDSpot sheet for arg1 values (the row number should be increased by 1, so Kugane Coast would be 10, not 9).
                self.actor_control_self(ActorControlCategory::DirectorEvent {
                    handler_id: self.content_handler_id,
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
                self.set_eorzean_time(val).await;

                true
            }
            _ => false,
        }
    }
}
