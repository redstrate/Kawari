//! Managing the inventory, equipped model IDs and shops.

use crate::{
    ItemInfoQuery, ToServer, ZoneConnection,
    inventory::{DesiredHousingInventoryPages, EQUIP_RESTRICTED, Storage},
};
use kawari::{
    common::{ContainerType, ItemOperationKind, ObjectId},
    ipc::zone::{
        ActorControlCategory, ContainerInfo, CurrencyInfo, Equip, ItemInfo, ItemOperation,
        ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

use physis::equipment::EquipSlot;
use strum::IntoEnumIterator;

impl ZoneConnection {
    /// Inform other clients (including yourself) that you changed your equipped model ids.
    pub async fn inform_equip(&mut self) {
        let main_weapon_id;
        let sub_weapon_id;
        let model_ids;
        {
            let mut game_data = self.gamedata.lock();
            let inventory = &self.player_data.inventory;

            main_weapon_id = inventory.get_main_weapon_id(&mut game_data);
            sub_weapon_id = inventory.get_sub_weapon_id(&mut game_data);
            model_ids = inventory.get_model_ids(&mut game_data);
        }

        self.handle
            .send(ToServer::Equip(
                self.id,
                self.player_data.character.actor_id,
                main_weapon_id,
                sub_weapon_id,
                model_ids,
            ))
            .await;
    }

    pub async fn send_inventory(&mut self) {
        for (container_type, container) in (&self.player_data.inventory.clone()).into_iter() {
            self.send_container(container, container_type).await;
        }

        // Inform the client of dummy container states that are not implemented
        for container_type in [
            ContainerType::Mail,
            ContainerType::Unk2,
            ContainerType::ArmoryWaist,
        ] {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
                sequence: self.player_data.item_sequence,
                num_items: 0,
                container: container_type,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
            self.player_data.item_sequence += 1;
        }
    }

    /// Sends the updateitem and containerinfo packets for the equipped container.
    pub async fn send_equipped_inventory(&mut self) {
        let equipped = self.player_data.inventory.equipped;

        self.send_container(&equipped, ContainerType::Equipped)
            .await;
    }

    pub async fn send_housing_inventory(&mut self, which: DesiredHousingInventoryPages) {
        let cloned_inv = match which {
            DesiredHousingInventoryPages::Interior => {
                self.player_data.house_inventory.interior.clone()
            }
            DesiredHousingInventoryPages::InteriorStoreroom => {
                self.player_data.house_inventory.interior_storeroom.clone()
            }
            DesiredHousingInventoryPages::Exterior => {
                self.player_data.house_inventory.exterior.clone()
            }
            DesiredHousingInventoryPages::ExteriorStoreroom => {
                self.player_data.house_inventory.exterior_storeroom.clone()
            }
            DesiredHousingInventoryPages::None => {
                return;
            }
        };

        for container in cloned_inv.into_iter() {
            self.send_container(&container, container.kind).await;
        }
    }

    pub async fn send_container(&mut self, container: &dyn Storage, container_type: ContainerType) {
        let mut num_items = 0;
        for slot_index in 0..container.max_slots() {
            let item = container.get_slot(slot_index as u16);
            // Don't tell the client about things they don't have
            if item.is_empty_slot() {
                continue;
            }

            let ipc = match container_type {
                ContainerType::Currency | ContainerType::Crystals => ServerZoneIpcSegment::new(
                    ServerZoneIpcData::CurrencyCrystalInfo(CurrencyInfo {
                        sequence: self.player_data.item_sequence,
                        container: container_type,
                        quantity: item.quantity,
                        catalog_id: item.item_id,
                        slot: slot_index as u16,
                        ..Default::default()
                    }),
                ),
                _ => ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateItem(ItemInfo {
                    sequence: self.player_data.item_sequence,
                    container: container_type,
                    slot: slot_index as u16,
                    ..(*item).into()
                })),
            };

            self.send_ipc_self(ipc).await;

            num_items += 1;
        }

        // Inform the client of container state
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
            container: container_type,
            num_items,
            sequence: self.player_data.item_sequence,
            ..Default::default()
        }));
        self.send_ipc_self(ipc).await;

        self.player_data.item_sequence += 1;
    }

    /// Sends two containers to the client, particularly helpful when doing various housing operations, such as moving an item from the main inventory directly to the storeroom.
    pub async fn send_affected_containers(
        &mut self,
        src_container_type: ContainerType,
        dst_container_type: ContainerType,
    ) {
        // This cloning is sort of ugly, but we run into numerous borrowing issues if we don't.
        let main_inventory = self.player_data.inventory.clone();
        let house_inventory = self.player_data.house_inventory.clone();

        let get_container = |kind: &ContainerType| -> Option<&dyn Storage> {
            if let Some(temp_container) = main_inventory.get_container(*kind) {
                return Some(temp_container);
            } else if let Some(temp_container) = house_inventory.get_container(*kind) {
                return Some(temp_container);
            }

            None
        };

        let Some(src_container) = get_container(&src_container_type) else {
            return;
        };

        let Some(dst_container) = get_container(&dst_container_type) else {
            return;
        };

        self.send_container(src_container, src_container_type).await;
        self.send_container(dst_container, dst_container_type).await;
    }

    pub async fn update_equip(
        &mut self,
        actor_id: ObjectId,
        main_weapon_id: u64,
        sub_weapon_id: u64,
        model_ids: [u32; 10],
    ) {
        self.send_stats().await;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Equip(Equip {
            main_weapon_id,
            sub_weapon_id,
            classjob_id: self.player_data.classjob.current_class as u8,
            model_ids,
            ..Default::default()
        }));

        self.send_ipc_from(actor_id, ipc).await;

        // TODO: get a capture of another player equipping stuff to see if we get this as well, but it seems unlikely.
        if self.player_data.character.actor_id == actor_id {
            let level;
            {
                let mut game_data = self.gamedata.lock();

                level = self
                    .player_data
                    .inventory
                    .equipped
                    .calculate_item_level(&mut game_data) as u32;
            }

            self.actor_control_self(ActorControlCategory::SetItemLevel { level })
                .await;

            // This seems to be pattern/crest related, it's seen when (un)equipping stuff.
            self.actor_control_self(ActorControlCategory::Unknown {
                category: 57,
                param1: 0,
                param2: 0,
                param3: 0,
                param4: 0,
                param5: 0,
            })
            .await;
        }

        self.send_stats().await;
        self.update_class_info().await;
    }

    pub async fn send_inventory_ack(&mut self, sequence: u32, action_type: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryActionAck {
            sequence,
            action_type,
        });
        self.send_ipc_self(ipc).await;
        self.player_data.item_sequence += 1;
    }

    pub async fn send_inventory_transaction_finish(&mut self, unk1: u32, unk2: u32) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransactionFinish {
            sequence: self.player_data.item_sequence,
            sequence_repeat: self.player_data.item_sequence,
            unk1,
            unk2,
        });
        self.send_ipc_self(ipc).await;
        self.player_data.item_sequence += 1;
    }

    /// Swaps two items from two (possibly different) containers and informs the client of this change.
    pub async fn swap_items(
        &mut self,
        src_container: ContainerType,
        src_index: u16,
        dst_container: ContainerType,
        dst_index: u16,
    ) {
        let Some(src_item) = self
            .player_data
            .inventory
            .get_item(src_container, src_index)
        else {
            tracing::warn!(
                "Unable to swap items: src_container was an invalid container for this operation: {src_container}!"
            );
            return;
        };

        // move src item into dst slot
        let Some(dst_slot) = self
            .player_data
            .inventory
            .get_item_mut(dst_container, dst_index)
        else {
            tracing::warn!(
                "Unable to swap items: dst_container was an invalid container for this operation: {dst_container}!"
            );
            return;
        };

        let dst_item = *dst_slot;
        let was_empty = dst_item.quantity == 0;
        dst_slot.clone_from(&src_item);

        // move dst item into src slot
        let src_slot = self
            .player_data
            .inventory
            .get_item_mut(src_container, src_index)
            .unwrap(); // This unwrap should be fine since we've reached this point.
        src_slot.clone_from(&dst_item);

        // Then inform the client of the updated slots, we have to do this since this is caused server-side.
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot(ItemInfo {
                sequence: self.player_data.item_sequence,
                container: src_container,
                slot: src_index,
                ..dst_item.into()
            }));
            self.send_ipc_self(ipc).await;
            self.player_data.item_sequence += 1;
        }

        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot(ItemInfo {
                sequence: self.player_data.item_sequence,
                container: dst_container,
                slot: dst_index,
                ..src_item.into()
            }));
            self.send_ipc_self(ipc).await;
            self.player_data.item_sequence += 1;
        }

        // And also update the current transaction
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
            sequence: self.player_data.item_sequence,
            operation_type: if was_empty {
                ItemOperationKind::Move
            } else {
                ItemOperationKind::Exchange
            },

            src_actor_id: self.player_data.character.actor_id,
            src_storage_id: src_container,
            src_container_index: src_index,
            src_stack: src_item.quantity,
            src_catalog_id: src_item.item_id,

            dst_actor_id: self.player_data.character.actor_id,
            dst_storage_id: dst_container,
            dst_container_index: dst_index,
            dst_stack: dst_item.quantity,
            dst_catalog_id: dst_item.item_id,

            dummy_container: ContainerType::Equipped,
        });
        self.send_ipc_self(ipc).await;
    }

    /// Changes the class based on the weapon equipped.
    pub async fn change_class_based_on_weapon(&mut self) {
        // Check the weapon's compatible class jobs:
        let classjobs;
        {
            let mut game_data = self.gamedata.lock();

            let weapon = self.player_data.inventory.equipped.main_hand.item_id;
            let item_info = game_data
                .get_item_info(ItemInfoQuery::ById(weapon))
                .unwrap();
            classjobs = game_data.get_applicable_classjobs(item_info.classjob_category as u16);
        }

        self.player_data.classjob.current_class = classjobs.first().copied().unwrap() as i32;
        assert!(self.player_data.classjob.current_class != 0); // If this is 0, then something went seriously wrong.

        self.update_class_info().await;
        self.finish_changing_class().await;
    }

    /// Changes the class based on the soul crystal equipped.
    pub async fn change_class_based_on_soul_crystal(&mut self) {
        // Then check the soul crystal:
        let soul_crystal = self.player_data.inventory.equipped.soul_crystal;
        if soul_crystal.quantity > 0 {
            let classjob_id;
            {
                let mut game_data = self.gamedata.lock();
                classjob_id = game_data.get_applicable_classjob(soul_crystal.item_id);
            }

            if let Some(classjob_id) = classjob_id {
                self.player_data.classjob.current_class = classjob_id as i32;
                assert!(self.player_data.classjob.current_class != 0); // If this is 0, then something went seriously wrong.

                self.update_class_info().await;
                self.finish_changing_class().await;
            }
        }
    }

    /// Removes armor that's incompatible with your current class.
    pub async fn remove_incompatible_armor(&mut self, action: &ItemOperation) {
        // NOTE: This has to match client behavior exactly! As this happens client-side.

        let mut game_data = self.gamedata.lock();

        // First remove incompatible classjob gear.
        for slot in EquipSlot::iter() {
            let item = self.player_data.inventory.equipped.get_slot(slot as u16);
            if item.quantity > 0 {
                let classjob_category = game_data.get_item_classjobcategory(item.item_id);
                let classjobs = game_data.get_applicable_classjobs(classjob_category as u16);
                if !classjobs.contains(&(self.player_data.classjob.current_class as u8)) {
                    tracing::info!(
                        "Unequipping item in slot {slot:#?} because it's incompatible with the current class."
                    );
                    self.player_data.inventory.unequip_equipment(slot as u16);
                }
            }
        }

        // Then unequip slots that are restricted by any body armor.
        let body_item = self
            .player_data
            .inventory
            .equipped
            .get_slot(EquipSlot::Body as u16);
        if body_item.quantity > 0
            && let Some(body_item_info) =
                game_data.get_item_info(ItemInfoQuery::ById(body_item.item_id))
        {
            let body_restrictions = [
                (EquipSlot::Head, body_item_info.equip_restrictions.head),
                (EquipSlot::Hands, body_item_info.equip_restrictions.hands),
                (EquipSlot::Legs, body_item_info.equip_restrictions.legs),
                (EquipSlot::Feet, body_item_info.equip_restrictions.feet),
            ];
            for (slot, restriction) in body_restrictions {
                if action.dst_storage_id == ContainerType::Equipped
                    && restriction == EQUIP_RESTRICTED
                {
                    // If body was equipped, remove this restricted gear.
                    if action.dst_container_index == EquipSlot::Body as u16 {
                        tracing::info!(
                            "Unequipping item in slot {slot:#?} because it's incompatible with the current body armor."
                        );
                        self.player_data.inventory.unequip_equipment(slot as u16);
                    }
                    // Otherwise, we're equipping into a restricted slot, so remove the body instead and exit the loop.
                    else if action.dst_container_index == slot as u16 {
                        tracing::info!(
                            "Unequipping item in slot Body because it's incompatible with the current {slot:#?} armor."
                        );
                        self.player_data
                            .inventory
                            .unequip_equipment(EquipSlot::Body as u16);
                        break;
                    }
                }
            }
        }
    }
}
