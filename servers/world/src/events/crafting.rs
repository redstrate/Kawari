use async_trait::async_trait;
use kawari::ipc::zone::SceneFlags;

use crate::{Event, EventHandler, ZoneConnection, lua::LuaPlayer};

/// For crafting events.
#[derive(Debug)]
pub struct CraftingEventHandler;

impl Default for CraftingEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl CraftingEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for CraftingEventHandler {
    async fn on_yield(
        &self,
        _event: &Event,
        _connection: &mut ZoneConnection,
        _scene: u16,
        _yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        if results[0] == 0 {
            player.play_scene(0, SceneFlags::NO_DEFAULT_CAMERA, vec![2, 303, 0, 1]);
        } else {
            player.play_scene(0, SceneFlags::NO_DEFAULT_CAMERA, vec![3, 0, 0, 0]);
        }
    }
}
