use std::sync::Arc;

use async_trait::async_trait;
use kawari::common::{HandlerId, HandlerType, ObjectTypeId};
use mlua::{Function, Lua};
use parking_lot::Mutex;

use crate::{
    Event, EventHandler, GameData, ZoneConnection,
    lua::{KawariLua, LuaPlayer},
};

/// For events implemented in Lua scripts.
#[derive(Debug)]
pub struct LuaEventHandler {
    pub file_name: String,
    lua: KawariLua,
}

impl LuaEventHandler {
    pub fn new(id: HandlerId, path: &str, game_data: Arc<Mutex<GameData>>) -> Option<Self> {
        let mut lua = KawariLua::new();

        // inject parameters as necessary
        {
            let mut game_data = game_data.lock();
            Self::inject_lua_parameters(id, &mut lua.0, &mut game_data);
        }

        let file_name = format!("resources/scripts/{path}");

        let result = std::fs::read(&file_name);
        if let Err(err) = result {
            tracing::warn!("Failed to load {}: {:?}", file_name, err);
            return None;
        }
        let file = result.unwrap();

        if let Err(err) = lua
            .0
            .load(file)
            .set_name("@".to_string() + &file_name)
            .exec()
        {
            tracing::warn!("Syntax error in {}: {:?}", file_name, err);
            return None;
        }

        lua.0.globals().set("EVENT_ID", id.0).unwrap();
        lua.0.globals().set("GAME_DATA", game_data).unwrap();

        Some(Self { file_name, lua })
    }

    /// Injects any applicable Lua parameters from Excel, such as from `Opening`.
    fn inject_lua_parameters(id: HandlerId, lua: &mut Lua, gamedata: &mut GameData) {
        let variables = match id.handler_type() {
            HandlerType::Opening => {
                let opening_id = id.0;
                gamedata.get_opening_variables(opening_id)
            }
            HandlerType::Quest => {
                let quest_id = id.0;
                gamedata.get_quest_variables(quest_id)
            }
            HandlerType::CustomTalk => {
                let ct_id = id.0;
                gamedata.get_custom_talk_variables(ct_id)
            }
            // NOTE: ExitRange Lua script uses AetheryteSystemDefine variables, that's why it's here...
            HandlerType::Aetheryte | HandlerType::ExitRange => gamedata.get_aetheryte_variables(),
            _ => Vec::new(),
        };

        tracing::info!("Variables available in event {id}:");
        for (name, value) in &variables {
            lua.globals().set(&**name, *value).unwrap();
            tracing::info!("- {name}: {value}");
        }
    }
}

#[async_trait]
impl EventHandler for LuaEventHandler {
    async fn on_enter_territory(&self, _event: &Event, player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.0.scope(|scope| {
                let player_data = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.0.globals().get("onEnterTerritory")?;

                func.call::<()>(player_data)?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error while calling onEnterTerritory in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    async fn on_enter_trigger(&self, _event: &Event, player: &mut LuaPlayer, arg: u32) {
        let mut run_script = || {
            self.lua.0.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.0.globals().get("onEnterTrigger")?;

                func.call::<()>((player, arg))?;

                Ok(())
            })
        };

        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error while calling onEnterTrigger in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    async fn on_talk(&self, _event: &Event, target_id: ObjectTypeId, player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.0.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.0.globals().get("onTalk")?;

                func.call::<()>((target_id, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error while calling onTalk in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    async fn on_yield(
        &self,
        _event: &Event,
        _connection: &mut ZoneConnection,
        scene: u16,
        yield_id: u8,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        let mut run_script = || {
            self.lua.0.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.0.globals().get("onYield")?;

                func.call::<()>((scene, yield_id, results, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error while calling onYield in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    async fn on_return(
        &self,
        _event: &Event,
        _connection: &mut ZoneConnection,
        scene: u16,
        results: &[i32],
        player: &mut LuaPlayer,
    ) {
        let mut run_script = || {
            self.lua.0.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.0.globals().get("onReturn")?;

                func.call::<()>((scene, results, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error while calling onReturn in {}: {:?}",
                self.file_name,
                err
            );
        }
    }
}
