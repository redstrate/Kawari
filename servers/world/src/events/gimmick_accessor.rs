use async_trait::async_trait;
use kawari::{common::ObjectTypeId, ipc::zone::SceneFlags};

use crate::{Event, EventHandler, ToServer, ZoneConnection, lua::LuaPlayer};

/// For gimmick accessor events.
#[derive(Debug)]
pub struct GimmickAccessorEventHandler;

impl Default for GimmickAccessorEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GimmickAccessorEventHandler {
    pub const SCENE_BEGIN: u16 = 1;

    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for GimmickAccessorEventHandler {
    async fn on_talk(&self, _event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        player.play_scene(Self::SCENE_BEGIN, SceneFlags::HIDE_HOTBAR, Vec::new());
    }

    async fn on_return(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        _scene: u16,
        results: &[i32],
        _player: &mut LuaPlayer,
    ) {
        connection
            .handle
            .send(ToServer::GimmickAccessor(
                connection.player_data.character.actor_id,
                event.id & 0xFFF,
                results.to_vec(),
            ))
            .await;
    }
}
