use async_trait::async_trait;
use kawari::{common::ObjectTypeId, ipc::zone::SceneFlags};

use crate::{
    Event, EventHandler, ShopEventHandler, ZoneConnection, inventory::Item, lua::LuaPlayer,
};

/// For gimmick accessor events.
#[derive(Debug)]
pub struct InclusionShopEventHandler;

impl Default for InclusionShopEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InclusionShopEventHandler {
    pub const SCENE_SHOW_MENU: u16 = 1;
    pub const SCENE_BOUGHT_ITEM: u16 = 2;

    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for InclusionShopEventHandler {
    async fn on_talk(&self, _event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        player.play_scene(Self::SCENE_SHOW_MENU, SceneFlags::HIDE_HOTBAR, Vec::new());
    }

    async fn on_yield(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        _scenee: u16,
        _yield_id: u8,
        results: &[i32],
        _player: &mut LuaPlayer,
    ) {
        // TODO: decrease currency
        // TODO: support quantity

        let special_shop_id = results[2] as u32;
        let item_index = results[4] as u16;

        let result;
        {
            let mut game_data = connection.gamedata.lock();
            result = game_data.get_specialshop_item(special_shop_id, item_index);
        }

        let item_quantity = 1;

        if let Some(item_info) = result
            && let Some(add_result) = connection
                .player_data
                .inventory
                .add_in_next_free_slot(Item::new(item_info.clone(), item_quantity))
        {
            ShopEventHandler::send_gilshop_item_update(
                connection,
                add_result.container,
                add_result.index,
                add_result.quantity,
                item_info.id,
            )
            .await;

            // TODO: ACS 854 is sent
            // TODO: itemobtainedlogmessage is sent

            connection
                .event_scene(
                    event,
                    Self::SCENE_BOUGHT_ITEM,
                    SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
                    vec![],
                )
                .await;
        }
    }

    async fn on_return(
        &self,
        _event: &Event,
        _connectionn: &mut ZoneConnection,
        _scene: u16,
        _results: &[i32],
        player: &mut LuaPlayer,
    ) {
        player.finish_event();
    }
}
