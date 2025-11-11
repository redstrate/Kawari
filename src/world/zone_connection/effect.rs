//! Status effect list handling.

use mlua::Function;

use crate::{
    common::ObjectId,
    config::get_config,
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, ServerZoneIpcData, ServerZoneIpcSegment,
        StatusEffect, StatusEffectList,
    },
    world::{
        ZoneConnection,
        lua::{ExtraLuaState, LuaPlayer},
    },
};

impl ZoneConnection {
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

        // then send the actor control to lose the effect
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::LoseEffect {
                effect_id: effect_id as u32,
                unk2: effect_param as u32,
                source_actor_id: effect_source_actor_id,
            },
        })
        .await;
    }

    pub async fn process_effects_list(&mut self) {
        // Only update the client if absolutely necessary (e.g. an effect is added, removed or changed duration)
        if self.status_effects.dirty {
            let mut list = [StatusEffect::default(); 30];
            list[..self.status_effects.status_effects.len()]
                .copy_from_slice(&self.status_effects.status_effects);

            let ipc;
            {
                let game_data = self.gamedata.lock();

                ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StatusEffectList(
                    StatusEffectList {
                        statues: list,
                        classjob_id: self.player_data.classjob_id,
                        level: self.current_level(&game_data) as u8,
                        curr_hp: self.player_data.curr_hp,
                        max_hp: self.player_data.max_hp,
                        curr_mp: self.player_data.curr_mp,
                        max_mp: self.player_data.max_mp,
                        ..Default::default()
                    },
                ));
            }
            self.send_ipc_self(ipc).await;

            self.status_effects.dirty = false;
        }
    }
}
