use std::{collections::HashMap, fmt::Display, sync::Arc};

use bitflags::Flags;
use mlua::{IntoLua, Lua};
use parking_lot::Mutex;
use strum::IntoEnumIterator;

use crate::{Event, GameData, inventory::CurrencyKind};
use kawari::{
    common::HandlerType,
    config::get_config,
    ipc::zone::{EventType, GameMasterRank, SceneFlags, ServerNoticeFlags},
};

use super::EffectsBuilder;

#[derive(Default)]
pub struct ExtraLuaState {
    pub action_scripts: HashMap<u32, String>,
    pub command_scripts: HashMap<String, String>,
    pub gm_command_scripts: HashMap<u32, String>,
    pub effect_scripts: HashMap<u32, String>,
    pub zone_eobj_scripts: HashMap<u32, String>,
}

/// Perform initial setup of global constants.
pub fn initial_setup(lua: &mut Lua) {
    // TODO: we should use a global static here so we can define this at the enum level
    // Specifically something like the linkme crate
    register_flags::<ServerNoticeFlags>(lua, "SERVER_NOTICE");
    register_enum::<GameMasterRank>(lua, "GM_RANK");
    register_flags::<SceneFlags>(lua, ""); // TODO: might want to prefix these at some point
    register_enum::<EventType>(lua, "EVENT_TYPE");
    register_enum::<HandlerType>(lua, "HANDLER_TYPE");
    register_enum::<CurrencyKind>(lua, "CURRENCY");

    // Load globals
    let config = get_config();
    let file_name = format!("{}/Global.lua", &config.world.scripts_location);
    lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
        .exec()
        .unwrap();
}

/// Loads `Init.lua`
pub fn load_init_script(lua: &mut Lua, game_data: Arc<Mutex<GameData>>) -> mlua::Result<()> {
    initial_setup(lua);

    let register_action_func =
        lua.create_function(|lua, (action_id, action_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.action_scripts.insert(action_id, action_script);
            Ok(())
        })?;

    let register_command_func =
        lua.create_function(|lua, (command_name, command_script): (String, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.command_scripts.insert(command_name, command_script);
            Ok(())
        })?;

    let register_gm_command_func =
        lua.create_function(|lua, (command_type, command_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state
                .gm_command_scripts
                .insert(command_type, command_script);
            Ok(())
        })?;

    let get_login_message_func = lua.create_function(|_, _: ()| {
        let config = get_config();
        Ok(config.world.login_message)
    })?;

    let run_event_func = lua.create_function(|lua, (event_id, event_script): (u32, String)| {
        Ok(Event::new(
            event_id,
            &event_script,
            lua.globals().get("GAME_DATA").unwrap(),
        ))
    })?;

    let run_action_func =
        lua.create_function(|_, (action_script, arg): (String, u32)| Ok((action_script, arg)))?;

    let mut extra_lua_state = ExtraLuaState::default();

    let config = get_config();

    let load_based_on_filename = |name: &str, hash_map: &mut HashMap<u32, String>| {
        let effects_dir = format!("{}/{name}", &config.world.scripts_location);
        for entry in std::fs::read_dir(effects_dir)
            .expect("Didn't find effects directory?")
            .flatten()
        {
            for entry in std::fs::read_dir(entry.path())
                .expect("Failed to read into effects directory")
                .flatten()
            {
                let path = entry.path();
                if path.extension().and_then(|x| x.to_str()) == Some("lua") {
                    let stem = path
                        .file_stem()
                        .expect("No file name?!")
                        .to_str()
                        .expect("Failed to convert filename")
                        .to_string();
                    let Some((_, num)) = stem.split_once('_') else {
                        tracing::warn!("Invalid status effect file name: {stem}");
                        continue;
                    };
                    let num = num.parse().expect("Failed to parse status effect ID");
                    hash_map.insert(
                        num,
                        path.strip_prefix(&config.world.scripts_location)
                            .expect("Failed to express scripts location")
                            .to_str()
                            .expect("Failed to convert path")
                            .to_string(),
                    );
                }
            }
        }
    };

    // Locate these based on the ID in their filename
    load_based_on_filename("effects", &mut extra_lua_state.effect_scripts);
    load_based_on_filename("actions", &mut extra_lua_state.action_scripts);

    lua.set_app_data(extra_lua_state);
    lua.globals().set("registerAction", register_action_func)?;
    lua.globals()
        .set("registerCommand", register_command_func)?;
    lua.globals()
        .set("registerGMCommand", register_gm_command_func)?;
    lua.globals()
        .set("getLoginMessage", get_login_message_func)?;
    lua.globals().set("runEvent", run_event_func)?;
    lua.globals().set("runAction", run_action_func)?;

    let effectsbuilder_constructor = lua.create_function(|_, ()| Ok(EffectsBuilder::default()))?;
    lua.globals()
        .set("EffectsBuilder", effectsbuilder_constructor)?;

    lua.globals().set("GAME_DATA", game_data.clone())?;

    let file_name = format!("{}/Init.lua", &config.world.scripts_location);
    lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
        .set_name("@".to_string() + &file_name)
        .exec()?;

    Ok(())
}

