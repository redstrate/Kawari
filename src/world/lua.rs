use crate::INVENTORY_ACTION_ACK_SHOP;
use crate::{
    LogMessageType,
    common::{
        INVALID_OBJECT_ID, ObjectId, ObjectTypeId, Position, timestamp_secs,
        workdefinitions::RemakeMode, write_quantized_rotation,
    },
    config::get_config,
    inventory::{
        BuyBackList, ContainerType, CurrencyKind, CurrencyStorage, EquippedStorage, GenericStorage,
        Inventory, Item,
    },
    ipc::zone::{
        ActionEffect, ActorControlCategory, ActorControlSelf, DamageElement, DamageKind,
        DamageType, EffectKind, EventScene, ServerZoneIpcData, ServerZoneIpcSegment, Warp,
    },
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentData, SegmentType},
    world::ExtraLuaState,
};
use mlua::{FromLua, Lua, LuaSerdeExt, UserData, UserDataFields, UserDataMethods, Value};

use super::{PlayerData, StatusEffects, Zone, connection::TeleportQuery};

#[derive(Clone)]
pub enum Task {
    ChangeTerritory {
        zone_id: u16,
    },
    SetRemakeMode(RemakeMode),
    Warp {
        warp_id: u32,
    },
    BeginLogOut,
    FinishEvent {
        handler_id: u32,
        arg: u32,
    },
    SetClassJob {
        classjob_id: u8,
    },
    WarpAetheryte {
        aetheryte_id: u32,
    },
    ReloadScripts,
    ToggleInvisibility {
        invisible: bool,
    },
    Unlock {
        id: u32,
    },
    UnlockAetheryte {
        id: u32,
        on: bool,
    },
    SetLevel {
        level: i32,
    },
    ChangeWeather {
        id: u16,
    },
    AddGil {
        amount: u32,
    },
    RemoveGil {
        amount: u32,
        send_client_update: bool,
    },
    UnlockOrchestrion {
        id: u16,
        on: bool,
    },
    AddItem {
        id: u32,
        quantity: u32,
        send_client_update: bool,
    },
    CompleteAllQuests {},
    UnlockContent {
        id: u16,
    },
    UpdateBuyBackList {
        list: BuyBackList,
    },
    AddExp {
        amount: u32,
    },
    StartEvent {
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: u8,
        event_arg: u32,
    },
}

#[derive(Default, Clone)]
pub struct LuaZone {
    pub zone_id: u16,
    pub weather_id: u16,
    pub internal_name: String,
    pub region_name: String,
    pub place_name: String,
    pub intended_use: u8,
}

impl UserData for LuaZone {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.zone_id));
        fields.add_field_method_get("weather_id", |_, this| Ok(this.weather_id));
        fields.add_field_method_get("internal_name", |_, this| Ok(this.internal_name.clone()));
        fields.add_field_method_get("region_name", |_, this| Ok(this.region_name.clone()));
        fields.add_field_method_get("place_name", |_, this| Ok(this.place_name.clone()));
        fields.add_field_method_get("intended_use", |_, this| Ok(this.intended_use));
    }
}

#[derive(Default)]
pub struct LuaPlayer {
    pub player_data: PlayerData,
    pub status_effects: StatusEffects,
    pub queued_segments: Vec<PacketSegment<ServerZoneIpcSegment>>,
    pub queued_tasks: Vec<Task>,
    pub zone_data: LuaZone,
}

