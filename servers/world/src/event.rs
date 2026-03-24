use std::sync::Arc;

use async_trait::async_trait;
use parking_lot::Mutex;

use kawari::{
    common::{HandlerId, HandlerType, ObjectTypeId},
    ipc::zone::{Condition, EventType},
};

use crate::{
    CraftingEventHandler, FishingEventHandler, GameData, GatheringEventHandler,
    GimmickAccessorEventHandler, InclusionShopEventHandler, InstanceContentEventHandler,
    LuaEventHandler, ShopEventHandler, ZoneConnection,
};

use super::lua::LuaPlayer;

#[derive(Debug)]
pub struct Event {
    pub id: u32,
    pub event_type: EventType,
    pub event_arg: u32,
    /// The condition set for this event.
    pub condition: Condition,
    /// The actor associated with this event. This is usually an NPC for Talk events, otherwise the player who initiated it.
    pub actor_id: ObjectTypeId,
}

/// Abstract event handler that can be implemented in Lua or Rust.
#[allow(unused)]
#[async_trait]
pub trait EventHandler: std::fmt::Debug + Send + Sync {
    async fn on_enter_territory(&self, event: &Event, player: &mut LuaPlayer) {}

    async fn on_enter_trigger(&self, event: &Event, player: &mut LuaPlayer, arg: u32) {}

    async fn on_talk(&self, event: &Event, target_id: ObjectTypeId, player: &mut LuaPlayer) {}

    async fn on_yield(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        scene: u16,
        yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
    }

    async fn on_return(
        &self,
        event: &Event,
        connection: &mut ZoneConnection,
        scene: u16,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
    }
}

/// Extracts the script id from a given CustomTalk `name`. For example, "CmnDefBeginnerGuide_00327" will return 327.
fn extract_script_id(name: &str) -> Result<u32, std::num::ParseIntError> {
    name[name.len() - 5..].parse()
}

/// Creates the proper folder name from a given script id. For example, 327 will return 003.
fn folder_from_script_id(id: u32) -> String {
    format!("{:03}", (id / 100))
}

