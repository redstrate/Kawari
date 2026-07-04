//! Managing the inventory, equipped model IDs and shops.

use crate::{
    ItemInfoQuery, ToServer, ZoneConnection,
    inventory::{DesiredHousingInventoryPages, EQUIP_RESTRICTED, Item, Storage, get_next_free_slot},
};
use kawari::{
    common::{ContainerType, ItemOperationKind, LegacyEquipmentModelId, ObjectId, WeaponModelId},
    ipc::zone::{
        ActorControlCategory, ContainerInfo, CurrencyInfo, Equip, ItemInfo, ItemOperation,
        ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

use physis::equipment::EquipSlot;
use strum::IntoEnumIterator;

const DYE_RESULT_LOG_MESSAGE: u32 = 0xBC77;

impl ZoneConnection {
    /// Inform other clients (including yourself) that you changed your equipped model ids.
    pub async fn inform_equip(&mut self) {
        let main_weapon_id;
        let sub_weapon_id;
        let model_ids;
        let second_model_stain_ids;
        {
            let mut game_data = self.gamedata.lock();
            let inventory = &self.player_data.inventory;

            main_weapon_id = inventory.get_main_weapon_id(&mut game_data);
            sub_weapon_id = inventory.get_sub_weapon_id(&mut game_data);
            model_ids = inventory.legacy_model_ids(&mut game_data);
            second_model_stain_ids = inventory.second_model_stain_ids();
        }

        self.handle
            .send(ToServer::Equip(
                self.player_data.character.actor_id,
                main_weapon_id,
                sub_weapon_id,
                model_ids,
                second_model_stain_ids,
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
        main_weapon_id: WeaponModelId,
        sub_weapon_id: WeaponModelId,
        models: [LegacyEquipmentModelId; 10],
        second_model_stain_ids: [u8; 10],
    ) {
        self.send_stats().await;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Equip(Equip {
            main_weapon_id,
            sub_weapon_id,
            classjob_id: self.player_data.classjob.current_class as u8,
            models,
            second_model_stain_ids,
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

    /// Sends the InventoryTransactionFinish that closes a transaction batch. `transaction_id` MUST
    /// be the same id used for the InventoryTransaction packets in this batch (see `swap_items`),
    /// otherwise the client can't correlate the finish and stays stuck "syncing with the server".
    pub async fn send_inventory_transaction_finish(
        &mut self,
        transaction_id: u32,
        unk1: u32,
        unk2: u32,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransactionFinish {
            sequence: transaction_id,
            sequence_repeat: transaction_id,
            unk1,
            unk2,
        });
        self.send_ipc_self(ipc).await;
    }

    /// Allocates a fresh InventoryTransaction batch id.
    pub fn next_transaction_id(&mut self) -> u32 {
        let id = self.player_data.transaction_sequence;
        self.player_data.transaction_sequence += 1;
        id
    }

    /// Equips a gearset: moves/swaps the requested items into the equipped container, informs the
    /// client, re-derives stats, and updates the active class/job.
    ///
    /// Shared by both the `EquipGearset` and `EquipGearset2` client packets — they carry the same
    /// `gearset_index`/`containers`/`indices` payload, so the equip behavior is identical. (The only
    /// difference is `EquipGearset2` appends an extra trailing blob whose purpose is still unknown.)
    pub async fn equip_gearset(
        &mut self,
        gearset_index: u32,
        containers: &[ContainerType; 14],
        indices: &[i16; 14],
    ) {
        // TODO: handle missing items, full inventory and such
        // One transaction id for the whole gearset batch: every InventoryTransaction
        // we emit below (via swap_items) and the closing InventoryTransactionFinish
        // share this id, matching retail. Without a shared id the client can't
        // correlate the finish and stays stuck "syncing with the server".
        let gearset_txid = self.next_transaction_id();
        for slot in 0..14 {
            let from_slot = indices[slot];
            let from_container = containers[slot];

            if from_container == ContainerType::Equipped {
                continue;
            }
            let mut from_item = Item::default();

            if from_slot != -1
                && let Some(the_item) = self
                    .player_data
                    .inventory
                    .get_item(from_container, from_slot as u16)
            {
                from_item = the_item;
            }

            let equipped_item = self.player_data.inventory.equipped.get_slot(slot as u16);

            if !from_item.is_empty_slot() && !equipped_item.is_empty_slot() {
                // Something is equipped here AND the gearset wants a different
                // item in this slot. Swap them: the new item goes to the equip
                // slot, the displaced item goes back into the NEW item's source
                // slot. This matches client behavior — the client expects the
                // displaced gear to land in the slot the new item came from, so
                // it stays in sync. (Routing the displaced item to some other
                // free armoury slot instead desyncs the client: it later tries to
                // move gear back into the now-occupied source slot and stalls with
                // "syncing with server".) The source slot is always in the same
                // armoury category as the equip slot, so this never scrambles
                // gear across categories.
                self.swap_items(
                    from_container,
                    from_slot as u16,
                    ContainerType::Equipped,
                    slot as u16,
                    gearset_txid,
                )
                .await;
            } else if !from_item.is_empty_slot() && equipped_item.is_empty_slot() {
                // If there is nothing equipped but a new item in that slot, we just have to move it.
                // TODO: be a little smarter about this maybe?
                self.swap_items(
                    from_container,
                    from_slot as u16,
                    ContainerType::Equipped,
                    slot as u16,
                    gearset_txid,
                )
                .await;
            } else if from_item.is_empty_slot() && !equipped_item.is_empty_slot() {
                // If there is something equipped but the slot is empty in the gearset, we have to move it somewhere.
                let target_container_type = ContainerType::from_equip_slot(slot as u8);

                if let Some(target_container) = self
                    .player_data
                    .inventory
                    .get_container(target_container_type)
                    && let Some(free_slot) = get_next_free_slot(target_container)
                {
                    self.swap_items(
                        ContainerType::Equipped,
                        slot as u16,
                        target_container_type,
                        free_slot,
                        gearset_txid,
                    )
                    .await;
                }
            }
        }

        // Inform the client that the gearset was successfully equipped.
        self.actor_control_self(ActorControlCategory::GearSetEquipped { gearset_index })
            .await;

        // Re-populate the runtime derived fields (item level, defense, base
        // params) on the newly-equipped items, so item level and stats are
        // correct even if a swapped item carried stale serde-skipped fields.
        {
            let mut game_data = self.gamedata.lock();
            self.player_data.inventory.prepare_equipped(&mut game_data);
        }

        // Close the transaction batch. The finish repeats the batch transaction
        // id and the retail-observed unk1=0xD1/unk2=0xD00.
        self.send_inventory_transaction_finish(gearset_txid, 0xD1, 0xD00)
            .await;

        // Retail also re-sends the equipped container
        self.send_equipped_inventory().await;
        self.inform_equip().await;

        // Change class as needed. If a soul crystal is equipped, the job is
        // defined by the crystal (e.g. a SMN gearset equips the SMN stone, and
        // the class must become SMN — NOT ACN, which is what the weapon alone
        // would resolve to since SMN/ACN share weapons). Fall back to the weapon
        // only when no crystal is equipped. This matches the manual ItemOperation
        // path.
        if self.player_data.inventory.equipped.soul_crystal.quantity > 0 {
            self.change_class_based_on_soul_crystal().await;
        } else {
            self.change_class_based_on_weapon().await;
        }

        // Then finally, resend stats.
        self.send_stats().await;
    }

    /// Swaps two items from two (possibly different) containers and informs the client of this change.
    ///
    /// `transaction_id` is the InventoryTransaction batch id. All swaps that belong to the same
    /// logical operation (e.g. a single gearset equip) must pass the SAME id, and the trailing
    /// `send_inventory_transaction_finish` must be given that same id — that's how the client
    /// correlates the finish with its transactions. (Retail confirms: every InventoryTransaction in
    /// a gearset batch shares one id, repeated in the finish.)
    pub async fn swap_items(
        &mut self,
        src_container: ContainerType,
        src_index: u16,
        dst_container: ContainerType,
        dst_index: u16,
        transaction_id: u32,
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

        // And also update the current transaction. This uses the shared transaction batch id, NOT
        // item_sequence, so the trailing InventoryTransactionFinish (which repeats this id) can be
        // correlated by the client.
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
            sequence: transaction_id,
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

            // If there's no weapon equipped (e.g. the player is mid-swap and the main hand slot is
            // momentarily empty), there's nothing to derive a class from. Keep the current class
            // rather than panicking on an empty classjob list.
            if weapon == 0 {
                return;
            }

            let Some(item_info) = game_data.get_item_info(ItemInfoQuery::ById(weapon)) else {
                tracing::warn!(
                    "No item info for equipped weapon {weapon}; keeping the current class."
                );
                return;
            };
            classjobs = game_data.get_applicable_classjobs(item_info.classjob_category as u16);
        }

        let Some(new_class) = classjobs.first().copied() else {
            // The weapon isn't tied to any class (e.g. a non-combat/glamour item). Keep the
            // current class instead of crashing.
            tracing::warn!(
                "Equipped weapon has no applicable class jobs; keeping the current class."
            );
            return;
        };

        // If the class didn't actually change, don't replay the change VFX/message/update. This
        // avoids the duplicate "job changed" message when a weapon swap is followed by the
        // automatic soul-crystal re-check that lands on the same job.
        if self.player_data.classjob.current_class == new_class as i32 {
            return;
        }

        self.player_data.classjob.current_class = new_class as i32;
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
                // Skip if the class is already correct, to avoid a redundant change VFX/message.
                if self.player_data.classjob.current_class == classjob_id as i32 {
                    return;
                }

                self.player_data.classjob.current_class = classjob_id as i32;
                assert!(self.player_data.classjob.current_class != 0); // If this is 0, then something went seriously wrong.

                self.update_class_info().await;
                self.finish_changing_class().await;
            }
        }
    }

    /// Removes armor that's incompatible with your current class.
    ///
    /// Returns `true` if anything was unequipped (in which case the caller should re-sync the
    /// affected containers to the client, since this happens server-side).
    pub async fn remove_incompatible_armor(&mut self, action: &ItemOperation) -> bool {
        // NOTE: This has to match client behavior exactly! As this happens client-side.

        // First, decide which slots to unequip. We hold the game data lock only for the lookups,
        // then drop it before mutating the inventory and sending packets.
        let mut slots_to_unequip: Vec<u16> = Vec::new();
        {
            let mut game_data = self.gamedata.lock();

            // First remove incompatible classjob gear.
            for slot in EquipSlot::iter() {
                // Skip slots that must never be auto-unequipped here:
                // - MainHand: the weapon *defines* the current class, so it's never "incompatible".
                //   Stripping it would desync (and can make the weapon vanish during a swap).
                // - SoulCrystal: the crystal IS what defines the job; reverting to a base class
                //   (e.g. SMN -> ACN) must not also strip the crystal.
                // - Waist: legacy slot with no model.
                if slot == EquipSlot::MainHand
                    || slot == EquipSlot::SoulCrystal
                    || slot == EquipSlot::Waist
                {
                    continue;
                }

                let item = self.player_data.inventory.equipped.get_slot(slot as u16);
                if item.quantity > 0 {
                    let classjob_category = game_data.get_item_classjobcategory(item.item_id);
                    let classjobs = game_data.get_applicable_classjobs(classjob_category as u16);
                    if !classjobs.contains(&(self.player_data.classjob.current_class as u8)) {
                        tracing::info!(
                            "Unequipping item in slot {slot:#?} because it's incompatible with the current class."
                        );
                        slots_to_unequip.push(slot as u16);
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
                            slots_to_unequip.push(slot as u16);
                        }
                        // Otherwise, we're equipping into a restricted slot, so remove the body instead and exit the loop.
                        else if action.dst_container_index == slot as u16 {
                            tracing::info!(
                                "Unequipping item in slot Body because it's incompatible with the current {slot:#?} armor."
                            );
                            slots_to_unequip.push(EquipSlot::Body as u16);
                            break;
                        }
                    }
                }
            }
        }

        if slots_to_unequip.is_empty() {
            return false;
        }

        // Perform the unequips and collect the armoury containers that received items, so we can
        // re-sync them (plus the Equipped container) to the client. Without this the character
        // window keeps showing the old gear even though it was moved server-side.
        let mut affected_containers: Vec<ContainerType> = vec![ContainerType::Equipped];
        for slot in slots_to_unequip {
            if self.player_data.inventory.unequip_equipment(slot) {
                let armoury = ContainerType::from_equip_slot(slot as u8);
                if !affected_containers.contains(&armoury) {
                    affected_containers.push(armoury);
                }
            }
        }

        for container_type in affected_containers {
            let container = self.player_data.inventory.clone();
            if let Some(storage) = container.get_container(container_type) {
                self.send_container(storage, container_type).await;
            }
        }

        true
    }
}