impl LuaPlayer {
    fn queue_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        self.queued_segments.push(segment);
    }

    fn create_segment_target(
        &mut self,
        op_code: ServerZoneIpcType,
        data: ServerZoneIpcData,
        source_actor: u32,
        target_actor: u32,
    ) {
        let ipc = ServerZoneIpcSegment {
            op_code,
            timestamp: timestamp_secs(),
            data,
            ..Default::default()
        };

        self.queue_segment(PacketSegment {
            source_actor,
            target_actor,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        });
    }

    fn create_segment_self(&mut self, op_code: ServerZoneIpcType, data: ServerZoneIpcData) {
        self.create_segment_target(
            op_code,
            data,
            self.player_data.actor_id,
            self.player_data.actor_id,
        );
    }

    fn send_message(&mut self, message: &str, param: u8) {
        let op_code = ServerZoneIpcType::ServerChatMessage;
        let data = ServerZoneIpcData::ServerChatMessage {
            message: message.to_string(),
            param,
        };

        self.create_segment_self(op_code, data);
    }

    fn give_status_effect(&mut self, effect_id: u16, effect_param: u16, duration: f32) {
        let op_code = ServerZoneIpcType::ActorControlSelf;
        let data = ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
            category: ActorControlCategory::GainEffect {
                effect_id: effect_id as u32,
                param: effect_param as u32,
                source_actor_id: INVALID_OBJECT_ID, // TODO: fill
            },
        });
        self.create_segment_self(op_code, data);

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

        if let Some((op_code, data)) = scene.package_scene() {
            self.create_segment_self(op_code, data);
        } else {
            let error_message = "Unsupported amount of parameters in play_scene! This is likely a bug in your script! Cancelling event...".to_string();
            tracing::warn!(error_message);
            self.send_message(&error_message, 0);
            self.finish_event(event_id, 0);
        }
    }

    fn set_position(&mut self, position: Position, rotation: f32) {
        let op_code = ServerZoneIpcType::Warp;
        let data = ServerZoneIpcData::Warp(Warp {
            dir: write_quantized_rotation(&rotation),
            position,
            ..Default::default()
        });

        self.create_segment_self(op_code, data);
    }

    fn set_festival(&mut self, festival1: u32, festival2: u32, festival3: u32, festival4: u32) {
        let op_code = ServerZoneIpcType::ActorControlSelf;
        let data = ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
            category: ActorControlCategory::SetFestival {
                festival1,
                festival2,
                festival3,
                festival4,
            },
        });

        self.create_segment_self(op_code, data);
    }

    fn unlock(&mut self, id: u32) {
        self.queued_tasks.push(Task::Unlock { id });
    }

    fn set_speed(&mut self, speed: u16) {
        let op_code = ServerZoneIpcType::ActorControlSelf;
        let data = ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
            category: ActorControlCategory::Flee { speed },
        });

        self.create_segment_self(op_code, data);
    }

    fn toggle_wireframe(&mut self) {
        let op_code = ServerZoneIpcType::ActorControlSelf;
        let data = ServerZoneIpcData::ActorControlSelf(ActorControlSelf {
            category: ActorControlCategory::ToggleWireframeRendering(),
        });

        self.create_segment_self(op_code, data);
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

    fn finish_event(&mut self, handler_id: u32, arg: u32) {
        self.queued_tasks
            .push(Task::FinishEvent { handler_id, arg });
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

    fn unlock_orchestrion(&mut self, unlocked: u32, id: u16) {
        self.queued_tasks.push(Task::UnlockOrchestrion {
            id,
            on: unlocked == 1,
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
            (
                ServerZoneIpcType::UpdateInventorySlot,
                ServerZoneIpcData::UpdateInventorySlot {
                    sequence: self.player_data.shop_sequence,
                    dst_storage_id: ContainerType::Currency as u16,
                    dst_container_index: 0,
                    dst_stack: new_gil,
                    dst_catalog_id: CurrencyKind::Gil as u32,
                    unk1: 0x7530_0000,
                },
            ),
            (
                ServerZoneIpcType::InventoryActionAck,
                ServerZoneIpcData::InventoryActionAck {
                    sequence: u32::MAX,
                    action_type: INVENTORY_ACTION_ACK_SHOP as u16,
                },
            ),
            (
                ServerZoneIpcType::UpdateInventorySlot,
                ServerZoneIpcData::UpdateInventorySlot {
                    sequence: self.player_data.shop_sequence,
                    dst_storage_id: item_dst_info.container as u16,
                    dst_container_index: item_dst_info.index,
                    dst_stack: item_dst_info.quantity,
                    dst_catalog_id: bb_item.id,
                    unk1: 0x7530_0000,
                },
            ),
            (
                ServerZoneIpcType::ShopLogMessage,
                ServerZoneIpcData::ShopLogMessage {
                    event_id: shop_id,
                    message_type: LogMessageType::ItemBoughtBack as u32,
                    params_count: 3,
                    item_id: bb_item.id,
                    item_quantity: item_dst_info.quantity,
                    total_sale_cost: item_dst_info.quantity * bb_item.price_low,
                },
            ),
        ];

        // Finally, queue up the packets required to make the magic happen.
        for (op_code, data) in shop_packets_to_send {
            self.create_segment_self(op_code, data);
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
        methods.add_method_mut("finish_event", |_, this, (handler_id, arg): (u32, u32)| {
            this.finish_event(handler_id, arg);
            Ok(())
        });
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
        methods.add_method_mut("unlock_orchestrion", |_, this, (unlock, id): (u32, u16)| {
            this.unlock_orchestrion(unlock, id);
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
    }

    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| {
            Ok(ObjectTypeId {
                object_id: ObjectId(this.player_data.actor_id),
                object_type: 0,
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
    }
}

impl UserData for TeleportQuery {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("aetheryte_id", |_, this| Ok(this.aetheryte_id));
    }
}

impl UserData for Position {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.x));
        fields.add_field_method_get("y", |_, this| Ok(this.y));
        fields.add_field_method_get("z", |_, this| Ok(this.z));
    }
}

impl UserData for Inventory {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("equipped", |_, this| Ok(this.equipped));
        fields.add_field_method_get("pages", |_, this| Ok(this.pages.clone()));
        fields.add_field_method_get("armoury_main_hand", |_, this| {
            Ok(this.armoury_main_hand.clone())
        });
        fields.add_field_method_get("armoury_head", |_, this| Ok(this.armoury_head.clone()));
        fields.add_field_method_get("armoury_body", |_, this| Ok(this.armoury_body.clone()));
        fields.add_field_method_get("armoury_hands", |_, this| Ok(this.armoury_hands.clone()));
        fields.add_field_method_get("armoury_legs", |_, this| Ok(this.armoury_legs.clone()));
        fields.add_field_method_get("armoury_feet", |_, this| Ok(this.armoury_feet.clone()));
        fields.add_field_method_get("armoury_off_hand", |_, this| {
            Ok(this.armoury_off_hand.clone())
        });
        fields.add_field_method_get("armoury_earring", |_, this| {
            Ok(this.armoury_earring.clone())
        });
        fields.add_field_method_get("armoury_necklace", |_, this| {
            Ok(this.armoury_necklace.clone())
        });
        fields.add_field_method_get("armoury_bracelet", |_, this| Ok(this.armoury_body.clone()));
        fields.add_field_method_get("armoury_rings", |_, this| Ok(this.armoury_rings.clone()));
        fields.add_field_method_get("armoury_soul_crystal", |_, this| {
            Ok(this.armoury_soul_crystal.clone())
        });
        fields.add_field_method_get("currency", |_, this| Ok(this.currency));
    }
}

impl UserData for Item {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("quantity", |_, this| Ok(this.quantity));
        fields.add_field_method_get("id", |_, this| Ok(this.id));
        fields.add_field_method_get("condition", |_, this| Ok(this.condition));
        fields.add_field_method_get("glamour_catalog_id", |_, this| Ok(this.condition));
    }
}

impl UserData for CurrencyStorage {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("gil", |_, this| Ok(this.gil));
    }
}

