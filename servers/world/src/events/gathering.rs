use async_trait::async_trait;
use kawari::{
    common::ObjectTypeId,
    ipc::zone::{ActorControlCategory, SceneFlags},
};

use crate::{Event, EventHandler, ItemInfoQuery, ZoneConnection, inventory::Item, lua::LuaPlayer};

/// For gathering events.
#[derive(Debug)]
pub struct GatheringEventHandler;

impl Default for GatheringEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GatheringEventHandler {
    pub const SCENE_HIDING_ROD: u16 = 3;

    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for GatheringEventHandler {
    async fn on_talk(&self, event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        // TODO: figure out these params
        player.play_scene(
            0,
            SceneFlags::NO_DEFAULT_CAMERA,
            vec![
                0,
                event.id & 0xFFFF,
                2147485320,
                262148,
                24,
                1310820,
                67305316,
                9437184,
                108,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                65636,
                67305316,
                1638400,
                29,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                32755,
                0,
                0,
                1121910784,
                32755,
                2616352782,
                1,
                1114399744,
                0,
                0,
                0,
                33024,
                0,
                0,
                0,
                0,
                0,
            ],
        );
    }

    async fn on_yield(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        scene: u16,
        _yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        // TODO: store this on begin gather
        let items;
        {
            let mut gamedata = connection.gamedata.lock();
            items = gamedata.get_gathering_point_items(event.id & 0xFFFF);
        }

        if results[2] == 2 {
            // gather
            let item_index = results[1];
            let gather_item_id = items[item_index as usize];
            let item_id;
            {
                let mut gamedata = connection.gamedata.lock();
                item_id = gamedata.convert_gathering_point_item(gather_item_id as u32);
            }

            // plays the animation
            player.play_scene(1, SceneFlags::NO_DEFAULT_CAMERA, vec![2, 266]);

            // Add item to their inventory
            {
                let mut gamedata = connection.gamedata.lock();

                if let Some(item_info) = gamedata.get_item_info(ItemInfoQuery::ById(item_id as u32))
                {
                    connection
                        .player_data
                        .inventory
                        .add_in_next_free_slot(Item::new(item_info, 1));
                }
            }

            connection.send_inventory().await;

            // The item was added to your inventory.
            connection
                .actor_control_self(ActorControlCategory::LogMessage {
                    log_message: 789,
                    id: item_id as u32,
                })
                .await;

            return;
        }

        // quit
        if scene == 0 && results[2] == 0 {
            player.finish_event();
            return;
        }

        // TODO: figure out these params
        // TODO: why is the items in such a weird order?
        player.play_scene(
            0,
            SceneFlags::NO_DEFAULT_CAMERA,
            vec![
                7,
                event.id & 0xFFFF,
                2147485320,
                262148,
                // first item
                items[0] as u32,
                1310820,
                67305316,
                9437184,
                2365587564,
                0,
                // second item
                items[1] as u32,
                32756,
                0,
                0,
                2373844992,
                32756,
                // third item
                items[2] as u32,
                0,
                0,
                0,
                2373910528,
                32756,
                // fourth item
                items[3] as u32,
                32756,
                0,
                0,
                2373910528,
                32756,
                // fifth item
                items[4] as u32,
                65636,
                67305316,
                1638400,
                2365587485,
                0,
                // sixth item
                items[5] as u32,
                48,
                0,
                32756,
                2373844992,
                32756,
                // seventh item
                items[6] as u32,
                32756,
                0,
                0,
                0,
                0,
                // eight item
                items[7] as u32,
                32756,
                0,
                0,
                2945843200,
                32756,
                859451662,
                1,
                0,
                0,
                0,
                0,
                33024,
            ],
        );
    }
}
