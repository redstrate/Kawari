use async_trait::async_trait;
use kawari::{common::ObjectTypeId, ipc::zone::SceneFlags};

use crate::{Event, EventHandler, ZoneConnection, lua::LuaPlayer};

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
                event.id & 0xFFF,
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
        _connection: &mut ZoneConnection,
        _scene: u16,
        _yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        // quit
        if results[2] == 0 {
            player.finish_event();
            return;
        }

        // TODO: figure out these params
        // requests another page?
        player.play_scene(
            0,
            SceneFlags::NO_DEFAULT_CAMERA,
            vec![
                7,
                event.id & 0xFFF,
                2147485320,
                262148,
                24,
                1310820,
                67305316,
                9437184,
                2365587564,
                0,
                0,
                32756,
                0,
                0,
                2373844992,
                32756,
                0,
                0,
                0,
                0,
                2373910528,
                32756,
                0,
                32756,
                0,
                0,
                2373910528,
                32756,
                1,
                65636,
                67305316,
                1638400,
                2365587485,
                0,
                0,
                48,
                0,
                32756,
                2373844992,
                32756,
                0,
                32756,
                0,
                0,
                0,
                0,
                0,
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
                0,
                0,
                0,
                0,
                0,
            ],
        );
    }
}