impl<const N: usize> UserData for GenericStorage<N> {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("slots", |_, this| Ok(this.slots.clone()));
    }
}

impl UserData for EquippedStorage {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("main_hand", |_, this| Ok(this.main_hand));
        fields.add_field_method_get("off_hand", |_, this| Ok(this.off_hand));
        fields.add_field_method_get("head", |_, this| Ok(this.head));
        fields.add_field_method_get("body", |_, this| Ok(this.body));
        fields.add_field_method_get("hands", |_, this| Ok(this.hands));
        fields.add_field_method_get("belt", |_, this| Ok(this.belt));
        fields.add_field_method_get("legs", |_, this| Ok(this.legs));
        fields.add_field_method_get("feet", |_, this| Ok(this.feet));
        fields.add_field_method_get("ears", |_, this| Ok(this.ears));
        fields.add_field_method_get("neck", |_, this| Ok(this.neck));
        fields.add_field_method_get("wrists", |_, this| Ok(this.wrists));
        fields.add_field_method_get("right_ring", |_, this| Ok(this.right_ring));
        fields.add_field_method_get("left_ring", |_, this| Ok(this.left_ring));
        fields.add_field_method_get("soul_crystal", |_, this| Ok(this.soul_crystal));
    }
}

impl UserData for ObjectTypeId {}

