//! Executing status effect related functions.

use std::{sync::Arc, time::Duration};

use mlua::{Function, Lua};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, PlayerData, StatusEffects, ToServer,
    lua::{ExtraLuaState, LuaPlayer, LuaZone},
    server::{
        WorldServer,
        actor::NetworkedActor,
        instance::QueuedTaskData,
        network::{DestinationNetwork, NetworkState},
    },
};
use kawari::{
    common::ObjectId,
    config::get_config,
    ipc::zone::{
        ActorControlCategory, ServerZoneIpcData, ServerZoneIpcSegment, StatusEffect,
        StatusEffectList,
    },
};

/// Process status effect-related messages.
pub fn handle_effect_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    lua: Arc<Mutex<Lua>>,
    msg: &ToServer,
) {
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
        }
        _ => {}
    }
}

/// Sends an updated status effects list, as needed.
fn process_effects_list(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
) {
    let mut data = data.lock();

    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
        return;
    };

    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
        return;
    };

    let NetworkedActor::Player {
        status_effects,
        spawn,
        ..
    } = actor
    else {
        return;
    };

    // Only update the client if absolutely necessary (e.g. an effect is added, removed or changed duration)
    let ipc;
    if status_effects.is_dirty() {
        let mut list = [StatusEffect::default(); 30];
        let status_data = status_effects.data();
        list[..status_data.len()].copy_from_slice(status_data);

        ipc = Some(ServerZoneIpcSegment::new(
            ServerZoneIpcData::StatusEffectList(StatusEffectList {
                statues: list,
                classjob_id: spawn.common.class_job,
                level: spawn.common.level,
                curr_hp: spawn.common.hp_curr,
                max_hp: spawn.common.hp_max,
                curr_mp: spawn.common.mp_curr,
                max_mp: spawn.common.mp_max,
                ..Default::default()
            }),
        ));

        // Inform the player
        let mut network = network.lock();
        network.send_ipc_to(from_id, ipc.clone().unwrap(), from_actor_id);

        status_effects.reset_dirty();
    } else {
        ipc = None;
    }

    // Inform other players in range
    if let Some(ipc) = ipc {
        let mut network = network.lock();
        network.send_in_range(
            from_actor_id,
            &data,
            FromServer::PacketSegment(ipc, from_actor_id),
            DestinationNetwork::ZoneClients,
        );
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
    send_acs: bool,
) {
    {
        // First, add it to the actor's effect's list
        let mut data = data.lock();

        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let NetworkedActor::Player { status_effects, .. } = actor else {
            return;
        };

        status_effects.add(effect_id, effect_param, effect_duration);

        let mut network = network.lock();

        let ipc = ActorControlCategory::GainEffect {
            effect_id: effect_id as u32,
            param: effect_param as u32,
            source_actor_id: effect_source_actor_id,
        };

        // Then, Send an actor control to inform the client if needed
        if send_acs {
            network.send_ac_in_range_inclusive(&data, from_id, from_actor_id, ipc);
        } else {
            network.send_ac_in_range(&data, from_actor_id, ipc);
        }
    }

    // We also need to send them an updated StatusEffectsList
    process_effects_list(network.clone(), data.clone(), from_id, from_actor_id);

    // Scheduling doesn't make sense when the effect never ends.
    if effect_duration == 0.0 {
        return;
    }

    // Eventually tell the player they lost this effect
    tracing::info!("Effect {effect_id} lasts for {effect_duration} seconds");

    let mut data = data.lock();
    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
        return;
    };

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
}

/// Removes an effect from the actor.
pub fn remove_effect(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    lua: Arc<Mutex<Lua>>,
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

        let NetworkedActor::Player { status_effects, .. } = actor else {
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
        let mut network = network.lock();
        let data = data.lock();

        let ipc = ActorControlCategory::LoseEffect {
            effect_id: effect_id as u32,
            unk2: effect_param as u32,
            source_actor_id: effect_source_actor_id,
        };
        network.send_ac_in_range_inclusive(&data, from_id, from_actor_id, ipc);
    }

    // Finally, inform the client of their new status effects list
    process_effects_list(network.clone(), data.clone(), from_id, from_actor_id);

    // Also run the effect's Lua script in case it wants to do something!
    {
        let lua = lua.lock();
        let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

        let mut lua_player = LuaPlayer {
            player_data: PlayerData::default(),
            status_effects: StatusEffects::default(),
            queued_tasks: Vec::new(),
            zone_data: LuaZone::default(),
        };

        let key = effect_id as u32;
        if let Some(effect_script) = state.effect_scripts.get(&key) {
            lua.scope(|scope| {
                let connection_data = scope.create_userdata_ref_mut(&mut lua_player).unwrap();

                let config = get_config();

                let file_name = format!("{}/{}", &config.world.scripts_location, effect_script);
                lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
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

        // Inform the client of any new Lua tasks
        let mut network = network.lock();
        let msg = FromServer::NewTasks(lua_player.queued_tasks);
        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
    }
}
