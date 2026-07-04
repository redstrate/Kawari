//! Executing status effect related functions.

use std::{sync::Arc, time::Duration};

use mlua::Function;
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, PlayerData, StatusEffects, ToServer,
    lua::{KawariLua, KawariLuaState, LuaContent, LuaPlayer, LuaZone},
    server::{
        WorldServer,
        combat_state::PlayerCombatState,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
    zone_connection::BaseParameters,
};
use kawari::{
    common::ObjectId,
    ipc::zone::{
        ActorControlCategory, ServerZoneIpcData, ServerZoneIpcSegment, StatusEffect,
        StatusEffectList,
    },
};

/// Process status effect-related messages.
pub fn handle_effect_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    lua: Arc<Mutex<KawariLua>>,
    msg: &ToServer,
) -> bool {
    match msg {
        ToServer::GainEffect(
            from_id,
            from_actor_id,
            effect_id,
            effect_param,
            effect_duration,
            effect_source_actor_id,
        ) => {
            gain_effect(
                network.clone(),
                data.clone(),
                *from_id,
                *from_actor_id,
                *effect_id,
                *effect_param,
                *effect_duration,
                *effect_source_actor_id,
                true,
            );

            true
        }
        ToServer::LoseEffect(
            from_id,
            from_actor_id,
            effect_id,
            effect_param,
            effect_source_actor_id,
        ) => {
            remove_effect(
                network.clone(),
                data.clone(),
                lua.clone(),
                *from_id,
                *from_actor_id,
                *effect_id,
                *effect_param,
                *effect_source_actor_id,
            );

            true
        }
        _ => false,
    }
}

pub fn send_effects_list(
    network: Arc<Mutex<NetworkState>>,
    instance: &Instance,
    from_actor_id: ObjectId,
) {
    let Some(actor) = instance.find_actor(from_actor_id) else {
        return;
    };

    let Some(status_effects) = actor.status_effects() else {
        return;
    };
    let common_spawn = actor.get_common_spawn();

    let mut statuses = [StatusEffect::default(); 30];
    let status_data = status_effects.data();
    statuses[..status_data.len()].copy_from_slice(status_data);

    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StatusEffectList(StatusEffectList {
        statuses,
        classjob_id: common_spawn.class_job,
        level: common_spawn.level,
        unk1: common_spawn.level,
        health_points: common_spawn.health_points,
        max_health_points: common_spawn.max_health_points,
        resource_points: common_spawn.resource_points,
        max_resource_points: common_spawn.max_resource_points,
        shield: status_effects.shield_percent(common_spawn.max_health_points) as u16,
        ..Default::default()
    }));

    let mut network = network.lock();
    network.send_in_range_inclusive_instance(
        from_actor_id,
        instance,
        FromServer::PacketSegment(ipc, from_actor_id),
        DestinationNetwork::ZoneClients,
    );
}

/// Sends an updated status effects list, as needed.
fn process_effects_list(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    from_actor_id: ObjectId,
) {
    let is_dirty;
    {
        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let Some(status_effects) = actor.status_effects() else {
            return;
        };

        is_dirty = status_effects.is_dirty();
    }

    // Only update the client if absolutely necessary (e.g. an effect is added, removed or changed duration)
    if is_dirty {
        send_effects_list(network, instance, from_actor_id);

        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        actor.status_effects_mut().unwrap().reset_dirty();
    }
}

/// Gives the actor a new effect. You can also optionally send an ACS, if needed.
pub fn gain_effect(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
    effect_id: u16,
    effect_param: u16,
    effect_duration: f32,
    effect_source_actor_id: ObjectId,
    inform_players: bool,
) -> u8 {
    let mut data = data.lock();
    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
        return 0;
    };

    gain_effect_instance(
        network,
        from_id,
        instance,
        from_actor_id,
        effect_id,
        effect_param,
        effect_duration,
        effect_source_actor_id,
        inform_players,
    )
}

