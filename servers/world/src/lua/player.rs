use std::sync::Arc;

use mlua::{LuaSerdeExt, UserData, UserDataFields, UserDataMethods, Value};
use parking_lot::Mutex;

use crate::{
    GameData, PlayerData, RemakeMode, StatusEffects,
    inventory::{CurrencyKind, Item},
};
use kawari::{
    common::{
        ContainerType, HandlerId, INVENTORY_ACTION_ACK_SHOP, LogMessageType, ObjectTypeId,
        ObjectTypeKind, Position, adjust_quest_id,
    },
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, EventScene, EventType, OnlineStatus, SceneFlags,
        ServerNoticeFlags, ServerNoticeMessage, ServerZoneIpcData, ServerZoneIpcSegment, Warp,
    },
    packet::PacketSegment,
};

use super::{LuaTask, LuaZone, QueueSegments, create_ipc_self};

#[derive(Default, Clone, Copy)]
pub struct LuaContent {
    /// Duration in seconds.
    pub duration: u16,
    /// Duty finder settings. See ContentRegistrationFlags.
    pub settings: u32,
}

impl UserData for LuaContent {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("duration", |_, this| Ok(this.duration));

        fields.add_field_method_get("settings", |_, this| Ok(this.settings));
    }
}

#[derive(Default)]
pub struct LuaPlayer {
    pub player_data: PlayerData,
    pub queued_tasks: Vec<LuaTask>,
    pub zone_data: LuaZone,
    pub status_effects: StatusEffects,
    pub content_data: LuaContent,
}

impl QueueSegments for LuaPlayer {
    fn queue_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        self.queued_tasks.push(LuaTask::SendSegment { segment });
    }
}

