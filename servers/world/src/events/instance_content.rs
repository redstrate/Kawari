use async_trait::async_trait;
use kawari::ipc::zone::SceneFlags;

use crate::{Event, EventHandler, ZoneConnection, lua::LuaPlayer};

/// For instance content events.
#[derive(Debug)]
pub struct InstanceContentEventHandler;

impl Default for InstanceContentEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InstanceContentEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for InstanceContentEventHandler {
    async fn on_enter_territory(&self, _event: &Event, player: &mut LuaPlayer) {
        // TODO: figure out scene params
        player.play_scene(
            1,
            SceneFlags::NO_DEFAULT_CAMERA
                | SceneFlags::CONDITION_CUTSCENE
                | SceneFlags::HIDE_HOTBAR
                | SceneFlags::SILENT_ENTER_TERRI_ENV
                | SceneFlags::SILENT_ENTER_TERRI_BGM
                | SceneFlags::SILENT_ENTER_TERRI_SE
                | SceneFlags::DISABLE_STEALTH
                | SceneFlags::DISABLE_CANCEL_EMOTE
                | SceneFlags::INVIS_AOE
                | SceneFlags::UNK1,
            vec![
                0, // BGM, according to sapphire?
                0,
                0,
                5,
                14400,
                0,
                0,
                0,
                0,
                0,
                player.content_data.duration as u32,
                player.content_data.settings,
            ],
        )
    }

    async fn on_return(
        &self,
        event: &Event,
        _connection: &mut ZoneConnection,
        _scene: u16,
        _results: &[i32],
        player: &mut LuaPlayer,
    ) {
        player.commence_duty(event.id);
        player.finish_event();
    }
}