/// Gives the actor a new effect. You can also optionally send an ACS, if needed.
pub fn gain_effect_instance(
    network: Arc<Mutex<NetworkState>>,
    from_id: ClientId,
    instance: &mut Instance,
    from_actor_id: ObjectId,
    effect_id: u16,
    effect_param: u16,
    effect_duration: f32,
    effect_source_actor_id: ObjectId,
    inform_players: bool,
) -> u8 {
    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
        return 0;
    };

    let Some(status_effects) = actor.status_effects_mut() else {
        return 0;
    };

    status_effects.add_with_source(
        effect_id,
        effect_param,
        effect_duration,
        effect_source_actor_id,
    );
    // The status's slot index must be its ACTUAL position in the list, not `len()` before adding:
    // if the status already existed (a refresh, or the DoT/HoT tick registered it first via
    // add_tick), `add` reuses that slot instead of appending. Reporting the wrong index here makes
    // the client show the buff twice (one from this packet's index, one from the StatusEffectList).
    let index = status_effects.position_of(effect_id).unwrap_or(0) as u8;

    if inform_players {
        {
            let mut network = network.lock();

            let ipc = ActorControlCategory::GainEffect {
                effect_id: effect_id as u32,
                param: effect_param as u32,
                source_actor_id: effect_source_actor_id,
            };

            // Then, Send an actor control to inform the client if needed
            network.send_ac_in_range_inclusive_instance(instance, from_actor_id, ipc);
        }

        // We also need to send them an updated StatusEffectsList
        process_effects_list(network.clone(), instance, from_actor_id);
    }

    // Scheduling doesn't make sense when the effect never ends.
    if effect_duration == 0.0 {
        return index;
    }

    // Cancel any existing expiry task for this same status on this actor before scheduling the new
    // one. Re-applying a status (a refresh) re-adds it with the full duration, but the OLD expiry
    // task still points at the original end time — if left in place it fires early and removes the
    // just-refreshed status, so the client shows the refresh but the buff vanishes prematurely.
    instance.retain_tasks(|task| {
        !(task.from_actor_id == from_actor_id
            && matches!(
                task.data,
                QueuedTaskData::LoseStatusEffect { effect_id: e, .. } if e == effect_id
            ))
    });

    instance.insert_task(
        from_id,
        from_actor_id,
        Duration::from_secs_f32(effect_duration),
        QueuedTaskData::LoseStatusEffect {
            effect_id,
            effect_param,
            effect_source_actor_id,
        },
    );

    index
}

/// Removes an effect from the actor.
pub fn remove_effect(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    lua: Arc<Mutex<KawariLua>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
    effect_id: u16,
    effect_param: u16,
    effect_source_actor_id: ObjectId,
) {
    // Remove it from our internal data model first
    {
        let mut data = data.lock();

        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let Some(status_effects) = actor.status_effects_mut() else {
            return;
        };

        // If we don't have the status effect, just do nothing
        if status_effects.get(effect_id).is_none() {
            return;
        }

        status_effects.remove(effect_id);
    }

    // Then send the actor control to lose the effect
    {
        let data = data.lock();
        let mut network = network.lock();

        let ipc = ActorControlCategory::LoseEffect {
            effect_id: effect_id as u32,
            unk2: effect_param as u32,
            source_actor_id: effect_source_actor_id,
        };
        network.send_ac_in_range_inclusive(&data, from_actor_id, ipc);
    }

    // Finally, inform the client of their new status effects list
    {
        let mut data = data.lock();

        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };
        process_effects_list(network.clone(), instance, from_actor_id);
    }

    // Also run the effect's Lua script in case it wants to do something!
    {
        let lua = lua.lock();
        let state = lua.0.app_data_ref::<KawariLuaState>().unwrap();

        let mut lua_player = LuaPlayer {
            player_data: PlayerData::default(),
            status_effects: StatusEffects::default(),
            queued_tasks: Vec::new(),
            zone_data: LuaZone::default(),
            content_data: LuaContent::default(),
            base_parameters: BaseParameters::default(),
            combat_state: PlayerCombatState::default(),
            level: 0,
        };

        let key = effect_id as u32;
        if let Some(effect_script) = state.effect_scripts.get(&key) {
            match std::fs::read(effect_script) {
                Ok(script_bytes) => {
                    let result = lua.0.scope(|scope| {
                        let connection_data = scope.create_userdata_ref_mut(&mut lua_player)?;

                        lua.0
                            .load(script_bytes)
                            .set_name("@".to_string() + effect_script)
                            .exec()?;

                        let func: Function = lua.0.globals().get("onLose")?;

                        func.call::<()>(connection_data)?;

                        Ok(())
                    });
                    if let Err(err) = result {
                        tracing::warn!("Error executing effect script {effect_script}: {err:?}");
                    }
                }
                Err(err) => {
                    tracing::warn!("Failed to read effect script {effect_script}: {err:?}");
                }
            }
        } else {
            tracing::warn!("Effect {effect_id} isn't scripted yet! Ignoring...");
        }

        // Inform the client of any new Lua tasks
        let mut network = network.lock();
        let msg = FromServer::NewTasks(lua_player.queued_tasks);
        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
    }
}