/// Finds and creates the relevant `EventHandler` for this event.
pub fn dispatch_event(
    handler_id: HandlerId,
    game_data: Arc<Mutex<GameData>>,
) -> Option<Box<dyn EventHandler>> {
    let generic_lua_event = |path: &str| -> Option<Box<dyn EventHandler>> {
        if let Some(event) = LuaEventHandler::new(handler_id, path, game_data.clone()) {
            Some(Box::new(event))
        } else {
            tracing::warn!("{path} was not found!");
            None
        }
    };

    match handler_id.handler_type() {
        HandlerType::Quest => {
            let script_name;
            {
                let mut game_data = game_data.lock();
                script_name = game_data.get_quest_name(handler_id.0);
            }
            let Ok(script_id) = extract_script_id(&script_name) else {
                tracing::error!(
                    "Unable to obtain script id from script {script_name}, the event cannot continue!"
                );
                return None;
            };

            let script_folder = folder_from_script_id(script_id);
            let script_path = format!("events/quest/{script_folder}/{script_name}.lua");

            generic_lua_event(&script_path)
        }
        HandlerType::Shop => Some(Box::new(ShopEventHandler::new())),
        HandlerType::Warp => {
            let warp_name;
            {
                let mut game_data = game_data.lock();
                warp_name = game_data.get_warp_logic_name(handler_id.0);
            }

            if warp_name.is_empty() {
                generic_lua_event("events/generic/Warp.lua")
            } else {
                let script_path = format!("events/warp/{warp_name}.lua");
                generic_lua_event(&script_path)
            }
        }
        HandlerType::GatheringPoint => Some(Box::new(GatheringEventHandler::new())),
        HandlerType::Aetheryte => {
            // The Aetheryte sheet actually begins at 0, not 327680
            let aetheryte_id = handler_id.0 & 0xFFF;

            // Aetherytes and Aethernet shards are handled by different event scripts
            let is_aetheryte;
            {
                let mut game_data = game_data.lock();
                is_aetheryte = game_data.is_aetheryte(aetheryte_id);
            }

            if is_aetheryte {
                generic_lua_event("events/generic/Aetheryte.lua")
            } else {
                generic_lua_event("events/generic/AethernetShard.lua")
            }
        }
        HandlerType::Craft => Some(Box::new(CraftingEventHandler::new())),
        HandlerType::GuildLeveAssignment => generic_lua_event("events/generic/Levemete.lua"),
        HandlerType::CustomTalk => {
            let script_name;
            {
                let mut game_data = game_data.lock();
                script_name = game_data.get_custom_talk_name(handler_id.0);
            }
            let Ok(script_id) = extract_script_id(&script_name) else {
                tracing::error!(
                    "Unable to obtain script id from script {script_name}, the event cannot continue!"
                );
                return None;
            };

            let script_folder = folder_from_script_id(script_id);
            let script_path = format!("events/custom/{script_folder}/{script_name}.lua");

            generic_lua_event(&script_path)
        }
        HandlerType::GimmickAccessor => Some(Box::new(GimmickAccessorEventHandler::new())),
        HandlerType::GimmickBill => generic_lua_event("events/generic/GimmickBill.lua"),
        // NOTE: This is only applicable to instance exits for now.
        HandlerType::GimmickRect => generic_lua_event("events/generic/InstanceExit.lua"),
        HandlerType::ChocoboTaxiStand => generic_lua_event("events/generic/Chocobokeep.lua"),
        HandlerType::Opening => {
            let script_name;
            {
                let mut game_data = game_data.lock();
                script_name = game_data.get_opening_name(handler_id.0);
            }

            generic_lua_event(&format!("events/quest/opening/{script_name}.lua"))
        }
        HandlerType::ExitRange => generic_lua_event("events/generic/ExitRange.lua"),
        HandlerType::Fishing => Some(Box::new(FishingEventHandler::new())),
        HandlerType::SwitchTalk => generic_lua_event("events/generic/SwitchTalk.lua"),
        HandlerType::GoldSaucerArcadeMachine => {
            generic_lua_event("events/generic/GoldSaucerArcadeMachine.lua")
        }
        HandlerType::GoldSaucerTalk => generic_lua_event("events/generic/GoldSaucerTalk.lua"),
        HandlerType::DpsChallengeOfficer => {
            generic_lua_event("events/generic/DpsChallengeOfficer.lua")
        }
        HandlerType::TopicSelect => generic_lua_event("events/generic/TopicSelect.lua"),
        HandlerType::PreHandler => generic_lua_event("events/generic/PreHandler.lua"),
        HandlerType::Description => generic_lua_event("events/generic/Description.lua"),
        HandlerType::InclusionShop => Some(Box::new(InclusionShopEventHandler::new())),
        HandlerType::EventGimmickPathMove => {
            generic_lua_event("events/generic/GimmickPathMove.lua")
        }
        HandlerType::HousingAethernet => generic_lua_event("events/generic/HousingAethernet.lua"),
        HandlerType::InstanceContent => Some(Box::new(InstanceContentEventHandler::new())),
        _ => None,
    }
}

// Ensure that the script names we retrieve can be turned into sensible ids and folders.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_script_id() {
        // First ensure it succeeded.
        match extract_script_id("CmnDefBeginnerGuide_00327") {
            Ok(id) => assert_eq!(id, 327),
            Err(_) => assert!(false), // We don't care about the error, anything going wrong is an automatic test failure
        }

        // Next, fail intentionally with a bogus string and ensure it indeed failed.
        let fail_case = extract_script_id("CmnDefBeginnerGuide_XYZAB");
        assert!(fail_case.is_err());
    }

    #[test]
    fn test_folder_from_script_id() {
        assert_eq!(folder_from_script_id(327), "003");
    }
}
