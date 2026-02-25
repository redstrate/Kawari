use async_trait::async_trait;
use kawari::{
    common::{
        ContainerType, ERR_INVENTORY_ADD_FAILED, HandlerId, INVENTORY_ACTION_ACK_SHOP,
        ItemOperationKind, LogMessageType, ObjectTypeId,
    },
    ipc::zone::{ItemOperation, SceneFlags, ServerZoneIpcData, ServerZoneIpcSegment},
};

use crate::{
    Event, EventHandler, ItemInfoQuery, ZoneConnection,
    inventory::{BuyBackItem, CurrencyKind, Item, get_container_type},
    lua::LuaPlayer,
};

/// For shop events.
#[derive(Debug)]
pub struct ShopEventHandler;

impl Default for ShopEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ShopEventHandler {
    /// NPC greeting (usually an animation, sometimes text too?)
    pub const SCENE_GREETING: u16 = 0;
    /// Displays shop interface
    pub const SCENE_SHOW_SHOP: u16 = 10;
    /// Seems to be an event termination scene? When used standalone without the proper event sequence, it softlocks.
    pub const SCENE_SHOP_END: u16 = 255;

    pub fn new() -> Self {
        Self {}
    }
}

impl ShopEventHandler {
    fn get_buyback_list(
        connection: &mut ZoneConnection,
        shop_id: u32,
        shop_intro: bool,
    ) -> Vec<u32> {
        connection
            .player_data
            .buyback_list
            .as_scene_params(shop_id, shop_intro)
    }

    async fn do_gilshop_buyback(
        connection: &mut ZoneConnection,
        shop_id: u32,
        buyback_index: u32,
        lua_player: &mut LuaPlayer,
    ) {
        let bb_item;
        {
            let Some(tmp_bb_item) = connection
                .player_data
                .buyback_list
                .get_buyback_item(shop_id, buyback_index)
            else {
                let error = "Invalid buyback index, ignoring buyback action!";
                connection.send_notice(error).await;
                tracing::warn!(error);
                return;
            };
            bb_item = tmp_bb_item.clone();
        }

        // This is a no-op since we can't edit PlayerData from the Lua side, but we can queue it up afterward.
        // We *need* this information, though.
        let item_to_restore = Item::new(bb_item.as_item_info(), bb_item.quantity);
        let Some(item_dst_info) = connection
            .player_data
            .inventory
            .add_in_next_free_slot(item_to_restore)
        else {
            let error = "Your inventory is full. Unable to restore item.";
            connection.send_notice(error).await;
            tracing::warn!(error);
            return;
        };

        // This is a no-op since we can't edit PlayerData from the Lua side,
        // but we need to do it here so the shopkeeper script doesn't see stale data.
        connection
            .player_data
            .buyback_list
            .remove_item(shop_id, buyback_index);

        // TODO: port from LuaPlayer
        // Queue up the item restoration, but we're not going to send an entire inventory update to the client.
        lua_player.add_item(bb_item.id, item_dst_info.quantity, false);

        // Queue up the player's adjusted gil, but we're not going to send an entire inventory update to the client.
        let cost = item_dst_info.quantity * bb_item.price_low;
        let new_gil = connection.player_data.inventory.currency.gil.quantity - cost;
        lua_player.modify_currency(CurrencyKind::Gil, -(cost as i32), false);

        let shop_packets_to_send = [
            ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
                sequence: connection.player_data.shop_sequence,
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
                sequence: connection.player_data.shop_sequence,
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
            connection.send_ipc_self(ipc).await;
        }
    }

