use mlua::{Function, Lua};

use crate::{common::ObjectTypeId, config::get_config};

use super::{LuaPlayer, Zone};

pub struct Event {
    pub file_name: String,
    lua: Lua,
    pub id: u32,
}

impl Event {
    pub fn new(id: u32, path: &str) -> Self {
        let lua = Lua::new();

        let config = get_config();
        let file_name = format!("{}/{}", &config.world.scripts_location, path);

        let result = std::fs::read(&file_name);
        if let Err(err) = std::fs::read(&file_name) {
            tracing::warn!("Failed to load {}: {:?}", file_name, err);
            return Self { file_name, lua, id };
        }
        let file = result.unwrap();

        if let Err(err) = lua.load(file).set_name("@".to_string() + &file_name).exec() {
            tracing::warn!("Syntax error in {}: {:?}", file_name, err);
            return Self { file_name, lua, id };
        }

        lua.globals().set("EVENT_ID", id).unwrap();

        Self { file_name, lua, id }
    }

    pub fn enter_territory(&mut self, player: &mut LuaPlayer, zone: &Zone) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;
                let zone = scope.create_userdata_ref(zone)?;

                let func: Function = self.lua.globals().get("onEnterTerritory")?;

                func.call::<()>((player, zone))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error in {}: {:?}", self.file_name, err);
        }
    }

    pub fn scene_finished(&mut self, player: &mut LuaPlayer, scene: u16) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onSceneFinished")?;

                func.call::<()>((player, scene))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error in {}: {:?}", self.file_name, err);
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
            tracing::warn!("Syntax error in {}: {:?}", self.file_name, err);
        }
    }

    pub fn finish(&mut self, scene: u16, results: &[u32], player: &mut LuaPlayer) {
        let mut run_script = || {
            self.lua.scope(|scope| {
                let player = scope.create_userdata_ref_mut(player)?;

                let func: Function = self.lua.globals().get("onReturn")?;

                func.call::<()>((scene, results, player))?;

                Ok(())
            })
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error in {}: {:?}", self.file_name, err);
        }
    }
}
