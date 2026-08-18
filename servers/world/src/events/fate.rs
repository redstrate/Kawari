use async_trait::async_trait;
use kawari::{common::ObjectTypeId, ipc::zone::SceneFlags};

use crate::{Event, EventHandler, ZoneConnection, lua::LuaPlayer};

/// For FATE motivation NPCs.
#[derive(Debug)]
pub struct FateEventHandler {
    pub fate_id: u16,
}

#[async_trait]
impl EventHandler for FateEventHandler {
    async fn on_talk(&self, _event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        player.play_scene(
            3,
            SceneFlags::NO_DEFAULT_CAMERA | SceneFlags::HIDE_HOTBAR,
            vec![self.fate_id as u32, 139],
        );
    }

    async fn on_return(
        &self,
        _event: &Event,
        _connection: &mut ZoneConnection,
        _scene: u16,
        _results: &[i32],
        player: &mut LuaPlayer,
    ) {
        player.finish_event();
    }
}
