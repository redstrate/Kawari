use std::sync::Arc;

use async_trait::async_trait;
use kawari::{
    common::{HandlerId, HandlerType, ObjectTypeId},
    config::FilesystemConfig,
    ipc::zone::Condition,
};
use mlua::{Function, Lua};
use parking_lot::Mutex;

use crate::{
    Event, EventHandler, GameData, ZoneConnection,
    lua::{KawariLua, LuaPlayer},
};

const MAX_SCRIPT_REDIRECTS: usize = 8;

/// For events implemented in Lua scripts.
#[derive(Debug)]
pub struct LuaEventHandler {
    pub file_name: String,
    lua: KawariLua,
}

impl LuaEventHandler {
    pub fn new(
        id: HandlerId,
        base_id: Option<u32>,
        path: &str,
        game_data: Arc<Mutex<GameData>>,
    ) -> Option<Self> {
        let mut lua = KawariLua::new();

        // inject parameters as necessary
        {
            let mut game_data = game_data.lock();
            Self::inject_lua_parameters(id, &mut lua.0, &mut game_data);
        }

        let Some((file_name, file)) = Self::load_script(path) else {
            return None;
        };

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
        if let Some(base_id) = base_id {
            lua.0.globals().set("BASE_ID", base_id).unwrap();
        }
        lua.0.globals().set("GAME_DATA", game_data).unwrap();

        Some(Self { file_name, lua })
    }

    fn load_script(path: &str) -> Option<(String, Vec<u8>)> {
        let mut path = path.to_string();

        for _ in 0..MAX_SCRIPT_REDIRECTS {
            let file_name = FilesystemConfig::locate_script_file(&path);
            let result = std::fs::read(&file_name);
            if let Err(err) = result {
                tracing::warn!("Failed to load {}: {:?}", file_name, err);
                return None;
            }
            let file = result.unwrap();

            let Some(redirect_path) = Self::script_redirect_target(&path, &file) else {
                return Some((file_name, file));
            };

            tracing::debug!("Lua event script {path} redirects to {redirect_path}");
            path = redirect_path;
        }

        tracing::warn!("Lua event script {path} exceeded the redirect limit");
        None
    }

    fn script_redirect_target(path: &str, file: &[u8]) -> Option<String> {
        let source = std::str::from_utf8(file).ok()?;
        let redirect = source.trim().trim_start_matches('\u{feff}').trim();
        if !redirect.ends_with(".lua")
            || redirect.is_empty()
            || redirect.chars().any(char::is_whitespace)
        {
            return None;
        }

        let redirect = redirect.replace('\\', "/");
        if redirect.starts_with('/') || redirect.split('/').any(|part| part == "..") {
            return None;
        }

        if redirect.contains('/') {
            return Some(redirect);
        }

        let Some((parent, _)) = path.rsplit_once('/') else {
            return Some(redirect);
        };

        Some(format!("{parent}/{redirect}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_single_line_lua_redirect_relative_to_current_script() {
        let redirect = LuaEventHandler::script_redirect_target(
            "events/warp/WarpInnGridania.lua",
            b"WarpInnGeneric.lua\r\n",
        );

        assert_eq!(redirect, Some("events/warp/WarpInnGeneric.lua".to_string()));
    }

    #[test]
    fn ignores_normal_lua_scripts() {
        let redirect = LuaEventHandler::script_redirect_target(
            "events/warp/WarpInnGeneric.lua",
            b"function onTalk(target, player)\nend\n",
        );

        assert_eq!(redirect, None);
    }

    #[test]
    fn rejects_redirects_outside_scripts() {
        let redirect =
            LuaEventHandler::script_redirect_target("events/warp/Foo.lua", b"../Bar.lua\n");

        assert_eq!(redirect, None);
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

    fn condition(&self) -> Condition {
        self.lua
            .0
            .globals()
            .get("CONDITION")
            .unwrap_or(Condition::OccupiedInQuestEvent)
    }
}
