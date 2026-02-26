//! Managing the inventory, equipped model IDs and shops.

use crate::{
    ItemInfoQuery, ToServer, ZoneConnection,
    inventory::{Item, Storage},
};
use kawari::{
    common::{ContainerType, ItemOperationKind, ObjectId},
    ipc::zone::{
        ActorControlCategory, ContainerInfo, CurrencyInfo, Equip, ItemInfo, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};

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
        let mut last_sequence = 0;
        for (sequence, (container_type, container)) in (&self.player_data.inventory.clone())
            .into_iter()
            .enumerate()
        {
            let mut num_items = 0;

            if container_type == ContainerType::Currency
                || container_type == ContainerType::Crystals
            {
                // currencies
                let mut send_currency = async |slot: u16, item: &Item| {
                    // skip telling the client what they don't have
                    if item.quantity == 0 || item.id == 0 {
                        return;
                    }

                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CurrencyCrystalInfo(
                        CurrencyInfo {
                            sequence: sequence as u32,
                            container: container_type,
                            quantity: item.quantity,
                            catalog_id: item.id,
                            slot,
                            ..Default::default()
                        },
                    ));
                    self.send_ipc_self(ipc).await;

                    num_items += 1;
                };

                for i in 0..container.max_slots() {
                    send_currency(i as u16, container.get_slot(i as u16)).await;
                }
            } else {
                // items
                let mut send_slot = async |slot: u16, item: &Item| {
                    // skip telling the client what they don't have
                    if item.quantity == 0 || item.id == 0 {
                        return;
                    }

                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateItem(ItemInfo {
                        sequence: sequence as u32,
                        container: container_type,
                        slot,
                        quantity: item.quantity,
                        catalog_id: item.id,
                        condition: item.condition,
                        glamour_catalog_id: item.glamour_catalog_id,
                        ..Default::default()
                    }));
                    self.send_ipc_self(ipc).await;

                    num_items += 1;
                };

                for i in 0..container.max_slots() {
                    send_slot(i as u16, container.get_slot(i as u16)).await;
                }
            }

            // inform the client of container state
            {
                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
                        container: container_type,
                        num_items,
                        sequence: sequence as u32,
                        ..Default::default()
                    }));
                self.send_ipc_self(ipc).await;
            }

            last_sequence = sequence;
        }

        let mut sequence = last_sequence + 1;

        // dummy container states that are not implemented
        // inform the client of container state
        for container_type in [
            ContainerType::Mail,
            ContainerType::Unk2,
            ContainerType::ArmoryWaist,
        ] {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
                sequence: sequence as u32,
                num_items: 0,
                container: container_type,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
            sequence += 1;
        }
    }

    /// Sends the updateitem and containerinfo packets for the equipped container.
    pub async fn send_equipped_inventory(&mut self) {
        let equipped = self.player_data.inventory.equipped;

        let mut num_items = 0;

        let mut send_slot = async |slot_index: u16, item: &Item| {
            if item.quantity == 0 || item.id == 0 {
                return;
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateItem(ItemInfo {
                sequence: self.player_data.item_sequence,
                container: ContainerType::Equipped,
                slot: slot_index,
                quantity: item.quantity,
                catalog_id: item.id,
                condition: item.condition,
                glamour_catalog_id: item.glamour_catalog_id,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;

            num_items += 1;
        };

        for i in 0..equipped.max_slots() {
            send_slot(i as u16, equipped.get_slot(i as u16)).await;
        }

        // inform the client of container state
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
                container: ContainerType::Equipped,
                num_items,
                sequence: self.player_data.item_sequence,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
        }
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
            self.actor_control_self(ActorControlCategory::SetItemLevel {
                level: self.player_data.inventory.equipped.calculate_item_level() as u32,
            })
            .await;
            // This seems to be pattern/crest related, it's seen when (un)equipping stuff.
            self.actor_control_self(ActorControlCategory::Unknown {
                category: 57,
                param1: 0,
                param2: 0,
                param3: 0,
                param4: 0,
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
            sequence: self.transaction_sequence,
            sequence_repeat: self.transaction_sequence,
            unk1,
            unk2,
        });
        self.send_ipc_self(ipc).await;
        self.transaction_sequence += 1;
    }

    /// Swaps two items from two (possibly different) containers and informs the client of this change.
    pub async fn swap_items(
        &mut self,
        src_container: ContainerType,
        src_index: u16,
        dst_container: ContainerType,
        dst_index: u16,
    ) {
        let src_item = self
            .player_data
            .inventory
            .get_item(src_container, src_index);

        // move src item into dst slot
        let dst_slot = self
            .player_data
            .inventory
            .get_item_mut(dst_container, dst_index);

        let dst_item = *dst_slot;
        let was_empty = dst_item.quantity == 0;
        dst_slot.clone_from(&src_item);

        // move dst item into src slot
        if src_container != ContainerType::Invalid {
            let src_slot = self
                .player_data
                .inventory
                .get_item_mut(src_container, src_index);
            src_slot.clone_from(&dst_item);
        }

        // Then inform the client of the updated slots, we have to do this since this is caused server-side.
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                sequence: self.player_data.item_sequence,
                dst_storage_id: src_container,
                dst_container_index: src_index,
                dst_stack: dst_item.quantity,
                dst_catalog_id: dst_item.id,
                unk1: 1966080000,
            });
            self.send_ipc_self(ipc).await;
            self.player_data.item_sequence += 1;
        }

        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                sequence: self.player_data.item_sequence,
                dst_storage_id: dst_container,
                dst_container_index: dst_index,
                dst_stack: src_item.quantity,
                dst_catalog_id: src_item.id,
                unk1: 1966080000,
            });
            self.send_ipc_self(ipc).await;
            self.player_data.item_sequence += 1;
        }

        // And also update the current transaction
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
            sequence: self.transaction_sequence,
            operation_type: if was_empty {
                ItemOperationKind::Move
            } else {
                ItemOperationKind::Exchange
            },

            src_actor_id: self.player_data.character.actor_id,
            src_storage_id: src_container,
            src_container_index: src_index,
            src_stack: src_item.quantity,
            src_catalog_id: src_item.id,

            dst_actor_id: self.player_data.character.actor_id,
            dst_storage_id: dst_container,
            dst_container_index: dst_index,
            dst_stack: dst_item.quantity,
            dst_catalog_id: dst_item.id,

            dummy_container: ContainerType::Equipped,
        });
        self.send_ipc_self(ipc).await;
    }

    /// Changes the class based on the weapon equipped.
    // TODO: remove incompatible armor
    pub async fn change_class_based_on_weapon(&mut self) {
        // Check the weapon's compatible class jobs:
        let classjobs;
        {
            let mut game_data = self.gamedata.lock();

            let weapon = self.player_data.inventory.equipped.main_hand.id;
            let item_info = game_data
                .get_item_info(ItemInfoQuery::ById(weapon))
                .unwrap();
            classjobs = game_data.get_applicable_classjobs(item_info.classjob_category as u16);
        }

        self.player_data.classjob.current_class = classjobs.first().copied().unwrap() as i32;
        assert!(self.player_data.classjob.current_class != 0); // If this is 0, then something went seriously wrong.
        self.update_class_info().await;
    }

    /// Changes the class based on the soul crystal equipped.
    // TODO: remove incompatible armor
    pub async fn change_class_based_on_soul_crystal(&mut self) {
        // Then check the soul crystal:
        let soul_crystal = self.player_data.inventory.equipped.soul_crystal;
        if soul_crystal.quantity > 0 {
            let classjob_id;
            {
                let mut game_data = self.gamedata.lock();
                classjob_id = game_data.get_applicable_classjob(soul_crystal.id);
            }

            if let Some(classjob_id) = classjob_id {
                self.player_data.classjob.current_class = classjob_id as i32;
                assert!(self.player_data.classjob.current_class != 0); // If this is 0, then something went seriously wrong.

                self.update_class_info().await;
            }
        }
    }
}