    async fn process_shop_event_yield(
        connection: &mut ZoneConnection,
        event: &Event,
        results: &[i32],
    ) {
        let buy_sell_mode = results[0];
        let item_index = results[1];
        let item_quantity = results[2] as u32;

        tracing::info!(
            "Client is interacting with a shop! {:#?} {buy_sell_mode:#?} {item_quantity:#?} {item_index:#?}",
            event.id,
        );

        const BUY: i32 = 1;
        const SELL: i32 = 2;

        if buy_sell_mode == BUY {
            let result;
            {
                let mut game_data = connection.gamedata.lock();
                result = game_data.get_gilshop_item(event.id, item_index as u16);
            }

            if let Some(item_info) = result {
                if connection.player_data.inventory.currency.gil.quantity
                    >= item_quantity * item_info.price_mid
                {
                    if let Some(add_result) = connection
                        .player_data
                        .inventory
                        .add_in_next_free_slot(Item::new(item_info.clone(), item_quantity))
                    {
                        connection.player_data.inventory.currency.gil.quantity -=
                            item_quantity * item_info.price_mid;
                        Self::send_gilshop_item_update(
                            connection,
                            ContainerType::Currency,
                            0,
                            connection.player_data.inventory.currency.gil.quantity,
                            CurrencyKind::Gil as u32,
                        )
                        .await;

                        connection
                            .send_inventory_ack(u32::MAX, INVENTORY_ACTION_ACK_SHOP as u16)
                            .await;

                        Self::send_gilshop_item_update(
                            connection,
                            add_result.container,
                            add_result.index,
                            add_result.quantity,
                            item_info.id,
                        )
                        .await;
                        Self::send_gilshop_ack(
                            connection,
                            event.id,
                            item_info.id,
                            item_quantity,
                            item_info.price_mid,
                            LogMessageType::ItemBought,
                        )
                        .await;

                        // See GenericShopkeeper.lua for information about this scene, the flags, and the params.
                        connection
                            .event_scene(
                                event,
                                10,
                                SceneFlags::from_bits(8193).unwrap(),
                                vec![1, 100],
                            )
                            .await;
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        connection.send_notice(ERR_INVENTORY_ADD_FAILED).await;
                    }
                } else {
                    connection.send_notice(
                            "Insufficient gil to buy item. Nice try bypassing the client-side check!",
                        )
                        .await;
                }
            } else {
                connection
                    .send_notice("Unable to find shop item, this is a bug in Kawari!")
                    .await;
            }
        } else if buy_sell_mode == SELL {
            let storage = get_container_type(item_index as u32).unwrap();
            let index = item_quantity;
            let result;
            let quantity;
            {
                let item = connection
                    .player_data
                    .inventory
                    .get_item(storage, index as u16);
                let mut game_data = connection.gamedata.lock();
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
                connection
                    .player_data
                    .buyback_list
                    .push_item(event.id, bb_item);

                connection.player_data.inventory.currency.gil.quantity +=
                    quantity * item_info.price_low;
                Self::send_gilshop_item_update(
                    connection,
                    ContainerType::Currency,
                    0,
                    connection.player_data.inventory.currency.gil.quantity,
                    CurrencyKind::Gil as u32,
                )
                .await;
                Self::send_gilshop_item_update(connection, storage, index as u16, 0, 0).await;

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                    sequence: connection.player_data.item_sequence,
                    operation_type: ItemOperationKind::Update,
                    src_actor_id: connection.player_data.character.actor_id,
                    src_storage_id: ContainerType::Currency,
                    src_container_index: 0,
                    src_stack: connection.player_data.inventory.currency.gil.quantity,
                    src_catalog_id: CurrencyKind::Gil as u32,
                    dst_actor_id: Default::default(),
                    dummy_container: ContainerType::DiscardingItemSentinel,
                    dst_storage_id: ContainerType::DiscardingItemSentinel,
                    dst_container_index: u16::MAX,
                    dst_stack: 0,
                    dst_catalog_id: 0,
                });
                connection.send_ipc_self(ipc).await;

                // Process the server's inventory first.
                let action = ItemOperation {
                    operation_type: ItemOperationKind::Discard,
                    src_storage_id: storage,
                    src_container_index: index as u16,
                    ..Default::default()
                };

                connection.player_data.inventory.process_action(&action);

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransaction {
                    sequence: connection.player_data.item_sequence,
                    operation_type: ItemOperationKind::Discard,
                    src_actor_id: connection.player_data.character.actor_id,
                    src_storage_id: storage,
                    src_container_index: index as u16,
                    src_stack: quantity,
                    src_catalog_id: item_info.id,
                    dst_actor_id: Default::default(),
                    dummy_container: ContainerType::DiscardingItemSentinel,
                    dst_storage_id: ContainerType::DiscardingItemSentinel,
                    dst_container_index: u16::MAX,
                    dst_stack: 0,
                    dst_catalog_id: 0,
                });
                connection.send_ipc_self(ipc).await;

