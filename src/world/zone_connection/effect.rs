//! Status effect list handling.

use mlua::Function;

use crate::{
    common::ObjectId,
    config::get_config,
    world::{
        ToServer, ZoneConnection,
        lua::{ExtraLuaState, LuaPlayer},
    },
};

impl ZoneConnection {
    pub async fn gain_effect(&mut self, effect_id: u16, effect_param: u16, effect_duration: f32) {
        // The server will update our state later
        self.handle
            .send(ToServer::GainEffect(
                self.id,
                self.player_data.actor_id,
                effect_id,
                effect_param,
                effect_duration,
                ObjectId(self.player_data.actor_id),
            ))
            .await;
    }

    pub async fn lose_effect(
        &mut self,
        effect_id: u16,
        effect_param: u16,
        effect_source_actor_id: ObjectId,
        lua_player: &mut LuaPlayer,
    ) {
        // first, inform the effect script
        {
            let lua = self.lua.lock();
            let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

            let key = effect_id as u32;
            if let Some(effect_script) = state.effect_scripts.get(&key) {
                lua.scope(|scope| {
                    let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

                    let config = get_config();

                    let file_name = format!("{}/{}", &config.world.scripts_location, effect_script);
                    lua.load(
                        std::fs::read(&file_name).expect("Failed to locate scripts directory!"),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                    let func: Function = lua.globals().get("onLose").unwrap();

                    func.call::<()>(connection_data).unwrap();

                    Ok(())
                })
                .unwrap();
            } else {
                tracing::warn!("Effect {effect_id} isn't scripted yet! Ignoring...");
            }
        }

        // The server will update our state later
        self.handle
            .send(ToServer::LoseEffect(
                self.id,
                self.player_data.actor_id,
                effect_id,
                effect_param,
                effect_source_actor_id,
            ))
            .await;
    }
}
