use std::collections::HashMap;

use mlua::Lua;

use crate::config::get_config;

use super::EffectsBuilder;

#[derive(Default)]
pub struct ExtraLuaState {
    pub action_scripts: HashMap<u32, String>,
    pub event_scripts: HashMap<u32, String>,
    pub command_scripts: HashMap<String, String>,
    pub gm_command_scripts: HashMap<u32, String>,
    pub effect_scripts: HashMap<u32, String>,
    pub zone_eobj_scripts: HashMap<u32, String>,
}

/// Loads `Init.lua`
pub fn load_init_script(lua: &mut Lua) -> mlua::Result<()> {
    let register_action_func =
        lua.create_function(|lua, (action_id, action_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.action_scripts.insert(action_id, action_script);
            Ok(())
        })?;

    let register_event_func =
        lua.create_function(|lua, (event_id, event_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.event_scripts.insert(event_id, event_script);
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

    let register_effects_func =
        lua.create_function(|lua, (command_type, status_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.effect_scripts.insert(command_type, status_script);
            Ok(())
        })?;

    let register_zone_eobjs_func =
        lua.create_function(|lua, (zone_id, zone_eobj_script): (u32, String)| {
            let mut state = lua.app_data_mut::<ExtraLuaState>().unwrap();
            let _ = state.zone_eobj_scripts.insert(zone_id, zone_eobj_script);
            Ok(())
        })?;

    let get_login_message_func = lua.create_function(|_, _: ()| {
        let config = get_config();
        Ok(config.world.login_message)
    })?;

    lua.set_app_data(ExtraLuaState::default());
    lua.globals().set("registerAction", register_action_func)?;
    lua.globals().set("registerEvent", register_event_func)?;
    lua.globals()
        .set("registerCommand", register_command_func)?;
    lua.globals()
        .set("registerGMCommand", register_gm_command_func)?;
    lua.globals().set("registerEffect", register_effects_func)?;
    lua.globals()
        .set("registerZoneEObjs", register_zone_eobjs_func)?;
    lua.globals()
        .set("getLoginMessage", get_login_message_func)?;

    let effectsbuilder_constructor = lua.create_function(|_, ()| Ok(EffectsBuilder::default()))?;
    lua.globals()
        .set("EffectsBuilder", effectsbuilder_constructor)?;

    let config = get_config();
    let file_name = format!("{}/Init.lua", &config.world.scripts_location);
    lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
        .set_name("@".to_string() + &file_name)
        .exec()?;

    Ok(())
}
