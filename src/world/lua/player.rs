use mlua::{LuaSerdeExt, UserData, UserDataFields, UserDataMethods, Value};

use crate::{
    INVENTORY_ACTION_ACK_SHOP, LogMessageType,
    common::{
        INVALID_OBJECT_ID, ObjectId, ObjectTypeId, ObjectTypeKind, Position,
        workdefinitions::RemakeMode, write_quantized_rotation,
    },
    inventory::{ContainerType, CurrencyKind, Item},
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, EventScene, ServerZoneIpcData,
        ServerZoneIpcSegment, Warp,
    },
    packet::PacketSegment,
    world::{EventFinishType, PlayerData, StatusEffects},
};

use super::{LuaZone, QueueSegments, Task, create_ipc_self};

#[derive(Default)]
pub struct LuaPlayer {
    pub player_data: PlayerData,
    pub status_effects: StatusEffects,
    pub queued_segments: Vec<PacketSegment<ServerZoneIpcSegment>>,
    pub queued_tasks: Vec<Task>,
    pub zone_data: LuaZone,
}

impl QueueSegments for LuaPlayer {
    fn queue_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        self.queued_segments.push(segment);
    }
}

impl LuaPlayer {
    fn send_message(&mut self, message: &str, param: u8) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage {
            message: message.to_string(),
            param,
        });

        create_ipc_self(self, ipc, self.player_data.actor_id);
    }

    fn give_status_effect(&mut self, effect_id: u16, effect_param: u16, duration: f32) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::GainEffect {
                    effect_id: effect_id as u32,
                    param: effect_param as u32,
                    source_actor_id: INVALID_OBJECT_ID, // TODO: fill
                },
            }));
        create_ipc_self(self, ipc, self.player_data.actor_id);

        self.status_effects.add(effect_id, duration);
    }

    fn play_scene(
        &mut self,
        target: ObjectTypeId,
        event_id: u32,
        scene: u16,
        scene_flags: u32,
        params: Vec<u32>,
    ) {
        let scene = EventScene {
            actor_id: target,
            event_id,
            scene,
            scene_flags,
            params_count: params.len() as u8,
            params: params.clone(),
            ..Default::default()
        };

        if let Some(ipc) = scene.package_scene() {
            create_ipc_self(self, ipc, self.player_data.actor_id);
        } else {
            let error_message = "Unsupported amount of parameters in play_scene! This is likely a bug in your script! Cancelling event...".to_string();
            tracing::warn!(error_message);
            self.send_message(&error_message, 0);
            self.finish_event(event_id, 0, EventFinishType::Normal);
        }
    }

    fn set_position(&mut self, position: Position, rotation: f32) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Warp(Warp {
            dir: write_quantized_rotation(&rotation),
            position,
            ..Default::default()
        }));

        create_ipc_self(self, ipc, self.player_data.actor_id);
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

        create_ipc_self(self, ipc, self.player_data.actor_id);
    }

    fn unlock(&mut self, id: u32) {
        self.queued_tasks.push(Task::Unlock { id });
    }

    fn set_speed(&mut self, speed: u16) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::Flee { speed },
            }));

        create_ipc_self(self, ipc, self.player_data.actor_id);
    }

    fn toggle_wireframe(&mut self) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::ToggleWireframeRendering(),
            }));

        create_ipc_self(self, ipc, self.player_data.actor_id);
    }

    fn unlock_aetheryte(&mut self, unlocked: u32, id: u32) {
        self.queued_tasks.push(Task::UnlockAetheryte {
            id,
            on: unlocked == 1,
        });
    }

    fn change_territory(&mut self, zone_id: u16) {
        self.queued_tasks.push(Task::ChangeTerritory { zone_id });
    }

    fn set_remake_mode(&mut self, mode: RemakeMode) {
        self.queued_tasks.push(Task::SetRemakeMode(mode));
    }

    fn warp(&mut self, warp_id: u32) {
        self.queued_tasks.push(Task::Warp { warp_id });
    }

    fn begin_log_out(&mut self) {
        self.queued_tasks.push(Task::BeginLogOut);
    }

    fn finish_event(&mut self, handler_id: u32, arg: u32, finish_type: EventFinishType) {
        self.queued_tasks.push(Task::FinishEvent {
            handler_id,
            arg,
            finish_type,
        });
    }

    fn set_classjob(&mut self, classjob_id: u8) {
        self.queued_tasks.push(Task::SetClassJob { classjob_id });
    }

    fn warp_aetheryte(&mut self, aetheryte_id: u32) {
        self.queued_tasks.push(Task::WarpAetheryte { aetheryte_id });
    }

    fn reload_scripts(&mut self) {
        self.queued_tasks.push(Task::ReloadScripts);
    }

    fn toggle_invisiblity(&mut self) {
        self.queued_tasks.push(Task::ToggleInvisibility {
            invisible: !self.player_data.gm_invisible,
        });
    }

    fn set_level(&mut self, level: i32) {
        self.queued_tasks.push(Task::SetLevel { level });
    }

    fn change_weather(&mut self, id: u16) {
        self.queued_tasks.push(Task::ChangeWeather { id });
    }

    fn add_gil(&mut self, amount: u32) {
        self.queued_tasks.push(Task::AddGil { amount });
    }

    fn remove_gil(&mut self, amount: u32, send_client_update: bool) {
        self.queued_tasks.push(Task::RemoveGil {
            amount,
            send_client_update,
        });
    }

    fn toggle_orchestrion(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleOrchestrion {
            id,
        });
    }

    fn add_item(&mut self, id: u32, quantity: u32, send_client_update: bool) {
        self.queued_tasks.push(Task::AddItem {
            id,
            quantity,
            send_client_update,
        });
    }

    fn complete_all_quests(&mut self) {
        self.queued_tasks.push(Task::CompleteAllQuests {});
    }

    fn unlock_content(&mut self, id: u16) {
        self.queued_tasks.push(Task::UnlockContent { id });
    }

    fn get_buyback_list(&mut self, shop_id: u32, shop_intro: bool) -> Vec<u32> {
        let ret = self
            .player_data
            .buyback_list
            .as_scene_params(shop_id, shop_intro);
        if !shop_intro {
            self.queued_tasks.push(Task::UpdateBuyBackList {
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
        self.remove_gil(cost, false);

        let shop_packets_to_send = [
            ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                sequence: self.player_data.shop_sequence,
                dst_storage_id: ContainerType::Currency as u16,
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
                dst_storage_id: item_dst_info.container as u16,
                dst_container_index: item_dst_info.index,
                dst_stack: item_dst_info.quantity,
                dst_catalog_id: bb_item.id,
                unk1: 0x7530_0000,
            }),
            ServerZoneIpcSegment::new(ServerZoneIpcData::ShopLogMessage {
                event_id: shop_id,
                message_type: LogMessageType::ItemBoughtBack as u32,
                params_count: 3,
                item_id: bb_item.id,
                item_quantity: item_dst_info.quantity,
                total_sale_cost: item_dst_info.quantity * bb_item.price_low,
            }),
        ];

        // Finally, queue up the packets required to make the magic happen.
        for ipc in shop_packets_to_send {
            create_ipc_self(self, ipc, self.player_data.actor_id);
        }
    }

    fn do_solnine_teleporter(
        &mut self,
        event_id: u32,
        unk1: u32,
        unk2: u16,
        unk3: u32,
        unk4: u32,
        unk5: u32,
    ) {
        let packets_to_send = [
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::EventRelatedUnk3 { event_id },
            })),
            ServerZoneIpcSegment::new(ServerZoneIpcData::WalkInEvent {
                unk1,
                unk2,
                unk3,
                unk4,
                unk5,
            }),
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::WalkInTriggerRelatedUnk3 {
                    unk1: 1, // Sometimes the server sends 2 for this, but it's still completely unknown what it means.
                },
            })),
            ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::WalkInTriggerRelatedUnk1 { unk1: 1 },
            })),
        ];

        for ipc in packets_to_send {
            create_ipc_self(self, ipc, self.player_data.actor_id);
        }
    }

    fn add_exp(&mut self, amount: u32) {
        self.queued_tasks.push(Task::AddExp { amount });
    }

    fn start_event(
        &mut self,
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: u8,
        event_arg: u32,
    ) {
        self.queued_tasks.push(Task::StartEvent {
            actor_id,
            event_id,
            event_type,
            event_arg,
        });
    }

    fn set_inn_wakeup(&mut self, watched: bool) {
        self.queued_tasks.push(Task::SetInnWakeup { watched });
    }

    fn toggle_mount(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleMount { id });
    }

    fn toggle_glasses_style(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleGlassesStyle { id });
    }

    fn toggle_glasses_style_all(&mut self) {
        self.queued_tasks.push(Task::ToggleGlassesStyleAll {});
    }

    fn toggle_ornament(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleOrnament { id });
    }

    fn toggle_ornament_all(&mut self) {
        self.queued_tasks.push(Task::ToggleOrnamentAll {});
    }

    fn unlock_buddy_equip(&mut self, id: u32) {
        self.queued_tasks.push(Task::UnlockBuddyEquip { id });
    }

    fn unlock_buddy_equip_all(&mut self) {
        self.queued_tasks.push(Task::UnlockBuddyEquipAll {});
    }

    fn toggle_chocobo_taxi_stand(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleChocoboTaxiStand { id });
    }

    fn toggle_chocobo_taxi_stand_all(&mut self) {
        self.queued_tasks.push(Task::ToggleChocoboTaxiStandAll {});
    }

    fn toggle_caught_fish(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleCaughtFish { id });
    }

    fn toggle_caught_fish_all(&mut self) {
        self.queued_tasks.push(Task::ToggleCaughtFishAll {});
    }

    fn toggle_caught_spearfish(&mut self, id: u32) {
        self.queued_tasks.push(Task::ToggleCaughtSpearfish { id });
    }

    fn toggle_caught_spearfish_all(&mut self) {
        self.queued_tasks.push(Task::ToggleCaughtSpearfishAll {});
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
                this.play_scene(target, event_id, scene, scene_flags, params);
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
        methods.add_method_mut("change_territory", |_, this, zone_id: u16| {
            this.change_territory(zone_id);
            Ok(())
        });
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
        methods.add_method_mut(
            "finish_event",
            |lua, this, (handler_id, arg, finish_type): (u32, u32, Value)| {
                // It's desirable for finish_type to be optional since we do normal finishes 99% of the time.
                let finish_type: u32 = lua.from_value(finish_type).unwrap_or(0);
                let finish_type = match finish_type {
                    0 => EventFinishType::Normal,
                    1 => EventFinishType::Jumping,
                    _ => EventFinishType::Normal,
                };
                this.finish_event(handler_id, arg, finish_type);
                Ok(())
            },
        );
        methods.add_method_mut("set_classjob", |_, this, classjob_id: u8| {
            this.set_classjob(classjob_id);
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
        methods.add_method_mut("set_level", |_, this, level: i32| {
            this.set_level(level);
            Ok(())
        });
        methods.add_method_mut("change_weather", |_, this, id: u16| {
            this.change_weather(id);
            Ok(())
        });
        methods.add_method_mut("add_gil", |_, this, amount: u32| {
            this.add_gil(amount);
            Ok(())
        });
        methods.add_method_mut("remove_gil", |_, this, amount: u32| {
            // Can't think of any situations where we wouldn't want to force a client currency update after using debug commands.
            this.remove_gil(amount, true);
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
        methods.add_method_mut("complete_all_quests", |_, this, _: ()| {
            this.complete_all_quests();
            Ok(())
        });
        methods.add_method_mut("unlock_content", |_, this, id: u16| {
            this.unlock_content(id);
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
        methods.add_method_mut("add_exp", |_, this, amount: u32| {
            this.add_exp(amount);
            Ok(())
        });
        methods.add_method_mut(
            "start_event",
            |_, this, (target, event_id, event_type, event_arg): (ObjectTypeId, u32, u8, u32)| {
                this.start_event(target, event_id, event_type, event_arg);
                Ok(())
            },
        );
        methods.add_method_mut("set_inn_wakeup", |_, this, watched: bool| {
            this.set_inn_wakeup(watched);
            Ok(())
        });
        methods.add_method_mut(
            "do_solnine_teleporter",
            |_, this, (event_id, unk1, unk2, unk3, unk4, unk5): (u32, u32, u16, u32, u32, u32)| {
                this.do_solnine_teleporter(event_id, unk1, unk2, unk3, unk4, unk5);
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
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| {
            Ok(ObjectTypeId {
                object_id: ObjectId(this.player_data.actor_id),
                object_type: ObjectTypeKind::None,
            })
        });

        fields.add_field_method_get("teleport_query", |_, this| {
            Ok(this.player_data.teleport_query.clone())
        });
        fields.add_field_method_get("rotation", |_, this| Ok(this.player_data.rotation));
        fields.add_field_method_get("position", |_, this| Ok(this.player_data.position));
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
    }
}
