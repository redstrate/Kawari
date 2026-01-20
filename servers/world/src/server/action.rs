//! Executing actions and other related functions.

use std::{sync::Arc, time::Duration};

use mlua::{Function, Lua};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, PlayerData, StatusEffects, ToServer,
    lua::{EffectsBuilder, ExtraLuaState, LuaPlayer, LuaZone},
    server::{
        WorldServer,
        actor::NetworkedActor,
        effect::gain_effect,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
};
use kawari::{
    common::{CharacterMode, DEAD_FADE_OUT_TIME, INVALID_OBJECT_ID, ObjectId},
    config::get_config,
    ipc::zone::{
        ActionEffect, ActionKind, ActionRequest, ActionResult, ActorControlCategory, EffectEntry,
        EffectKind, EffectResult, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

/// Process action-related messages.
pub fn handle_action_messages(
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    msg: &ToServer,
) {
    if let ToServer::ActionRequest(from_id, from_actor_id, request) = msg {
        let cast_time;
        {
            let mut game_data = game_data.lock();
            cast_time = game_data.get_casttime(request.action_key).unwrap();
        }

        let delay_milliseconds = cast_time as u64 * 100;

        tracing::info!(
            "Delaying spell cast for {} milliseconds",
            delay_milliseconds
        );

        let mut data = data.lock();
        let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
            return;
        };

        instance.insert_task(
            *from_id,
            *from_actor_id,
            Duration::from_millis(delay_milliseconds),
            QueuedTaskData::CastAction {
                request: request.clone(),
                interruptible: delay_milliseconds > 0,
            },
        );
    }
}

/// Executes an action, and returns a list of Tasks that must be executed by the client.
pub fn execute_action(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    lua: Arc<Mutex<Lua>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
    request: ActionRequest,
) {
    let mut lua_player = LuaPlayer {
        player_data: PlayerData::default(),
        status_effects: StatusEffects::default(),
        queued_tasks: Vec::new(),
        zone_data: LuaZone::default(),
    };

    let effects_builder;
    let common_spawn;
    {
        let data = data.lock();

        let Some(instance) = data.find_actor_instance(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        lua_player.player_data.teleport_query = match actor {
            NetworkedActor::Player { teleport_query, .. } => teleport_query.clone(),
            _ => unreachable!(),
        };

        common_spawn = actor.get_common_spawn().clone();

        effects_builder = match &request.action_kind {
            ActionKind::Nothing => None,
            ActionKind::Normal => execute_normal_action(lua.clone(), &request, &mut lua_player),
            ActionKind::Item => {
                execute_item_action(game_data.clone(), lua.clone(), &request, &mut lua_player)
            }
            ActionKind::Mount => {
                execute_mount_action(network.clone(), from_id, from_actor_id, &request, actor)
            }
        };
    }

    // tell them the action results
    if let Some(effects_builder) = effects_builder {
        // Update our internal data model to their new HP
        {
            let mut data = data.lock();

            let Some(instance) = data.find_actor_instance_mut(request.target.object_id) else {
                return;
            };

            let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                return;
            };

            let common_spawn = actor.get_common_spawn_mut();

            for effect in &effects_builder.effects {
                if let EffectKind::Damage { amount, .. } = effect.kind {
                    common_spawn.hp = common_spawn.hp.saturating_sub(amount as u32);
                }
            }

            update_actor_hp_mp(network.clone(), instance, request.target.object_id);
        }

        // TODO: send Cooldown ActorControlSelf

        {
            let mut network = network.lock();

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
                        rotation: common_spawn.rotation,
                        action_animation_id: request.action_key as u16, // assuming action id == animation id
                        flag: 1,
                        effect_count: effects_builder.effects.len() as u8,
                        effects,
                        unk1: 2662353,
                        unk2: 3758096384,
                        hidden_animation: 1,
                        ..Default::default()
                    }));
                network.send_ipc_to(from_id, ipc, from_actor_id);
            }
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

                    gain_effect(
                        network.clone(),
                        data.clone(),
                        from_id,
                        from_actor_id,
                        effect_id,
                        param,
                        duration,
                        source_actor_id,
                        false,
                    ); // ACS isn't needed as EffectsResult will show it for us
                }

                // To lose effects, we just omit them from the list but increase the entry count!
                if let EffectKind::LoseEffect { .. } = effect.kind {
                    entries[num_entries as usize] = EffectEntry::default();
                    num_entries += 1;

                    //self.status_effects.remove(effect_id);
                }
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                unk1: 1,
                unk2: 776386,
                target_id: request.target.object_id,
                current_hp: common_spawn.hp,
                max_hp: common_spawn.max_hp,
                current_mp: common_spawn.mp,
                unk3: 0,
                class_id: common_spawn.class_job,
                shield: 0,
                entry_count: num_entries,
                unk4: 0,
                statuses: entries,
            }));
            let mut network = network.lock();
            network.send_ipc_to(from_id, ipc, from_actor_id);
        }
    }

    let mut network = network.lock();
    let msg = FromServer::NewTasks(lua_player.queued_tasks);
    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
}

