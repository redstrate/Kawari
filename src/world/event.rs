use mlua::{Function, Lua};

use crate::config::get_config;

use super::LuaPlayer;

pub struct Event {
    lua: Lua,
}

impl Event {
    pub fn new(path: &str) -> Self {
        let lua = Lua::new();

        let config = get_config();
        let file_name = format!("{}/{}", &config.world.scripts_location, path);
        lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
            .set_name("@".to_string() + &file_name)
            .exec()
            .unwrap();

        Self { lua }
    }

    pub fn enter_territory(&mut self, player: &mut LuaPlayer) {
        self.lua
            .scope(|scope| {
                let player = scope.create_userdata_ref_mut(player).unwrap();

                let func: Function = self.lua.globals().get("onEnterTerritory").unwrap();

                func.call::<()>(player).unwrap();

                Ok(())
            })
            .unwrap();
    }

    pub fn scene_finished(&mut self, player: &mut LuaPlayer, scene: u16) {
        self.lua
            .scope(|scope| {
                let player = scope.create_userdata_ref_mut(player).unwrap();

                let func: Function = self.lua.globals().get("onSceneFinished").unwrap();

                func.call::<()>((player, scene)).unwrap();

                Ok(())
            })
            .unwrap();
    }
}
