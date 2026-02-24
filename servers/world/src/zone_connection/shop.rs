use kawari::{
    common::{
        ContainerType, ERR_INVENTORY_ADD_FAILED, HandlerId, INVALID_OBJECT_ID,
        INVENTORY_ACTION_ACK_SHOP, ItemOperationKind, LogMessageType,
    },
    ipc::zone::{
        EventYieldHandler, ItemOperation, SceneFlags, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

use crate::{
    ItemInfoQuery, ZoneConnection,
    inventory::{BuyBackItem, CurrencyKind, Item, get_container_type},
    lua::LuaPlayer,
};

impl ZoneConnection {
    pub async fn process_shop_event_yield(
        &mut self,
        handler: &EventYieldHandler<4>,
        lua_player: &mut LuaPlayer,
    ) {
        let event_id = handler.handler_id;
        let buy_sell_mode = handler.params[0];
        let item_index = handler.params[1];
        let item_quantity = handler.params[2] as u32;

        tracing::info!(
            "Client is interacting with a shop! {event_id:#?} {buy_sell_mode:#?} {item_quantity:#?} {item_index:#?}"
        );

        const BUY: i32 = 1;
        const SELL: i32 = 2;

        if buy_sell_mode == BUY {
            let result;
            {
                let mut game_data = self.gamedata.lock();
                result = game_data.get_gilshop_item(event_id.0, item_index as u16);
            }

            if let Some(item_info) = result {
                if self.player_data.inventory.currency.gil.quantity
                    >= item_quantity * item_info.price_mid
                {
                    if let Some(add_result) = self
                        .player_data
                        .inventory
                        .add_in_next_free_slot(Item::new(item_info.clone(), item_quantity))
                    {
                        self.player_data.inventory.currency.gil.quantity -=
                            item_quantity * item_info.price_mid;
                        self.send_gilshop_item_update(
                            ContainerType::Currency,
                            0,
                            self.player_data.inventory.currency.gil.quantity,
                            CurrencyKind::Gil as u32,
                        )
                        .await;

                        self.send_inventory_ack(u32::MAX, INVENTORY_ACTION_ACK_SHOP as u16)
                            .await;

                        self.send_gilshop_item_update(
                            add_result.container,
                            add_result.index,
                            add_result.quantity,
                            item_info.id,
                        )
                        .await;
                        self.send_gilshop_ack(
                            event_id.0,
                            item_info.id,
                            item_quantity,
                            item_info.price_mid,
                            LogMessageType::ItemBought,
                        )
                        .await;

                        // See GenericShopkeeper.lua for information about this scene, the flags, and the params.
                        self.event_scene(
                            event_id.0,
                            10,
                            SceneFlags::from_bits(8193).unwrap(),
                            vec![1, 100],
                            lua_player,
                        )
                        .await;
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        self.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                        self.event_finish(event_id.0, lua_player).await;
                    }
                } else {
                    self.send_notice(
                        "Insufficient gil to buy item. Nice try bypassing the client-side check!",
                    )
                    .await;
                    self.event_finish(event_id.0, lua_player).await;
                }
            } else {
                self.send_notice("Unable to find shop item, this is a bug in Kawari!")
                    .await;
                self.event_finish(event_id.0, lua_player).await;
            }
        } else if buy_sell_mode == SELL {
            let storage = get_container_type(item_index as u32).unwrap();
            let index = item_quantity;
            let result;
            let quantity;
            {
                let item = self.player_data.inventory.get_item(storage, index as u16);
                let mut game_data = self.gamedata.lock();
                result = game_data.get_item_info(ItemInfoQuery::ById(item.id));
                quantity = item.quantity;
            }

            if let Some(item_info) = result {
                let bb_item = BuyBackItem {
                    id: item_info.id,
                    quantity,
                    price_low: item_info.price_low,
                    item_level: item_info.item_level,
                    stack_size: item_info.stack_size,
                };
                self.player_data.buyback_list.push_item(event_id.0, bb_item);

                self.player_data.inventory.currency.gil.quantity += quantity * item_info.price_low;
                self.send_gilshop_item_update(
                    ContainerType::Currency,
                    0,
                    self.player_data.inventory.currency.gil.quantity,
                    CurrencyKind::Gil as u32,
                )
                .await;
                self.send_gilshop_item_update(storage, index as u16, 0, 0)
                    .await;

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                    sequence: self.player_data.item_sequence,
                    operation_type: ItemOperationKind::Update,
                    src_actor_id: self.player_data.character.actor_id,
                    src_storage_id: ContainerType::Currency,
                    src_container_index: 0,
                    src_stack: self.player_data.inventory.currency.gil.quantity,
                    src_catalog_id: CurrencyKind::Gil as u32,
                    dst_actor_id: INVALID_OBJECT_ID,
                    dummy_container: ContainerType::DiscardingItemSentinel,
                    dst_storage_id: ContainerType::DiscardingItemSentinel,
                    dst_container_index: u16::MAX,
                    dst_stack: 0,
                    dst_catalog_id: 0,
                });
                self.send_ipc_self(ipc).await;

                // Process the server's inventory first.
                let action = ItemOperation {
                    operation_type: ItemOperationKind::Discard,
                    src_storage_id: storage,
                    src_container_index: index as u16,
                    ..Default::default()
                };

                self.player_data.inventory.process_action(&action);

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                    sequence: self.player_data.item_sequence,
                    operation_type: ItemOperationKind::Discard,
                    src_actor_id: self.player_data.character.actor_id,
                    src_storage_id: storage,
                    src_container_index: index as u16,
                    src_stack: quantity,
                    src_catalog_id: item_info.id,
                    dst_actor_id: INVALID_OBJECT_ID,
                    dummy_container: ContainerType::DiscardingItemSentinel,
                    dst_storage_id: ContainerType::DiscardingItemSentinel,
                    dst_container_index: u16::MAX,
                    dst_stack: 0,
                    dst_catalog_id: 0,
                });
                self.send_ipc_self(ipc).await;

                self.send_inventory_transaction_finish(0x100, 0x300).await;

                self.send_gilshop_ack(
                    event_id.0,
                    item_info.id,
                    quantity,
                    item_info.price_low,
                    LogMessageType::ItemSold,
                )
                .await;

                let mut params = self
                    .player_data
                    .buyback_list
                    .as_scene_params(event_id.0, false);
                params[0] = SELL as u32;
                params[1] = 0; // The "terminator" is 0 for sell mode.
                self.event_scene(
                    event_id.0,
                    10,
                    SceneFlags::from_bits(8193).unwrap(),
                    params,
                    lua_player,
                )
                .await;
            } else {
                self.send_notice("Unable to find shop item, this is a bug in Kawari!")
                    .await;
                self.event_finish(event_id.0, lua_player).await;
            }
        } else {
            tracing::error!("Received unknown transaction mode {buy_sell_mode}!");
            self.event_finish(event_id.0, lua_player).await;
        }
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
            handler_id: HandlerId(event_id),
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
        dst_storage_id: ContainerType,
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

    pub async fn buy_special_shop(
        &mut self,
        event_id: u32,
        special_shop_id: u32,
        item_index: u32,
        lua_player: &mut LuaPlayer,
    ) {
        // TODO: decrease currency
        // TODO: support quantity

        let result;
        {
            let mut game_data = self.gamedata.lock();
            result = game_data.get_specialshop_item(special_shop_id, item_index as u16);
        }

        let item_quantity = 1;

        if let Some(item_info) = result
            && let Some(add_result) = self
                .player_data
                .inventory
                .add_in_next_free_slot(Item::new(item_info.clone(), item_quantity))
        {
            self.send_gilshop_item_update(
                add_result.container,
                add_result.index,
                add_result.quantity,
                item_info.id,
            )
            .await;

            // TODO: ACS 854 is sent
            // TODO: itemobtainedlogmessage is sent

            // See GenericShopkeeper.lua for information about this scene, the flags, and the params.
            self.event_scene(
                event_id,
                2,
                SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                vec![],
                lua_player,
            )
            .await;
        }
    }
}
