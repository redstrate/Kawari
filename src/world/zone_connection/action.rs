//! Executing and other action-related functions.

use mlua::Function;

use crate::{
    common::INVALID_OBJECT_ID,
    config::get_config,
    ipc::zone::{
        ActionEffect, ActionKind, ActionRequest, ActionResult, ActorControlCategory,
        ActorControlSelf, EffectEntry, EffectKind, EffectResult, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
    world::{
        ToServer, ZoneConnection,
        lua::{EffectsBuilder, ExtraLuaState, LuaPlayer},
    },
};

impl ZoneConnection {
    pub async fn execute_action(&mut self, request: ActionRequest, lua_player: &mut LuaPlayer) {
        if request.action_kind == ActionKind::Mount {
            let mut effects = [ActionEffect::default(); 8];
            effects[0] = ActionEffect {
                kind: EffectKind::Mount {
                    unk1: 1,
                    unk2: 0,
                    id: request.action_key as u16,
                },
            };

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                main_target: request.target,
                target_id_again: request.target,
                action_id: request.action_key,
                animation_lock_time: 0.1,
                rotation: self.player_data.rotation,
                action_animation_id: 4,
                flag: 13,
                effect_count: 1,
                effects,
                unk1: 4232092,
                unk2: 3758096384,
                hidden_animation: 4,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Mount {
                id: request.action_key as u16,
                unk1: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            });
            self.send_ipc_self(ipc).await;
            return;
        }

        let mut effects_builder = None;

        if request.action_kind == ActionKind::Item {
            let lua = self.lua.lock();

            let key = request.action_key;
            let (action_type, action_data, additional_data);

            {
                let mut gamedata = self.gamedata.lock();
                (action_type, action_data, additional_data) =
                    gamedata.lookup_item_action_data(key).unwrap_or_default();
            }

            // FIXME: we should check if this data is valid instead of silently returning zeroes

            lua.scope(|scope| {
                let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

                let func: Function = lua.globals().get("dispatchItem").unwrap();

                if let Ok((action_script, arg)) = func.call::<(String, u32)>((
                    &connection_data,
                    self.gamedata.clone(),
                    key,
                    action_type,
                    action_data,
                    additional_data,
                )) {
                    let config = get_config();

                    let file_name = format!("{}/{}", &config.world.scripts_location, action_script);
                    lua.load(
                        std::fs::read(&file_name).expect("Failed to locate scripts directory!"),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                    let func: Function = lua.globals().get("doAction").unwrap();

                    effects_builder =
                        Some(func.call::<EffectsBuilder>((connection_data, arg)).unwrap());
                }

                Ok(())
            })
            .unwrap();
        } else {
            let lua = self.lua.lock();
            let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

            let key = request.action_key;
            if let Some(action_script) = state.action_scripts.get(&key) {
                lua.scope(|scope| {
                    let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

                    let config = get_config();

                    let file_name = format!("{}/{}", &config.world.scripts_location, action_script);
                    lua.load(
                        std::fs::read(&file_name).expect("Failed to locate scripts directory!"),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                    let func: Function = lua.globals().get("doAction").unwrap();

                    effects_builder = Some(func.call::<EffectsBuilder>(connection_data).unwrap());

                    Ok(())
                })
                .unwrap();
            } else {
                tracing::warn!("Action {key} isn't scripted yet! Ignoring {:#?}", request);
            }
        }

        // tell them the action results
        if let Some(effects_builder) = effects_builder {
            if let Some(actor) = self.get_actor_mut(request.target.object_id) {
                for effect in &effects_builder.effects {
                    match effect.kind {
                        EffectKind::Damage { amount, .. } => {
                            actor.hp = actor.hp.saturating_sub(amount as u32);
                        }
                        _ => todo!(),
                    }
                }

                let actor = *actor;
                self.update_hp_mp(actor.id, actor.hp, 10000).await;
            }

            // TODO: send Cooldown ActorControlSelf

            // ActionResult
            {
                let mut effects = [ActionEffect::default(); 8];
                effects[..effects_builder.effects.len()].copy_from_slice(&effects_builder.effects);

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                        main_target: request.target,
                        target_id_again: request.target,
                        action_id: request.action_key,
                        animation_lock_time: 0.6,
                        rotation: self.player_data.rotation,
                        action_animation_id: request.action_key as u16, // assuming action id == animation id
                        flag: 1,
                        effect_count: effects_builder.effects.len() as u8,
                        effects,
                        unk1: 2662353,
                        unk2: 3758096384,
                        hidden_animation: 1,
                        ..Default::default()
                    }));
                self.send_ipc_self(ipc).await;
            }

            // EffectResult
            // TODO: is this always sent? needs investigation
            {
                let mut num_entries = 0u8;
                let mut entries = [EffectEntry::default(); 4];

                for effect in &effects_builder.effects {
                    if let EffectKind::GainEffect {
                        effect_id,
                        duration,
                        param,
                        source_actor_id,
                        ..
                    } = effect.kind
                    {
                        entries[num_entries as usize] = EffectEntry {
                            index: num_entries,
                            unk1: 0,
                            id: effect_id,
                            param,
                            unk2: 0,
                            duration,
                            source_actor_id: INVALID_OBJECT_ID,
                        };
                        num_entries += 1;

                        // also inform the server of our new status effect
                        self.handle
                            .send(ToServer::GainEffect(
                                self.id,
                                self.player_data.actor_id,
                                effect_id,
                                duration,
                                param,
                                source_actor_id,
                            ))
                            .await;

                        self.status_effects.add(effect_id, param, duration);
                    }

                    // To lose effects, we just omit them from the list but increase the entry count!
                    if let EffectKind::LoseEffect { effect_id, .. } = effect.kind {
                        entries[num_entries as usize] = EffectEntry::default();
                        num_entries += 1;

                        self.status_effects.remove(effect_id);
                    }
                }

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                        unk1: 1,
                        unk2: 776386,
                        target_id: request.target.object_id,
                        current_hp: self.player_data.curr_hp,
                        max_hp: self.player_data.max_hp,
                        current_mp: self.player_data.curr_mp,
                        unk3: 0,
                        class_id: self.player_data.classjob_id,
                        shield: 0,
                        entry_count: num_entries,
                        unk4: 0,
                        statuses: entries,
                    }));
                self.send_ipc_self(ipc).await;
            }

            if let Some(actor) = self.get_actor(request.target.object_id)
                && actor.hp == 0
            {
                tracing::info!("Despawning {} because they died!", actor.id.0);
                // if the actor died, despawn them
                /*connection.handle
                 *                                       .send(ToServer::ActorDespawned(connection.id, actor.id.0))
                 *                                       .await;*/
            }
        }
    }

    pub async fn cancel_action(&mut self) {
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::CancelCast {},
        })
        .await;
    }
}
