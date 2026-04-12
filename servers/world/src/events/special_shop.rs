use async_trait::async_trait;
use kawari::{
    common::{
        ContainerType, ERR_INVENTORY_ADD_FAILED, HandlerId, INVENTORY_ACTION_ACK_SHOP, LogMessageType, ObjectTypeId,
    },
    ipc::zone::{ItemInfo, SceneFlags, ServerZoneIpcData, ServerZoneIpcSegment},
};

use crate::{
    Event, EventHandler, ZoneConnection,
    inventory::{CurrencyKind, Item},
    lua::LuaPlayer,
};

/// For special shop events.
#[derive(Debug)]
pub struct SpecialShopEventHandler;

impl Default for SpecialShopEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecialShopEventHandler {
    /// Item has been bought.
    pub const SCENE_ITEM_BOUGHT: u16 = 0;
    /// Displays shop interface
    pub const SCENE_SHOW_SHOP: u16 = 10;
    /// Seems to be an event termination scene? When used standalone without the proper event sequence, it softlocks.
    pub const SCENE_SHOP_END: u16 = 255;

    pub fn new() -> Self {
        Self {}
    }
}

impl SpecialShopEventHandler {
    async fn process_shop_event_return(
        &self,
        connection: &mut ZoneConnection,
        event: &Event,
        results: &[i32],
    ) -> bool {
        // If you're closing the window
        if results.len() == 1 {
            return true;
        }

        let item_index = results[1];

        let result;

        {
            let mut game_data = connection.gamedata.lock();
            result = game_data.get_specialshop_item(event.id, item_index as u16);
        }

        // TODO: Use the ItemCost field to determine which currency is used

        if let Some(item_info) = result {
            if connection.player_data.inventory.currency.gil.quantity >= item_info.price_mid {
                if let Some(add_result) = connection
                    .player_data
                    .inventory
                    .add_in_next_free_slot(Item::new(&item_info, 1))
                {
                    connection.player_data.inventory.currency.gil.quantity -= item_info.price_mid;
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
                        1,
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
                connection
                    .send_notice(
                        "Insufficient gil to buy item. Nice try bypassing the client-side check!",
                    )
                    .await;
            }
        } else {
            connection
                .send_notice("Unable to find shop item, this is a bug in Kawari!")
                .await;
        }

        false
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
        container: ContainerType,
        slot: u16,
        quantity: u32,
        item_id: u32,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot(ItemInfo {
            sequence: connection.player_data.shop_sequence,
            container,
            slot,
            quantity,
            item_id,
            ..Default::default()
        }));
        connection.send_ipc_self(ipc).await;
        connection.player_data.shop_sequence += 1;
    }
}

#[async_trait]
impl EventHandler for SpecialShopEventHandler {
    async fn on_talk(&self, _event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        player.play_scene(Self::SCENE_ITEM_BOUGHT, SceneFlags::HIDE_HOTBAR, Vec::new());
    }

    async fn on_return(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        scene: u16,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        if scene == Self::SCENE_ITEM_BOUGHT {
            if Self::process_shop_event_return(self, connection, event, results).await {
                player.finish_event();
            }
        } else if scene == Self::SCENE_SHOW_SHOP {
            player.play_scene(
                Self::SCENE_SHOP_END,
                SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                Vec::new(),
            );
        } else {
            player.finish_event();
        }
    }
}
