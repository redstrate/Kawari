use async_trait::async_trait;
use kawari::ipc::zone::{ActorControlCategory, LiveEventType, SceneFlags};

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
        connection: &mut ZoneConnection,
        _scene: u16,
        _yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        if results[0] == 0 {
            connection
                .actor_control_self(ActorControlCategory::LiveEvent {
                    event: LiveEventType::StartCraft {
                        unk1: 0,
                        unk2: 0,
                        unk3: 0,
                    },
                })
                .await;

            player.play_scene(
                0,
                SceneFlags::NO_DEFAULT_CAMERA,
                vec![2, connection.recipe.unwrap().id, 0, 1],
            );
        } else if results[0] == 10 {
            // results[1] is the CraftAction ID

            // Play the basic touch animation and VFX:
            connection
                .actor_control_self(ActorControlCategory::LiveEvent {
                    event: LiveEventType::Unknown {
                        event: 12,
                        param1: 239,
                        param2: 246,
                        param3: 0,
                    },
                })
                .await;

            // Increase to max
            player.play_scene(
                0,
                SceneFlags::NO_DEFAULT_CAMERA,
                vec![
                    9, 0, 0, 0, 100045, 0, 1, 19, // Progress
                    27, 0, 0, 1, 50, 4294967286, 1, 1, 100, 22,
                ],
            );
        } else if results[0] == 1 {
            player.play_scene(0, SceneFlags::NO_DEFAULT_CAMERA, vec![3, 0, 0, 0]);
        } else if results[0] == 11 {
            // The item was added to your inventory.
            connection
                .actor_control_self(ActorControlCategory::LogMessage {
                    log_message: 789,
                    id: connection.recipe.unwrap().item_id as u32,
                })
                .await;

            connection
                .actor_control_self(ActorControlCategory::LiveEvent {
                    event: LiveEventType::EndCraft {},
                })
                .await;

            // Kick 'em out to the crafting window
            player.play_scene(0, SceneFlags::NO_DEFAULT_CAMERA, vec![4, 0, 0, 0]);
        } else if results[0] == 7 {
            // Manually quit
            player.finish_event();
        }
    }
}
