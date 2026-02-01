use std::sync::Arc;

use mlua::{Function, Lua};
use parking_lot::Mutex;

use kawari::{
    common::{HandlerId, HandlerType, ObjectTypeId},
    config::get_config,
    ipc::zone::{Condition, EventType},
};

use crate::GameData;

use super::lua::{LuaPlayer, initial_setup};

#[derive(Debug, Clone)]
pub struct Event {
    pub file_name: String,
    lua: Lua,
    pub id: u32,
    pub event_type: EventType,
    pub event_arg: u32,
    /// The condition set for this event.
    pub condition: Option<Condition>,
}

impl Event {
    pub fn new(id: u32, path: &str, game_data: mlua::Value) -> Option<Self> {
        let mut lua = Lua::new();
        initial_setup(&mut lua);

        // "steal"" the game data global from the other lua state
        let game_data = match game_data {
            mlua::Value::UserData(ud) => ud.borrow::<Arc<Mutex<GameData>>>().unwrap().clone(),
            _ => unreachable!(),
        };

        // inject parameters as necessary
        {
            let mut game_data = game_data.lock();
            Self::inject_lua_parameters(HandlerId(id), &mut lua, &mut game_data);
        }

        let config = get_config();
        let file_name = format!("{}/{}", &config.world.scripts_location, path);

        let result = std::fs::read(&file_name);
        if let Err(err) = std::fs::read(&file_name) {
            tracing::warn!("Failed to load {}: {:?}", file_name, err);
            return None;
        }
        let file = result.unwrap();

        if let Err(err) = lua.load(file).set_name("@".to_string() + &file_name).exec() {
            tracing::warn!("Syntax error in {}: {:?}", file_name, err);
            return None;
        }

        lua.globals().set("EVENT_ID", id).unwrap();
        lua.globals().set("GAME_DATA", game_data).unwrap();

        // The event_type/event_arg is set later, so don't care about this value we set!
        Some(Self {
            file_name,
            lua,
            event_type: EventType::Talk,
            id,
            event_arg: 0,
            condition: None,
        })
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
            _ => Vec::new(),
        };

        tracing::info!("Variables available in event {id}:");
        for (name, value) in &variables {
            lua.globals().set(&**name, *value).unwrap();
            tracing::info!("- {name}: {value}");
        }
    }

    // TODO: this is a terrible hold-over name. what it actually is an onStart function that's really only useful for cutscenes.
    pub fn enter_territory(&mut self, player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player_data = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onEnterTerritory")?;

                func.call::<()>(player_data)?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error during enter_territory in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    pub fn enter_trigger(&mut self, player: &mut LuaPlayer, arg: u32) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onEnterTrigger")?;

                func.call::<()>((player, arg))?;

                Ok(())
            })
        };

        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error during enter_trigger in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    pub fn talk(&mut self, target_id: ObjectTypeId, player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onTalk")?;

                func.call::<()>((target_id, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during talk in {}: {:?}", self.file_name, err);
        }
    }

    pub fn finish(&mut self, scene: u16, results: &[i32], player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onYield")?;

                func.call::<()>((scene, results, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error during finish in {}: {:?}",
                self.file_name,
                err
            );
        }
    }

    pub fn do_return(&mut self, scene: u16, results: &[i32], player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onReturn")?;

                func.call::<()>((scene, results, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!(
                "Syntax error during return in {}: {:?}",
                self.file_name,
                err
            );
        }
    }
}