                connection
                    .send_inventory_transaction_finish(0x100, 0x300)
                    .await;

                Self::send_gilshop_ack(
                    connection,
                    event.id,
                    item_info.id,
                    quantity,
                    item_info.price_low,
                    LogMessageType::ItemSold,
                )
                .await;

                let mut params = connection
                    .player_data
                    .buyback_list
                    .as_scene_params(event.id, false);
                params[0] = SELL as u32;
                params[1] = 0; // The "terminator" is 0 for sell mode.
                connection
                    .event_scene(event, 10, SceneFlags::from_bits(8193).unwrap(), params)
                    .await;
            } else {
                connection
                    .send_notice("Unable to find shop item, this is a bug in Kawari!")
                    .await;
            }
        } else {
            tracing::error!("Received unknown transaction mode {buy_sell_mode}!");
        }
    }

    // TODO: When we add support for ItemObtainedLogMessage, rename this and update this
    async fn send_gilshop_ack(
        connection: &mut ZoneConnection,
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
        connection.send_ipc_self(ipc).await;
    }

    pub async fn send_gilshop_item_update(
        connection: &mut ZoneConnection,
        dst_storage_id: ContainerType,
        dst_container_index: u16,
        dst_stack: u32,
        dst_catalog_id: u32,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
            sequence: connection.player_data.shop_sequence,
            dst_storage_id,
            dst_container_index,
            dst_stack,
            dst_catalog_id,
            unk1: 0x7530_0000,
        });
        connection.send_ipc_self(ipc).await;
        connection.player_data.shop_sequence += 1;
    }
}

#[async_trait]
impl EventHandler for ShopEventHandler {
    async fn on_talk(&self, _event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        player.play_scene(Self::SCENE_GREETING, SceneFlags::HIDE_HOTBAR, Vec::new());
    }

    async fn on_yield(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        _scene: u16,
        _yield_id: u8,
        results: &[i32],
        _player: &mut LuaPlayer,
    ) {
        Self::process_shop_event_yield(connection, event, results).await;
    }

    async fn on_return(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        scene: u16,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        // Retail uses 221 or 222 u32s as the params to the shop cutscene, representing the buyback list and 1 or 2 additional parameters,
        // but it opens fine with a single zero u32 when the buyback list is empty.
        // 22 u32s are used to represent the ten buyback items. Most of these values are still unknown in meaning, but they likely relate to melds, crafting signature, durability, and more.
        // Historically, it seems cutscene 00040 was used instead of 00010 as it is now.
        // When the client concludes business with the shop, the scene finishes and returns control to the server. The server will then have the client play scene 255 with no params.

        if scene == Self::SCENE_GREETING {
            let buyback_list = Self::get_buyback_list(connection, event.id, true);
            player.play_scene(
                Self::SCENE_SHOW_SHOP,
                SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                buyback_list,
            );
        } else if scene == Self::SCENE_SHOW_SHOP {
            let buyback = 3;
            // It shouldn't even be possible to get into a situation where results[1] isn't BUYBACK, but we'll leave it as a guard.
            if !results.is_empty() && results[0] == buyback {
                let item_index = results[1];
                Self::do_gilshop_buyback(connection, event.id, item_index as u32, player).await;

                let mut buyback_list = Self::get_buyback_list(connection, event.id, false);
                buyback_list[0] = buyback as u32;
                buyback_list[1] = 100; // Unknown what this 100 represents: a terminator, perhaps? For sell mode it's 0, while buy and buyback are both 100.
                player.play_scene(
                    Self::SCENE_SHOW_SHOP,
                    SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                    buyback_list,
                );
            } else if results.is_empty() {
                // The player closed the shop window.
                player.play_scene(
                    Self::SCENE_SHOP_END,
                    SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                    Vec::new(),
                );
            }
        } else {
            player.finish_event();
        }
    }
}
