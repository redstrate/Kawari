use async_trait::async_trait;
use kawari::{
    common::ObjectTypeId,
    ipc::zone::{Condition, SceneFlags},
};

use crate::{Event, EventHandler, ToServer, ZoneConnection, lua::LuaPlayer};

/// For GimmickAccessor events.
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
                event.id.event_id(),
                connection.player_data.character.actor_id,
                event.actor_id.object_id,
                results.to_vec(),
            ))
            .await;
    }
}

/// For GimmickRect events.
#[derive(Debug)]
pub struct GimmickRectEventHandler;

impl Default for GimmickRectEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GimmickRectEventHandler {
    pub const SCENE_BEGIN: u16 = 1;

    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for GimmickRectEventHandler {
    // Only really needed for walking into content entrances (to exit)
    async fn on_return(
        &self,
        _event: &Event,
        _connection: &mut ZoneConnection,
        _scene: u16,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        // If chosen to leave duty:
        if results.len() == 1 && results[0] == 0 {
            player.abandon_content();
        }
        player.finish_event();
    }

    fn condition(&self) -> Condition {
        Condition::OccupiedInEvent
    }
}

/// For GimmickBill events.
#[derive(Debug)]
pub struct GimmickBillEventHandler;

impl Default for GimmickBillEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl GimmickBillEventHandler {
    pub const SCENE_BEGIN: u16 = 1;

    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for GimmickBillEventHandler {
    async fn on_talk(&self, _event: &Event, _target_id: ObjectTypeId, player: &mut LuaPlayer) {
        player.play_scene(Self::SCENE_BEGIN, SceneFlags::HIDE_HOTBAR, Vec::new());
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