impl LuaPlayer {
    fn send_message(&mut self, message: &str, param: u8) {
        // This is a completely arbitrary string, so we have to make sure it's the proper size.
        let mut message = message.to_string();
        message.truncate(775);

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
            ServerNoticeMessage {
                message,
                flags: ServerNoticeFlags::from_bits(param).unwrap_or_default(),
            },
        ));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn give_status_effect(&mut self, effect_id: u16, effect_param: u16, duration: f32) {
        self.queued_tasks.push(LuaTask::GainStatusEffect {
            effect_id,
            effect_param,
            duration,
        });
    }

    fn play_scene(
        &mut self,
        target: ObjectTypeId,
        event_id: u32,
        scene: u16,
        scene_flags: SceneFlags,
        params: Vec<u32>,
    ) {
        let scene = EventScene {
            actor_id: target,
            handler_id: HandlerId(event_id),
            scene,
            scene_flags,
            params_count: params.len() as u8,
            params: params.clone(),
            ..Default::default()
        };

        if let Some(ipc) = scene.package_scene() {
            create_ipc_self(self, ipc, self.player_data.character.actor_id);
        } else {
            let error_message = "Unsupported amount of parameters in play_scene! This is likely a bug in your script! Cancelling event...".to_string();
            tracing::warn!(error_message);
            self.send_message(&error_message, 0);
            self.finish_event(event_id);
        }
    }

    fn set_position(&mut self, position: Position, rotation: f32) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Warp(Warp {
            dir: rotation,
            position,
            ..Default::default()
        }));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn set_festival(&mut self, festival1: u32, festival2: u32, festival3: u32, festival4: u32) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::SetFestival {
                    festival1,
                    festival2,
                    festival3,
                    festival4,
                },
            }));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn unlock(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::Unlock { id });
    }

    fn set_speed(&mut self, speed: u16) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::Flee { speed },
            }));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn toggle_wireframe(&mut self) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::ToggleWireframeRendering(),
            }));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn unlock_aetheryte(&mut self, unlocked: u32, id: u32) {
        self.queued_tasks.push(LuaTask::UnlockAetheryte {
            id,
            on: unlocked == 1,
        });
    }

    fn change_territory(
        &mut self,
        zone_id: u16,
        exit_position: Option<Position>,
        exit_rotation: Option<f32>,
    ) {
        self.queued_tasks.push(LuaTask::ChangeTerritory {
            zone_id,
            exit_position,
            exit_rotation,
        });
    }

    fn set_remake_mode(&mut self, mode: RemakeMode) {
        self.queued_tasks.push(LuaTask::SetRemakeMode(mode));
    }

    fn warp(&mut self, warp_id: u32) {
        self.queued_tasks.push(LuaTask::Warp { warp_id });
    }

    fn begin_log_out(&mut self) {
        self.queued_tasks.push(LuaTask::BeginLogOut);
    }

    fn finish_event(&mut self, handler_id: u32) {
        self.queued_tasks.push(LuaTask::FinishEvent { handler_id });
    }

    fn unlock_classjob(&mut self, classjob_id: u8) {
        self.queued_tasks
            .push(LuaTask::UnlockClassJob { classjob_id });
    }

    fn warp_aetheryte(&mut self, aetheryte_id: u32) {
        self.queued_tasks
            .push(LuaTask::WarpAetheryte { aetheryte_id });
    }

    fn reload_scripts(&mut self) {
        self.queued_tasks.push(LuaTask::ReloadScripts);
    }

    fn toggle_invisiblity(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleInvisibility {
            invisible: !self.player_data.gm_invisible,
        });
    }

    fn set_level(&mut self, level: u16) {
        self.queued_tasks.push(LuaTask::SetLevel { level });
    }

    fn change_weather(&mut self, id: u16) {
        self.queued_tasks.push(LuaTask::ChangeWeather { id });
    }

    fn modify_currency(&mut self, id: CurrencyKind, amount: i32, send_client_update: bool) {
        self.queued_tasks.push(LuaTask::ModifyCurrency {
            id,
            amount,
            send_client_update,
        });
    }

    fn gm_set_orchestrion(&mut self, value: bool, id: u32) {
        self.queued_tasks
            .push(LuaTask::GmSetOrchestrion { value, id });
    }

    fn toggle_orchestrion(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleOrchestrion { id });
    }

    fn add_item(&mut self, id: u32, quantity: u32, send_client_update: bool) {
        self.queued_tasks.push(LuaTask::AddItem {
            id,
            quantity,
            send_client_update,
        });
    }

    fn unlock_content(&mut self, id: u16) {
        self.queued_tasks.push(LuaTask::UnlockContent { id });
    }

    fn unlock_all_content(&mut self) {
        self.queued_tasks.push(LuaTask::UnlockAllContent {});
    }

    fn get_buyback_list(&mut self, shop_id: u32, shop_intro: bool) -> Vec<u32> {
        let ret = self
            .player_data
            .buyback_list
            .as_scene_params(shop_id, shop_intro);
        if !shop_intro {
            self.queued_tasks.push(LuaTask::UpdateBuyBackList {
                list: self.player_data.buyback_list.clone(),
            })
        }
        ret
    }

    fn do_gilshop_buyback(&mut self, shop_id: u32, buyback_index: u32) {
        let bb_item;
        {
            let Some(tmp_bb_item) = self
                .player_data
                .buyback_list
                .get_buyback_item(shop_id, buyback_index)
            else {
                let error = "Invalid buyback index, ignoring buyback action!";
                self.send_message(error, 0);
                tracing::warn!(error);
                return;
            };
            bb_item = tmp_bb_item.clone();
        }

        // This is a no-op since we can't edit PlayerData from the Lua side, but we can queue it up afterward.
        // We *need* this information, though.
        let item_to_restore = Item::new(bb_item.as_item_info(), bb_item.quantity);
        let Some(item_dst_info) = self
            .player_data
            .inventory
            .add_in_next_free_slot(item_to_restore)
        else {
            let error = "Your inventory is full. Unable to restore item.";
            self.send_message(error, 0);
            tracing::warn!(error);
            return;
        };

        // This is a no-op since we can't edit PlayerData from the Lua side,
        // but we need to do it here so the shopkeeper script doesn't see stale data.
        self.player_data
            .buyback_list
            .remove_item(shop_id, buyback_index);

        // Queue up the item restoration, but we're not going to send an entire inventory update to the client.
        self.add_item(bb_item.id, item_dst_info.quantity, false);

        // Queue up the player's adjusted gil, but we're not going to send an entire inventory update to the client.
        let cost = item_dst_info.quantity * bb_item.price_low;
        let new_gil = self.player_data.inventory.currency.gil.quantity - cost;
        self.modify_currency(CurrencyKind::Gil, -(cost as i32), false);

        let shop_packets_to_send = [
            ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                sequence: self.player_data.shop_sequence,
                dst_storage_id: ContainerType::Currency,
                dst_container_index: 0,
                dst_stack: new_gil,
                dst_catalog_id: CurrencyKind::Gil as u32,
                unk1: 0x7530_0000,
            }),
            ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryActionAck {
                sequence: u32::MAX,
                action_type: INVENTORY_ACTION_ACK_SHOP as u16,
            }),
            ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                sequence: self.player_data.shop_sequence,
                dst_storage_id: item_dst_info.container,
                dst_container_index: item_dst_info.index,
                dst_stack: item_dst_info.quantity,
                dst_catalog_id: bb_item.id,
                unk1: 0x7530_0000,
            }),
            ServerZoneIpcSegment::new(ServerZoneIpcData::ShopLogMessage {
                handler_id: HandlerId(shop_id),
                message_type: LogMessageType::ItemBoughtBack as u32,
                params_count: 3,
                item_id: bb_item.id,
                item_quantity: item_dst_info.quantity,
                total_sale_cost: item_dst_info.quantity * bb_item.price_low,
            }),
        ];

        // Finally, queue up the packets required to make the magic happen.
        for ipc in shop_packets_to_send {
            create_ipc_self(self, ipc, self.player_data.character.actor_id);
        }
    }

    fn do_solnine_teleporter(
        &mut self,
        event_id: u32,
        path_id: u32,
        unk2: u16,
        unk3: u16,
        speed: u16,
        unk4: u16,
        unk5: u32,
    ) {
        let packets_to_send = [
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::DisableEventPosRollback {
                    handler_id: HandlerId(event_id),
                },
            })),
            ServerZoneIpcSegment::new(ServerZoneIpcData::WalkInEvent {
                path_id,
                unk2,
                unk3,
                speed,
                constant: 1,
                unk4,
                unk5,
            }),
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::WalkInTriggerRelatedUnk3 {
                    unk1: 1, // Sometimes the server sends 2 for this, but it's still completely unknown what it means.
                },
            })),
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::SetPetEntityId { unk1: 1 },
            })),
        ];

        for ipc in packets_to_send {
            create_ipc_self(self, ipc, self.player_data.character.actor_id);
        }
    }

    fn add_exp(&mut self, amount: i32) {
        self.queued_tasks.push(LuaTask::AddExp { amount });
    }

    fn start_event(
        &mut self,
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: EventType,
        event_arg: u32,
    ) {
        self.queued_tasks.push(LuaTask::StartEvent {
            actor_id,
            event_id,
            event_type,
            event_arg,
        });
    }

    fn set_inn_wakeup(&mut self, watched: bool) {
        self.queued_tasks.push(LuaTask::SetInnWakeup { watched });
    }

    fn toggle_mount(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleMount { id });
    }

    fn toggle_glasses_style(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleGlassesStyle { id });
    }

    fn toggle_glasses_style_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleGlassesStyleAll {});
    }

    fn toggle_ornament(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleOrnament { id });
    }

    fn toggle_ornament_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleOrnamentAll {});
    }

    fn unlock_buddy_equip(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::UnlockBuddyEquip { id });
    }

    fn unlock_buddy_equip_all(&mut self) {
        self.queued_tasks.push(LuaTask::UnlockBuddyEquipAll {});
    }

    fn toggle_chocobo_taxi_stand(&mut self, id: u32) {
        self.queued_tasks
            .push(LuaTask::ToggleChocoboTaxiStand { id });
    }

    fn toggle_chocobo_taxi_stand_all(&mut self) {
        self.queued_tasks
            .push(LuaTask::ToggleChocoboTaxiStandAll {});
    }

    fn toggle_caught_fish(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleCaughtFish { id });
    }

    fn toggle_caught_fish_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleCaughtFishAll {});
    }

    fn toggle_caught_spearfish(&mut self, id: u32) {
        self.queued_tasks
            .push(LuaTask::ToggleCaughtSpearfish { id });
    }

    fn toggle_caught_spearfish_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleCaughtSpearfishAll {});
    }

    fn toggle_triple_triad_card(&mut self, id: u32) {
        self.queued_tasks
            .push(LuaTask::ToggleTripleTriadCard { id });
    }

    fn toggle_triple_triad_card_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleTripleTriadCardAll {});
    }

    fn toggle_adventure(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleAdventure { id });
    }

    fn toggle_adventure_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleAdventureAll {});
    }

    fn toggle_cutscene_seen(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleCutsceneSeen { id });
    }

    fn toggle_cutscene_seen_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleCutsceneSeenAll {});
    }

    fn toggle_minion(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleMinion { id });
    }

    fn toggle_minion_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleMinionAll {});
    }

    fn toggle_aether_current(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::ToggleAetherCurrent { id });
    }

    fn toggle_aether_current_all(&mut self) {
        self.queued_tasks.push(LuaTask::ToggleAetherCurrentAll {});
    }

    fn toggle_aether_current_comp_flg_set(&mut self, id: u32) {
        self.queued_tasks
            .push(LuaTask::ToggleAetherCurrentCompFlgSet { id });
    }

    fn toggle_aether_current_comp_flg_set_all(&mut self) {
        self.queued_tasks
            .push(LuaTask::ToggleAetherCurrentCompFlgSetAll {});
    }

    fn move_to_pop_range(&mut self, id: u32, fade_out: bool) {
        self.queued_tasks
            .push(LuaTask::MoveToPopRange { id, fade_out });
    }

    fn set_hp(&mut self, hp: u32) {
        self.queued_tasks.push(LuaTask::SetHP { hp });
    }

    fn set_mp(&mut self, mp: u16) {
        self.queued_tasks.push(LuaTask::SetMP { mp });
    }

    fn set_race(&mut self, race: u8) {
        self.queued_tasks.push(LuaTask::SetRace { race });
    }

    fn set_tribe(&mut self, tribe: u8) {
        self.queued_tasks.push(LuaTask::SetTribe { tribe });
    }

    fn set_sex(&mut self, sex: u8) {
        self.queued_tasks.push(LuaTask::SetSex { sex });
    }

    fn start_talk_event(&mut self) {
        self.queued_tasks.push(LuaTask::StartTalkEvent {});
    }

    fn accept_quest(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::AcceptQuest { id });
    }

    fn finish_quest(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::FinishQuest { id });
    }

    fn prepare_zoning(&mut self, timeout: u8) {
        create_ipc_self(
            self,
            ServerZoneIpcSegment::new(ServerZoneIpcData::PrepareZoning {
                log_message: 0,
                target_zone: self.zone_data.zone_id,
                animation: 0,
                param4: 0,
                hide_character: 0,
                fade_out: 1,
                param_7: 1,
                fade_out_time: timeout,
                unk1: 0,
                unk2: 0,
            }),
            self.player_data.character.actor_id,
        );
    }

    fn commence_duty(&mut self, director_id: u32) {
        self.queued_tasks
            .push(LuaTask::CommenceDuty { director_id });
    }

    /// Returns the target DefaultTalk event for a given SwitchTalk event.
    /// This takes quest completion into account.
    fn get_switch_talk_target(
        &mut self,
        game_data: mlua::Value,
        switch_talk_id: u32,
    ) -> Option<u32> {
        let game_data = match game_data {
            mlua::Value::UserData(ud) => ud.borrow::<Arc<Mutex<GameData>>>().unwrap().clone(),
            _ => unreachable!(),
        };

        let mut game_data = game_data.lock();

        let subrows = game_data.get_switch_talk_subrows(switch_talk_id);
        // Higher subrows take precedence
        for (_, row) in subrows.iter().rev() {
            let quest0 = row
                .Quest0()
                .into_u32()
                .copied()
                .map(adjust_quest_id)
                .unwrap_or_default();
            let quest1 = row
                .Quest1()
                .into_u32()
                .copied()
                .map(adjust_quest_id)
                .unwrap_or_default();

            let should_check_quest0 = quest0 != 0;
            let should_check_quest1 = quest1 != 0;

            let quest0_completed = self.player_data.quest.completed.contains(quest0);
            let quest1_completed = self.player_data.quest.completed.contains(quest1);

            let quest0_passed = if should_check_quest0 {
                quest0_completed
            } else {
                true
            };
            let quest1_passed = if should_check_quest1 {
                quest1_completed
            } else {
                true
            };

            if quest0_passed && quest1_passed {
                return row.DefaultTalk().into_u32().copied();
            }
        }

        None
    }

    fn register_for_content(&mut self, content_id: u16) {
        self.queued_tasks
            .push(LuaTask::RegisterForContent { content_id });
    }

    fn quest_sequence(&mut self, id: u32, sequence: u8) {
        self.queued_tasks
            .push(LuaTask::QuestSequence { id, sequence });
    }

    fn cancel_quest(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::CancelQuest { id });
    }

    fn incomplete_quest(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::IncompleteQuest { id });
    }

    fn kill(&mut self) {
        self.queued_tasks.push(LuaTask::Kill {});
    }

    fn set_online_status(&mut self, online_status_id: u8) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::SetStatusIcon {
                    icon: OnlineStatus::from_repr(online_status_id).unwrap_or_default(),
                },
            }));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn abandon_content(&mut self) {
        self.queued_tasks.push(LuaTask::AbandonContent {});
    }

    fn set_item_level(&mut self, item_level: u32) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::SetItemLevel { level: item_level },
            }));

        create_ipc_self(self, ipc, self.player_data.character.actor_id);
    }

    fn set_homepoint(&mut self, homepoint: u16) {
        self.queued_tasks.push(LuaTask::SetHomepoint { homepoint });
    }

    fn return_to_homepoint(&mut self) {
        self.queued_tasks.push(LuaTask::ReturnToHomepoint {});
    }

    fn has_aetheryte(&self, aetheryte_id: u32) -> bool {
        self.player_data.aetheryte.unlocked.contains(aetheryte_id)
    }

    fn join_content(&mut self, id: u32) {
        self.queued_tasks.push(LuaTask::JoinContent { id });
    }

    fn finish_casting_glamour(&mut self) {
        self.queued_tasks.push(LuaTask::FinishCastingGlamour {});
    }
}

