//! Managing the inventory, equipped model IDs and shops.

use crate::{
    LogMessageType,
    inventory::{ContainerType, Item},
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, ContainerInfo, CurrencyInfo, Equip, ItemInfo,
        ServerZoneIpcData, ServerZoneIpcSegment,
    },
    packet::{PacketSegment, SegmentData, SegmentType},
    world::{ToServer, ZoneConnection},
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
                self.player_data.actor_id,
                main_weapon_id,
                sub_weapon_id,
                model_ids,
            ))
            .await;
    }

    pub async fn send_inventory(&mut self, first_update: bool) {
        let mut last_sequence = 0;
        for (sequence, (container_type, container)) in (&self.player_data.inventory.clone())
            .into_iter()
            .enumerate()
        {
            // currencies
            if container_type == ContainerType::Currency {
                let mut send_currency = async |item: &Item| {
                    // skip telling the client what they don't have
                    if item.quantity == 0 && first_update {
                        return;
                    }

                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CurrencyCrystalInfo(
                        CurrencyInfo {
                            sequence: sequence as u32,
                            container: container_type,
                            quantity: item.quantity,
                            catalog_id: item.id,
                            unk1: 1,
                            ..Default::default()
                        },
                    ));
                    self.send_ipc_self(ipc).await;
                };

                for i in 0..container.max_slots() {
                    send_currency(container.get_slot(i as u16)).await;
                }
            } else {
                // items

                let mut send_slot = async |slot_index: u16, item: &Item| {
                    // skip telling the client what they don't have
                    if item.quantity == 0 && first_update {
                        return;
                    }

                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateItem(ItemInfo {
                        sequence: sequence as u32,
                        container: container_type,
                        slot: slot_index,
                        quantity: item.quantity,
                        catalog_id: item.id,
                        condition: item.condition,
                        glamour_catalog_id: item.glamour_catalog_id,
                        ..Default::default()
                    }));
                    self.send_ipc_self(ipc).await;
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
                        num_items: container.num_items(),
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
            ContainerType::Crystals,
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

    pub async fn update_equip(
        &mut self,
        actor_id: u32,
        main_weapon_id: u64,
        sub_weapon_id: u64,
        model_ids: [u32; 10],
    ) {
        let chara_details = self.database.find_chara_make(self.player_data.content_id);
        self.send_stats(&chara_details).await;
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Equip(Equip {
            main_weapon_id,
            sub_weapon_id,
            model_ids,
            ..Default::default()
        }));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;

        // TODO: get a capture of another player equipping stuff to see if we get this as well, but it seems unlikely.
        if self.player_data.actor_id == actor_id {
            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::SetItemLevel {
                    level: self.player_data.inventory.equipped.calculate_item_level() as u32,
                },
            })
            .await;
            // Uknown what this is, it's seen when (un)equipping stuff.
            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::Unknown {
                    category: 57,
                    param1: 0,
                    param2: 0,
                    param3: 0,
                    param4: 0,
                },
            })
            .await;
        }

        self.process_effects_list().await;
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

    // TODO: When we add support for ItemObtainedLogMessage, rename this and update this
    pub async fn send_gilshop_ack(
        &mut self,
        event_id: u32,
        item_id: u32,
        item_quantity: u32,
        price_per_item: u32,
        message_type: LogMessageType,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ShopLogMessage {
            event_id,
            message_type: message_type as u32,
            params_count: 3,
            item_id,
            item_quantity,
            total_sale_cost: item_quantity * price_per_item,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_gilshop_item_update(
        &mut self,
        dst_storage_id: u16,
        dst_container_index: u16,
        dst_stack: u32,
        dst_catalog_id: u32,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
            sequence: self.player_data.shop_sequence,
            dst_storage_id,
            dst_container_index,
            dst_stack,
            dst_catalog_id,
            unk1: 0x7530_0000,
        });
        self.send_ipc_self(ipc).await;
        self.player_data.shop_sequence += 1;
    }

    pub async fn send_inventory_transaction_finish(&mut self, unk1: u32, unk2: u32) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransactionFinish {
            sequence: self.player_data.item_sequence,
            sequence_repeat: self.player_data.item_sequence,
            unk1,
            unk2,
        });
        self.send_ipc_self(ipc).await;
    }
}
