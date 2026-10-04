use async_trait::async_trait;
use kawari::ipc::zone::EventType;

use crate::{Event, EventHandler, ToServer, ZoneConnection, lua::LuaPlayer};

/// For content director events.
/// This is mostly a dummy struct, most of the logic exists in the global server state.
#[derive(Debug)]
pub struct ContentEventHandler;

impl Default for ContentEventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl EventHandler for ContentEventHandler {
    async fn on_return(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        _scene: u16,
        _results: &[i32],
        player: &mut LuaPlayer,
    ) {
        if event.event_type == EventType::EnterTerritory {
            connection
                .handle
                .send(ToServer::ReadyToCommence(
                    connection.player_data.character.actor_id,
                ))
                .await;
        }
        player.finish_event();
    }
}