impl UserData for LuaPlayer {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "send_message",
            |lua, this, (message, param): (String, Value)| {
                let param: u8 = lua.from_value(param).unwrap_or(0);
                this.send_message(&message, param);
                Ok(())
            },
        );
        methods.add_method_mut(
            "gain_effect",
            |_, this, (effect_id, param, duration): (u16, u16, f32)| {
                this.give_status_effect(effect_id, param, duration);
                Ok(())
            },
        );
        methods.add_method_mut(
            "play_scene",
            |_,
             this,
             (target, event_id, scene, scene_flags, params): (
                ObjectTypeId,
                u32,
                u16,
                u32,
                Vec<u32>,
            )| {
                this.play_scene(
                    target,
                    event_id,
                    scene,
                    SceneFlags::from_bits(scene_flags).unwrap_or_default(),
                    params,
                );
                Ok(())
            },
        );
        methods.add_method_mut(
            "set_position",
            |lua, this, (position, rotation): (Value, Value)| {
                let position: Position = lua.from_value(position).unwrap();
                let rotation: f32 = lua.from_value(rotation).unwrap();
                this.set_position(position, rotation);
                Ok(())
            },
        );
        methods.add_method_mut(
            "set_festival",
            |_, this, (festival1, festival2, festival3, festival4): (u32, u32, u32, u32)| {
                this.set_festival(festival1, festival2, festival3, festival4);
                Ok(())
            },
        );
        methods.add_method_mut("unlock_aetheryte", |_, this, (unlock, id): (u32, u32)| {
            this.unlock_aetheryte(unlock, id);
            Ok(())
        });
        methods.add_method_mut("unlock", |_, this, action_id: u32| {
            this.unlock(action_id);
            Ok(())
        });
        methods.add_method_mut("set_speed", |_, this, speed: u16| {
            this.set_speed(speed);
            Ok(())
        });
        methods.add_method_mut("toggle_wireframe", |_, this, _: Value| {
            this.toggle_wireframe();
            Ok(())
        });
        methods.add_method_mut("toggle_invisibility", |_, this, _: Value| {
            this.toggle_invisiblity();
            Ok(())
        });
        methods.add_method_mut(
            "change_territory",
            |lua, this, (zone_id, exit_position, exit_rotation): (u16, Value, Value)| {
                this.change_territory(
                    zone_id,
                    lua.from_value(exit_position).unwrap_or_default(),
                    lua.from_value(exit_rotation).unwrap_or_default(),
                );
                Ok(())
            },
        );
        methods.add_method_mut("set_remake_mode", |lua, this, mode: Value| {
            let mode: RemakeMode = lua.from_value(mode).unwrap();
            this.set_remake_mode(mode);
            Ok(())
        });
        methods.add_method_mut("warp", |_, this, warp_id: u32| {
            this.warp(warp_id);
            Ok(())
        });
        methods.add_method_mut("begin_log_out", |_, this, _: ()| {
            this.begin_log_out();
            Ok(())
        });
        methods.add_method_mut("finish_event", |_, this, handler_id: u32| {
            this.finish_event(handler_id);
            Ok(())
        });
        methods.add_method_mut("unlock_classjob", |_, this, classjob_id: u8| {
            this.unlock_classjob(classjob_id);
            Ok(())
        });
        methods.add_method_mut("warp_aetheryte", |_, this, aetheryte_id: u32| {
            this.warp_aetheryte(aetheryte_id);
            Ok(())
        });
        methods.add_method_mut("reload_scripts", |_, this, _: ()| {
            this.reload_scripts();
            Ok(())
        });
        methods.add_method_mut("set_level", |_, this, level: u16| {
            this.set_level(level);
            Ok(())
        });
        methods.add_method_mut("change_weather", |_, this, id: u16| {
            this.change_weather(id);
            Ok(())
        });
        methods.add_method_mut(
            "modify_currency",
            |_, this, (id, amount): (CurrencyKind, i32)| {
                this.modify_currency(id, amount, true);
                Ok(())
            },
        );
        methods.add_method_mut("gm_set_orchestrion", |_, this, (value, id): (bool, u32)| {
            this.gm_set_orchestrion(value, id);
            Ok(())
        });
        methods.add_method_mut("toggle_orchestrion", |_, this, id: u32| {
            this.toggle_orchestrion(id);
            Ok(())
        });
        methods.add_method_mut("add_item", |_, this, (id, quantity): (u32, u32)| {
            // Can't think of any situations where we wouldn't want to force a client inventory update after using debug commands.
            this.add_item(id, quantity, true);
            Ok(())
        });
        methods.add_method_mut("unlock_content", |_, this, id: u16| {
            this.unlock_content(id);
            Ok(())
        });
        methods.add_method_mut("unlock_all_content", |_, this, _: ()| {
            this.unlock_all_content();
            Ok(())
        });
        methods.add_method_mut(
            "get_buyback_list",
            |_, this, (shop_id, shop_intro): (u32, bool)| {
                Ok(this.get_buyback_list(shop_id, shop_intro))
            },
        );
        methods.add_method_mut(
            "do_gilshop_buyback",
            |_, this, (shop_id, buyback_index): (u32, u32)| {
                this.do_gilshop_buyback(shop_id, buyback_index);
                Ok(())
            },
        );
        methods.add_method_mut("add_exp", |_, this, amount: i32| {
            this.add_exp(amount);
            Ok(())
        });
        methods.add_method_mut(
            "start_event",
            |_, this, (target, event_id, event_type, event_arg): (ObjectTypeId, u32, u8, u32)| {
                this.start_event(
                    target,
                    event_id,
                    EventType::from_repr(event_type).unwrap(),
                    event_arg,
                );
                Ok(())
            },
        );
        methods.add_method_mut("set_inn_wakeup", |_, this, watched: bool| {
            this.set_inn_wakeup(watched);
            Ok(())
        });
        methods.add_method_mut(
            "do_solnine_teleporter",
            |_,
             this,
             (event_id, path_id, unk2, unk3, speed, unk4, unk5): (
                u32,
                u32,
                u16,
                u16,
                u16,
                u16,
                u32,
            )| {
                this.do_solnine_teleporter(event_id, path_id, unk2, unk3, speed, unk4, unk5);
                Ok(())
            },
        );
        methods.add_method_mut("toggle_mount", |_, this, id: u32| {
            this.toggle_mount(id);
            Ok(())
        });
        methods.add_method_mut("toggle_glasses_style", |_, this, id: u32| {
            this.toggle_glasses_style(id);
            Ok(())
        });
        methods.add_method_mut("toggle_glasses_style_all", |_, this, _: ()| {
            this.toggle_glasses_style_all();
            Ok(())
        });
        methods.add_method_mut("toggle_ornament", |_, this, id: u32| {
            this.toggle_ornament(id);
            Ok(())
        });
        methods.add_method_mut("toggle_ornament_all", |_, this, _: ()| {
            this.toggle_ornament_all();
            Ok(())
        });
        methods.add_method_mut("unlock_buddy_equip", |_, this, id: u32| {
            this.unlock_buddy_equip(id);
            Ok(())
        });
        methods.add_method_mut("unlock_buddy_equip_all", |_, this, _: ()| {
            this.unlock_buddy_equip_all();
            Ok(())
        });
        methods.add_method_mut("toggle_chocobo_taxi_stand", |_, this, id: u32| {
            this.toggle_chocobo_taxi_stand(id);
            Ok(())
        });
        methods.add_method_mut("toggle_chocobo_taxi_stand_all", |_, this, _: ()| {
            this.toggle_chocobo_taxi_stand_all();
            Ok(())
        });
        methods.add_method_mut("toggle_caught_fish", |_, this, id: u32| {
            this.toggle_caught_fish(id);
            Ok(())
        });
        methods.add_method_mut("toggle_caught_fish_all", |_, this, _: ()| {
            this.toggle_caught_fish_all();
            Ok(())
        });
        methods.add_method_mut("toggle_caught_spearfish", |_, this, id: u32| {
            this.toggle_caught_spearfish(id);
            Ok(())
        });
        methods.add_method_mut("toggle_caught_spearfish_all", |_, this, _: ()| {
            this.toggle_caught_spearfish_all();
            Ok(())
        });
        methods.add_method_mut("toggle_triple_triad_card", |_, this, id: u32| {
            this.toggle_triple_triad_card(id);
            Ok(())
        });
        methods.add_method_mut("toggle_triple_triad_card_all", |_, this, _: ()| {
            this.toggle_triple_triad_card_all();
            Ok(())
        });
        methods.add_method_mut("toggle_adventure", |_, this, id: u32| {
            this.toggle_adventure(id);
            Ok(())
        });
        methods.add_method_mut("toggle_adventure_all", |_, this, _: ()| {
            this.toggle_adventure_all();
            Ok(())
        });
        methods.add_method_mut("toggle_cutscene_seen", |_, this, id: u32| {
            this.toggle_cutscene_seen(id);
            Ok(())
        });
        methods.add_method_mut("toggle_cutscene_seen_all", |_, this, _: ()| {
            this.toggle_cutscene_seen_all();
            Ok(())
        });
        methods.add_method_mut("toggle_minion", |_, this, id: u32| {
            this.toggle_minion(id);
            Ok(())
        });
        methods.add_method_mut("toggle_minion_all", |_, this, _: ()| {
            this.toggle_minion_all();
            Ok(())
        });
        methods.add_method_mut("toggle_aether_current", |_, this, id: u32| {
            this.toggle_aether_current(id);
            Ok(())
        });
        methods.add_method_mut("toggle_aether_current_all", |_, this, _: ()| {
            this.toggle_aether_current_all();
            Ok(())
        });
        methods.add_method_mut("toggle_aether_current_comp_flg_set", |_, this, id: u32| {
            this.toggle_aether_current_comp_flg_set(id);
            Ok(())
        });
        methods.add_method_mut(
            "toggle_aether_current_comp_flg_set_all",
            |_, this, _: ()| {
                this.toggle_aether_current_comp_flg_set_all();
                Ok(())
            },
        );
        methods.add_method_mut(
            "move_to_pop_range",
            |lua, this, (id, fade_out): (u32, Value)| {
                let fade_out: bool = lua.from_value(fade_out).unwrap_or_default();
                this.move_to_pop_range(id, fade_out);
                Ok(())
            },
        );
        methods.add_method_mut("set_hp", |_, this, hp: u32| {
            this.set_hp(hp);
            Ok(())
        });
        methods.add_method_mut("set_mp", |_, this, mp: u16| {
            this.set_mp(mp);
            Ok(())
        });
        methods.add_method_mut("set_race", |_, this, race: u8| {
            this.set_race(race);
            Ok(())
        });
        methods.add_method_mut("set_tribe", |_, this, tribe: u8| {
            this.set_tribe(tribe);
            Ok(())
        });
        methods.add_method_mut("set_sex", |_, this, sex: u8| {
            this.set_sex(sex);
            Ok(())
        });
        methods.add_method("get_effect", |_, this, effect_id: u16| {
            Ok(this.status_effects.get(effect_id))
        });
        methods.add_method_mut("start_talk_event", |_, this, _: ()| {
            this.start_talk_event();
            Ok(())
        });
        methods.add_method_mut("accept_quest", |_, this, quest_id: u32| {
            this.accept_quest(quest_id);
            Ok(())
        });
        methods.add_method_mut("finish_quest", |_, this, quest_id: u32| {
            this.finish_quest(quest_id);
            Ok(())
        });
        methods.add_method_mut("prepare_zoning", |_, this, timeout: u8| {
            this.prepare_zoning(timeout);
            Ok(())
        });
        methods.add_method_mut("has_seen_cutscene", |_, this, cutscene_id: u32| {
            Ok(this.player_data.unlock.cutscene_seen.contains(cutscene_id))
        });
        methods.add_method_mut("commence_duty", |_, this, director_id: u32| {
            this.commence_duty(director_id);
            Ok(())
        });
        methods.add_method_mut(
            "get_switch_talk_target",
            |lua, this, switch_talk_target: u32| {
                Ok(this.get_switch_talk_target(
                    lua.globals().get("GAME_DATA").unwrap(),
                    switch_talk_target,
                ))
            },
        );
        methods.add_method_mut("register_for_content", |_, this, content_id: u16| {
            this.register_for_content(content_id);
            Ok(())
        });
        methods.add_method_mut(
            "quest_sequence",
            |_, this, (quest_id, sequence): (u32, u8)| {
                this.quest_sequence(quest_id, sequence);
                Ok(())
            },
        );
        methods.add_method_mut("cancel_quest", |_, this, quest_id: u32| {
            this.cancel_quest(quest_id);
            Ok(())
        });
        methods.add_method_mut("incomplete_quest", |_, this, quest_id: u32| {
            this.incomplete_quest(quest_id);
            Ok(())
        });
        methods.add_method_mut("kill", |_, this, _: ()| {
            this.kill();
            Ok(())
        });
        methods.add_method_mut("set_online_status", |_, this, online_status_id: u8| {
            this.set_online_status(online_status_id);
            Ok(())
        });
        methods.add_method_mut("abandon_content", |_, this, _: ()| {
            this.abandon_content();
            Ok(())
        });
        methods.add_method_mut("set_item_level", |_, this, item_level: u32| {
            this.set_item_level(item_level);
            Ok(())
        });
        methods.add_method_mut("set_homepoint", |_, this, homepoint: u16| {
            this.set_homepoint(homepoint);
            Ok(())
        });
        methods.add_method_mut("return_to_homepoint", |_, this, _: ()| {
            this.return_to_homepoint();
            Ok(())
        });
        methods.add_method("has_aetheryte", |_, this, aetheryte_id: u32| {
            Ok(this.has_aetheryte(aetheryte_id))
        });
        methods.add_method_mut("join_content", |_, this, id: u32| {
            this.join_content(id);
            Ok(())
        });
        methods.add_method_mut("finish_casting_glamour", |_, this, _: ()| {
            this.finish_casting_glamour();
            Ok(())
        });
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| {
            Ok(ObjectTypeId {
                object_id: this.player_data.character.actor_id,
                object_type: ObjectTypeKind::None,
            })
        });

        fields.add_field_method_get("teleport_query", |_, this| {
            Ok(this.player_data.teleport_query.clone())
        });
        fields.add_field_method_get("rotation", |_, this| Ok(this.player_data.volatile.rotation));
        fields.add_field_method_get("position", |_, this| Ok(this.player_data.volatile.position));
        fields.add_field_method_get("inventory", |_, this| {
            Ok(this.player_data.inventory.clone())
        });
        fields.add_field_method_get("zone", |_, this| Ok(this.zone_data.clone()));
        // Helper method to reduce the amount of typing for gil
        fields.add_field_method_get("gil", |_, this| {
            Ok(this.player_data.inventory.currency.gil.quantity)
        });
        fields.add_field_method_get("saw_inn_wakeup", |_, this| {
            Ok(this.player_data.saw_inn_wakeup)
        });
        fields.add_field_method_get("city_state", |_, this| Ok(this.player_data.city_state));
        fields.add_field_method_get("content", |_, this| Ok(this.content_data));
    }
}