impl FromLua for ObjectTypeId {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl UserData for Zone {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "get_pop_range",
            |lua: &Lua, this, id: u32| -> mlua::Result<mlua::Value> {
                if let Some(pop_range) = this.find_pop_range(id) {
                    let trans = pop_range.0.transform.translation;
                    return lua.pack(Position {
                        x: trans[0],
                        y: trans[1],
                        z: trans[2],
                    });
                } else {
                    tracing::warn!("Failed to find pop range for {id}!");
                }
                Ok(mlua::Nil)
            },
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct EffectsBuilder {
    pub effects: Vec<ActionEffect>,
}

impl UserData for EffectsBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("damage", |lua, this, (damage_kind, damage_type, damage_element, amount): (Value, Value, Value, u16)| {
            let damage_kind: DamageKind = lua.from_value(damage_kind).unwrap();
            let damage_type: DamageType = lua.from_value(damage_type).unwrap();
            let damage_element: DamageElement = lua.from_value(damage_element).unwrap();

            this.effects.push(ActionEffect {
                kind: EffectKind::Damage {
                    damage_kind,
                    damage_type,
                    damage_element,
                    bonus_percent: 0,
                    unk3: 0,
                    unk4: 0,
                    amount,
                },
            });
            Ok(())
        });
        methods.add_method_mut(
            "gain_effect",
            |_, this, (effect_id, param, duration): (u16, u16, f32)| {
                this.effects.push(ActionEffect {
                    kind: EffectKind::Unk1 {
                        unk1: 0,
                        unk2: 7728,
                        effect_id,
                        duration,
                        param,
                        source_actor_id: INVALID_OBJECT_ID,
                    },
                });
                Ok(())
            },
        );
    }
}

impl FromLua for EffectsBuilder {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.clone()),
            _ => unreachable!(),
        }
    }
}

/// Loads `Init.lua`
pub fn load_init_script(lua: &mut Lua) -> mlua::Result<()> {
    let register_action_func =
        lua.create_function(|lua, (action_id, action_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.action_scripts.insert(action_id, action_script);
            Ok(())
        })?;

    let register_event_func =
        lua.create_function(|lua, (event_id, event_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.event_scripts.insert(event_id, event_script);
            Ok(())
        })?;

    let register_command_func =
        lua.create_function(|lua, (command_name, command_script): (String, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.command_scripts.insert(command_name, command_script);
            Ok(())
        })?;

    let register_gm_command_func =
        lua.create_function(|lua, (command_type, command_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state
                .gm_command_scripts
                .insert(command_type, command_script);
            Ok(())
        })?;

    let register_effects_func =
        lua.create_function(|lua, (command_type, status_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.effect_scripts.insert(command_type, status_script);
            Ok(())
        })?;

    let get_login_message_func = lua.create_function(|_, _: ()| {
        let config = get_config();
        Ok(config.world.login_message)
    })?;

    lua.set_app_data(ExtraLuaState::default());
    lua.globals().set("registerAction", register_action_func)?;
    lua.globals().set("registerEvent", register_event_func)?;
    lua.globals()
        .set("registerCommand", register_command_func)?;
    lua.globals()
        .set("registerGMCommand", register_gm_command_func)?;
    lua.globals().set("registerEffect", register_effects_func)?;
    lua.globals()
        .set("getLoginMessage", get_login_message_func)?;

    let effectsbuilder_constructor = lua.create_function(|_, ()| Ok(EffectsBuilder::default()))?;
    lua.globals()
        .set("EffectsBuilder", effectsbuilder_constructor)?;

    let config = get_config();
    let file_name = format!("{}/Init.lua", &config.world.scripts_location);
    lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
        .set_name("@".to_string() + &file_name)
        .exec()?;

    Ok(())
}