/// Registers bitflags into the Lua state. All values are prefixed with `prefix`.
pub fn register_flags<T: Flags<Bits: IntoLua>>(lua: &mut Lua, prefix: &str) {
    for variant in T::FLAGS {
        let new_name = if prefix.is_empty() {
            variant.name().to_string()
        } else {
            format!("{prefix}_{}", variant.name())
        };
        lua.globals().set(new_name, variant.value().bits()).unwrap();
    }
}

/// Registers enum into the Lua state. All values are prefixed with `prefix`.
pub fn register_enum<T: IntoEnumIterator + IntoLua + Display>(lua: &mut Lua, prefix: &str) {
    for variant in T::iter() {
        let new_name = if prefix.is_empty() {
            variant.to_string()
        } else {
            format!("{prefix}_{variant}")
        };
        lua.globals().set(new_name, variant).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use bitflags::bitflags;
    use strum_macros::{Display, EnumIter};

    bitflags! {
        struct DisplayFlag : u32 {
            const NONE = 0x000;
            const ACTIVE_STANCE = 0x001;
            const OTHER_STANCE = 0x002;
        }
    }

    #[test]
    fn test_register_flags() {
        let mut lua = Lua::new();
        register_flags::<DisplayFlag>(&mut lua, "DISPLAY_FLAG");

        assert_eq!(
            lua.load("return DISPLAY_FLAG_NONE")
                .call::<u32>(())
                .unwrap(),
            0
        );
        assert_eq!(
            lua.load("return DISPLAY_FLAG_ACTIVE_STANCE")
                .call::<u32>(())
                .unwrap(),
            1
        );
        assert_eq!(
            lua.load("return DISPLAY_FLAG_ACTIVE_STANCE + DISPLAY_FLAG_OTHER_STANCE")
                .call::<u32>(())
                .unwrap(),
            3
        );
    }

    #[test]
    fn test_register_flags_empty_prefix() {
        let mut lua = Lua::new();
        register_flags::<DisplayFlag>(&mut lua, "");

        assert_eq!(lua.load("return NONE").call::<u32>(()).unwrap(), 0);
        assert_eq!(lua.load("return ACTIVE_STANCE").call::<u32>(()).unwrap(), 1);
        assert_eq!(
            lua.load("return ACTIVE_STANCE + OTHER_STANCE")
                .call::<u32>(())
                .unwrap(),
            3
        );
    }

    #[repr(u32)]
    #[derive(Display, EnumIter)]
    #[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
    enum GMRanks {
        Lesser = 0,
        Upper = 1,
        Master = 2,
    }

    impl IntoLua for GMRanks {
        fn into_lua(self, _: &Lua) -> mlua::Result<mlua::Value> {
            Ok(mlua::Value::Integer(self as i64))
        }
    }

    #[test]
    fn test_register_enum() {
        let mut lua = Lua::new();
        register_enum::<GMRanks>(&mut lua, "GM_RANK");

        assert_eq!(
            lua.load("return GM_RANK_LESSER").call::<u32>(()).unwrap(),
            0
        );
        assert_eq!(lua.load("return GM_RANK_UPPER").call::<u32>(()).unwrap(), 1);
        assert_eq!(
            lua.load("return GM_RANK_MASTER").call::<u32>(()).unwrap(),
            2
        );
    }

    #[test]
    fn test_register_enum_empty_prefix() {
        let mut lua = Lua::new();
        register_enum::<GMRanks>(&mut lua, "");

        assert_eq!(lua.load("return LESSER").call::<u32>(()).unwrap(), 0);
        assert_eq!(lua.load("return UPPER").call::<u32>(()).unwrap(), 1);
        assert_eq!(lua.load("return MASTER").call::<u32>(()).unwrap(), 2);
    }
}