pub fn cancel_action(network: Arc<Mutex<NetworkState>>, from_id: ClientId) {
    let msg = FromServer::ActorControlSelf(ActorControlCategory::CancelCast {});

    {
        let mut network = network.lock();
        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
    }
}

/// Handles normal actions, powered by Lua.
pub fn execute_normal_action(
    lua: Arc<Mutex<Lua>>,
    request: &ActionRequest,
    lua_player: &mut LuaPlayer,
) -> Option<EffectsBuilder> {
    let mut effects_builder = None;
    let lua = lua.lock();
    let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

    let key = request.action_key;
    if let Some(action_script) = state.action_scripts.get(&key) {
        lua.scope(|scope| {
            let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

            let config = get_config();

            let file_name = format!("{}/{}", &config.world.scripts_location, action_script);
            lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
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

    effects_builder
}

/// Handles item actions, powered by Lua.
pub fn execute_item_action(
    game_data: Arc<Mutex<GameData>>,
    lua: Arc<Mutex<Lua>>,
    request: &ActionRequest,
    lua_player: &mut LuaPlayer,
) -> Option<EffectsBuilder> {
    let lua = lua.lock();

    let key = request.action_key;
    let (action_type, action_data, additional_data);

    {
        let mut gamedata = game_data.lock();
        (action_type, action_data, additional_data) =
            gamedata.lookup_item_action_data(key).unwrap_or_default();
    }

    // FIXME: we should check if this data is valid instead of silently returning zeroes

    let mut effects_builder = None;
    lua.scope(|scope| {
        let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

        let func: Function = lua.globals().get("dispatchItem").unwrap();

        match func.call::<(String, u32)>((
            &connection_data,
            key,
            action_type,
            action_data,
            additional_data,
        )) {
            Ok((action_script, arg)) => {
                let config = get_config();

                let file_name = format!("{}/{}", &config.world.scripts_location, action_script);
                lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                let func: Function = lua.globals().get("doAction").unwrap();

                effects_builder =
                    Some(func.call::<EffectsBuilder>((connection_data, arg)).unwrap());
            }
            Err(err) => {
                tracing::error!("Error while calling dispatchItem: {:?}", err);
            }
        }

        Ok(())
    })
    .unwrap();

    effects_builder
}

/// Handles mount-related actions.
pub fn execute_mount_action(
    network: Arc<Mutex<NetworkState>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
    request: &ActionRequest,
    actor: &NetworkedActor,
) -> Option<EffectsBuilder> {
    let mut network = network.lock();

    let common_spawn = actor.get_common_spawn();

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
        rotation: common_spawn.rotation,
        action_animation_id: 4,
        flag: 13,
        effect_count: 1,
        effects,
        unk1: 4232092,
        unk2: 3758096384,
        hidden_animation: 4,
        ..Default::default()
    }));
    network.send_ipc_to(from_id, ipc, from_actor_id);

    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Mount {
        id: request.action_key as u16,
        unk1: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    network.send_ipc_to(from_id, ipc, from_actor_id);

    None
}

// Sends the ActorControls to inform the actor that they're dead.
pub fn kill_actor(network: Arc<Mutex<NetworkState>>, from_actor_id: ObjectId) {
    // TODO: set HP/MP to zero here

    // First, set their state (otherwise they can still walk)
    {
        // TODO: these should be ActorControlSelf if target_actor_id == from_actor_id
        let msg = FromServer::ActorControl(
            from_actor_id,
            ActorControlCategory::SetMode {
                mode: CharacterMode::Dead,
                mode_arg: 0,
            },
        );

        let mut network = network.lock();
        network.send_to_all(None, msg, DestinationNetwork::ZoneClients);
    }

    // Then, play the death animation.
    {
        let msg = FromServer::ActorControl(
            from_actor_id,
            ActorControlCategory::Kill { animation_id: 0 },
        );

        let mut network = network.lock();
        network.send_to_all(None, msg, DestinationNetwork::ZoneClients);
    }
}

/// Updates other actors about this actor's HP and MP.
pub fn update_actor_hp_mp(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    target_actor_id: ObjectId,
) {
    let Some(actor) = instance.find_actor_mut(target_actor_id) else {
        return;
    };

    let common_spawn = actor.get_common_spawn();

    // Inform the client of the new actor's HP/MP
    {
        // TODO: send to all relevant players
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp {
            hp: common_spawn.hp,
            mp: common_spawn.max_mp,
            unk: 0,
        });
        let mut network = network.lock();
        network.send_to_all(
            None,
            FromServer::PacketSegment(ipc, target_actor_id),
            DestinationNetwork::ZoneClients,
        );
    }

    if common_spawn.hp == 0 {
        kill_actor(network.clone(), target_actor_id);

        // Queue up despawn if this is an NPC
        if !matches!(actor, NetworkedActor::Player { .. }) {
            instance.insert_task(
                ClientId::default(),
                target_actor_id,
                DEAD_FADE_OUT_TIME,
                QueuedTaskData::DeadFadeOut {
                    actor_id: target_actor_id,
                },
            );
        }
    }
}
