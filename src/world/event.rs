use mlua::{Function, Lua};

use crate::{common::ObjectTypeId, config::get_config};

use super::{LuaPlayer, Zone};

pub struct Event {
    lua: Lua,
}

impl Event {
    pub fn new(id: u32, path: &str) -> Self {
        let lua = Lua::new();

        let config = get_config();
        let file_name = format!("{}/{}", &config.world.scripts_location, path);
        lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
            .set_name("@".to_string() + &file_name)
            .exec()
            .unwrap();

        lua.globals().set("EVENT_ID", id).unwrap();

        Self { lua }
    }

    pub fn enter_territory(&mut self, player: &mut LuaPlayer, zone: &Zone) {
        self.lua
            .scope(|scope| {
                let player = scope.create_userdata_ref_mut(player).unwrap();
                let zone = scope.create_userdata_ref(zone).unwrap();

                let func: Function = self.lua.globals().get("onEnterTerritory").unwrap();

                func.call::<()>((player, zone)).unwrap();

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

    pub fn talk(&mut self, target_id: ObjectTypeId, player: &mut LuaPlayer) {
        self.lua
            .scope(|scope| {
                let player = scope.create_userdata_ref_mut(player).unwrap();

                let func: Function = self.lua.globals().get("onTalk").unwrap();

                func.call::<()>((target_id, player)).unwrap();

                Ok(())
            })
            .unwrap();
    }

    pub fn finish(&mut self, scene: u16, results: &[u32], player: &mut LuaPlayer) {
        self.lua
            .scope(|scope| {
                let player = scope.create_userdata_ref_mut(player).unwrap();

                let func: Function = self.lua.globals().get("onReturn").unwrap();

                func.call::<()>((scene, results, player)).unwrap();

                Ok(())
            })
            .unwrap();
    }
}
