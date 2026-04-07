use async_trait::async_trait;
use kawari::{common::CharacterMode, ipc::zone::Condition};

use crate::{Event, EventHandler, ZoneConnection, lua::LuaPlayer};

/// For fishing events.
#[derive(Debug)]
pub struct FishingEventHandler;

impl Default for FishingEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FishingEventHandler {
    pub const SCENE_HIDING_ROD: u16 = 3;

    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for FishingEventHandler {
    async fn on_return(
        &self,
        _event: &Event,
        _connection: &mut ZoneConnection,
        scene: u16,
        _results: &[i32],
        player: &mut LuaPlayer,
    ) {
        if scene == Self::SCENE_HIDING_ROD {
            player.finish_event();
        }
    }

    fn condition(&self) -> Condition {
        Condition::Fishing
    }

    fn character_mode(&self) -> CharacterMode {
        CharacterMode::Gathering
    }
}
