//! Executing actions and other related functions.

use std::{sync::Arc, time::Duration};

use mlua::Function;
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, PlayerData, StatusEffects, ToServer,
    lua::{EffectsBuilder, KawariLua, KawariLuaState, LuaContent, LuaPlayer, LuaZone},
    server::{
        WorldServer,
        actor::{NetworkedActor, NpcState},
        effect::gain_effect,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
        set_character_mode, set_shared_group_timeline_state,
    },
    zone_connection::BaseParameters,
};
use kawari::{
    common::{CharacterMode, DEAD_FADE_OUT_TIME, ObjectId, STRIKING_DUMMY_NAME_ID, TimepointData},
    config::FilesystemConfig,
    ipc::zone::{
        ActionEffect, ActionKind, ActionRequest, ActionResult, ActorControlCategory,
        BattleNpcSubKind, CommonSpawn, EffectEntry, EffectKind, EffectResult, ObjectKind,
        ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc,
    },
};

/// Process action-related messages.
pub fn handle_action_messages(
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    msg: &ToServer,
) -> bool {
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
            return true;
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

        return true;
    }

    false
}

/// Executes an action, and returns a list of Tasks that must be executed by the client.
pub fn execute_action(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    lua: Arc<Mutex<KawariLua>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
    request: ActionRequest,
) {
    let mut lua_player = LuaPlayer {
        player_data: PlayerData::default(),
        status_effects: StatusEffects::default(),
        queued_tasks: Vec::new(),
        zone_data: LuaZone::default(),
        content_data: LuaContent::default(),
        base_parameters: BaseParameters::default(),
    };
    // TODO: Isn't there a better way to do this without a bunch of borrow checking issues involving data, actor, and instance below?
    // Regardless, we need to set the player's mount id in their common spawn so both pillion works and also letting players see this existing actor's mount when they spawn.
    if request.action_kind == ActionKind::Mount {
        let mut data = data.lock();
        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let common = actor.get_common_spawn_mut();

        common.current_mount = request.action_key as u16;
        common.mode = CharacterMode::Mounted;

        {
            let mut network = network.lock();
            let msg = FromServer::SetCurrentMount(common.current_mount);
            network.send_to_by_actor_id(from_actor_id, msg, DestinationNetwork::ZoneClients);
        }
    }

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
        lua_player.base_parameters = match actor {
            NetworkedActor::Player { parameters, .. } => parameters.clone(),
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
                execute_mount_action(network.clone(), from_actor_id, &request, actor, instance)
            }
        };
    }

    // tell them the action results
    if let Some(mut effects_builder) = effects_builder {
        // Update our internal data model to their new HP
        {
            let mut data = data.lock();

            let Some(instance) = data.find_actor_instance_mut(request.target.object_id) else {
                return;
            };

            // aggro any NPCs
            {
                let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                    return;
                };

                if let NetworkedActor::Npc {
                    newly_hated_actor, ..
                } = actor
                {
                    *newly_hated_actor = Some(from_actor_id);
                }
            }

            // Handle invulnerability
            {
                let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                    return;
                };

                if let NetworkedActor::Npc {
                    currently_invulnerable,
                    ..
                } = actor
                    && *currently_invulnerable
                {
                    effects_builder.effects = effects_builder
                        .effects
                        .iter()
                        .map(|effect| match effect.kind {
                            EffectKind::Damage { .. } => ActionEffect {
                                kind: EffectKind::Invincible {},
                            },
                            _ => *effect,
                        })
                        .collect();
                }
            }

            for effect in &effects_builder.effects {
                match &effect.kind {
                    EffectKind::Damage { amount, .. } => {
                        let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                            return;
                        };
                        let common_spawn = actor.get_common_spawn_mut();
                        if common_spawn.name_id != STRIKING_DUMMY_NAME_ID {
                            common_spawn.health_points =
                                common_spawn.health_points.saturating_sub(*amount as u32);
                        }
                    }
                    EffectKind::InterruptAction {} => {
                        // TODO: this could cancel more than just casting, so we need to be more specific eventually
                        // TODO: also cancel the cast visually
                        instance.cancel_actor_tasks(request.target.object_id);
                    }
                    EffectKind::SummonPet { .. } => {
                        let Some(actor) = instance.find_actor(from_actor_id) else {
                            return;
                        };

                        let pet_id = 23; // TODO: hardcoded

                        let mut network = network.lock();
                        network.send_to_by_actor_id(
                            from_actor_id,
                            FromServer::ActorControlSelf(ActorControlCategory::SetPetParameters {
                                pet_id,
                                unk2: 2,
                                unk3: 5,
                                unk4: 7,
                            }),
                            DestinationNetwork::ZoneClients,
                        );

                        let pet_actor_id = ObjectId(fastrand::u32(..));

                        instance.insert_npc(
                            pet_actor_id,
                            SpawnNpc {
                                common: CommonSpawn {
                                    base_id: 13498, // TODO: hardcoded
                                    name_id: 10261,
                                    pet_id,
                                    owner_id: from_actor_id,
                                    max_health_points: 100, // TODO
                                    health_points: 100,
                                    model_chara: 411, // TODO: hardcoded
                                    object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Pet),
                                    level: actor.get_common_spawn().level,
                                    position: actor.position(),
                                    rotation: actor.rotation(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        );

                        network.send_to_by_actor_id(
                            from_actor_id,
                            FromServer::ActorControlSelf(ActorControlCategory::SetupPet {
                                owner_id: from_actor_id,
                                pet_id,
                                pet_actor_id,
                                unk2: 1,
                                unk3: 1,
                            }),
                            DestinationNetwork::ZoneClients,
                        );
                    }
                    _ => {}
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

                let action_animation_id;
                {
                    let mut game_data = game_data.lock();

                    // TODO: Not sure if this is correct in every item situation
                    // If the action is an item being used, the animation id doesn't necessarily match the action id.
                    action_animation_id = if request.action_kind == ActionKind::Item
                        && let Some((action_type, _, _)) =
                            game_data.lookup_item_action_data(request.action_key)
                    {
                        action_type
                    } else {
                        // Otherwise, just assume the animation id is the action key for now.
                        request.action_key as u16
                    };
                }

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                        main_target: request.target,
                        target_id_again: request.target,
                        action_id: request.action_key,
                        animation_lock_time: 0.6,
                        rotation: common_spawn.rotation,
                        action_animation_id,
                        flag: 1,
                        effect_count: effects_builder.effects.len() as u8,
                        effects,
                        unk1: 2662353,
                        unk2: 3758096384,
                        hidden_animation: 1,
                        ..Default::default()
                    }));

                let mut data = data.lock();

                let Some(instance) = data.find_actor_instance_mut(request.target.object_id) else {
                    return;
                };
                network.send_in_range_inclusive_instance(
                    from_actor_id,
                    instance,
                    FromServer::PacketSegment(ipc, from_actor_id),
                    DestinationNetwork::ZoneClients,
                );
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
                        source_actor_id: Default::default(),
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
                health_points: common_spawn.health_points,
                max_health_points: common_spawn.max_health_points,
                resource_points: common_spawn.resource_points,
                unk3: 0,
                class_id: common_spawn.class_job,
                shield: 0,
                entry_count: num_entries,
                unk4: 0,
                statuses: entries,
            }));
            let mut network = network.lock();
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(request.target.object_id) else {
                return;
            };
            network.send_in_range_inclusive_instance(
                from_actor_id,
                instance,
                FromServer::PacketSegment(ipc, from_actor_id),
                DestinationNetwork::ZoneClients,
            );
        }
    }

    let mut network = network.lock();
    let msg = FromServer::NewTasks(lua_player.queued_tasks);
    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);

    // Update cooldowns, currently only for the GM command
    {
        let data = data.lock();

        let Some(instance) = data.find_actor_instance(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        let NetworkedActor::Player {
            remove_cooldowns, ..
        } = actor
        else {
            return;
        };

        if *remove_cooldowns {
            let mut game_data = game_data.lock();
            let cooldown_group = game_data.get_action_cooldown_group(request.action_key) as u32 - 1;
            network.send_to_by_actor_id(
                from_actor_id,
                FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                    cooldown_group,
                    unk1: 0,
                    unk2: 0,
                }),
                DestinationNetwork::ZoneClients,
            );
        }
    }
}

/// Executes an action from an enemy.
pub fn execute_enemy_action(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    lua: Arc<Mutex<KawariLua>>,
    from_actor_id: ObjectId,
    request: ActionRequest,
) {
    // TODO: de-duplicate with the function above

    let mut lua_player = LuaPlayer {
        player_data: PlayerData::default(),
        status_effects: StatusEffects::default(),
        queued_tasks: Vec::new(),
        zone_data: LuaZone::default(),
        content_data: LuaContent::default(),
        base_parameters: BaseParameters::default(),
    };

    let effects_builder;
    let common_spawn;
    {
        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        common_spawn = actor.get_common_spawn().clone();

        effects_builder = match &request.action_kind {
            ActionKind::Normal => execute_normal_action(lua.clone(), &request, &mut lua_player),
            _ => unreachable!(),
        };
    }

    // tell them the action results
    if let Some(effects_builder) = effects_builder {
        // Update our internal data model to their new HP
        {
            let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                return;
            };

            let common_spawn = actor.get_common_spawn_mut();

            for effect in &effects_builder.effects {
                if let EffectKind::Damage { amount, .. } = effect.kind {
                    common_spawn.health_points =
                        common_spawn.health_points.saturating_sub(amount as u32);
                }
            }
        }

        update_actor_hp_mp(network.clone(), instance, request.target.object_id);

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

                network.send_in_range_inclusive_instance(
                    from_actor_id,
                    instance,
                    FromServer::PacketSegment(ipc, from_actor_id),
                    DestinationNetwork::ZoneClients,
                );
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
                        source_actor_id: Default::default(),
                    };
                    num_entries += 1;

                    // TODO: does this make sense for enemies...?
                    // gain_effect_instance(
                    //     network.clone(),
                    //             from_id,
                    //             instance,
                    //             from_actor_id,
                    //             effect_id,
                    //             param,
                    //             duration,
                    //             source_actor_id,
                    //             false,
                    // ); // ACS isn't needed as EffectsResult will show it for us
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
                health_points: common_spawn.health_points,
                max_health_points: common_spawn.max_health_points,
                resource_points: common_spawn.resource_points,
                unk3: 0,
                class_id: common_spawn.class_job,
                shield: 0,
                entry_count: num_entries,
                unk4: 0,
                statuses: entries,
            }));
            let mut network = network.lock();
            network.send_in_range_inclusive_instance(
                from_actor_id,
                instance,
                FromServer::PacketSegment(ipc, from_actor_id),
                DestinationNetwork::ZoneClients,
            );
        }
    }
}

pub fn cancel_action(network: Arc<Mutex<NetworkState>>, from_id: ClientId) {
    let msg = FromServer::ActorControlSelf(ActorControlCategory::CancelCast {});

    let mut network = network.lock();
    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
}

/// Handles normal actions, powered by Lua.
pub fn execute_normal_action(
    lua: Arc<Mutex<KawariLua>>,
    request: &ActionRequest,
    lua_player: &mut LuaPlayer,
) -> Option<EffectsBuilder> {
    let mut effects_builder = None;
    let lua = lua.lock();
    let state = lua.0.app_data_ref::<KawariLuaState>().unwrap();

    let key = request.action_key;
    if let Some(action_script) = state.action_scripts.get(&key) {
        lua.0
            .scope(|scope| {
                let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

                lua.0
                    .load(
                        std::fs::read(action_script).expect("Failed to locate scripts directory!"),
                    )
                    .set_name("@".to_string() + action_script)
                    .exec()
                    .unwrap();

                let func: Function = lua.0.globals().get("doAction").unwrap();

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
    lua: Arc<Mutex<KawariLua>>,
    request: &ActionRequest,
    lua_player: &mut LuaPlayer,
) -> Option<EffectsBuilder> {
    let lua = lua.lock();

    let key = request.action_key;
    let (action_type, action_data, additional_data);
    let is_misc;
    {
        let mut gamedata = game_data.lock();
        (action_type, action_data, additional_data) =
            gamedata.lookup_item_action_data(key).unwrap_or_default();
        is_misc = gamedata.item_is_misc(key);
    }

    // FIXME: we should check if this data is valid instead of silently returning zeroes

    let mut effects_builder = None;
    lua.0
        .scope(|scope| {
            let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

            let func: Function = lua.0.globals().get("dispatchItem").unwrap();

            match func.call::<(String, u32)>((
                &connection_data,
                key,
                action_type,
                action_data,
                additional_data,
                is_misc,
            )) {
                Ok((action_script, arg)) => {
                    lua.0
                        .load(
                            std::fs::read(FilesystemConfig::locate_script_file(&action_script))
                                .expect("Failed to locate scripts directory!"),
                        )
                        .set_name("@".to_string() + &action_script)
                        .exec()
                        .unwrap();

                    let func: Function = lua.0.globals().get("doAction").unwrap();

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
    from_actor_id: ObjectId,
    request: &ActionRequest,
    actor: &NetworkedActor,
    instance: &Instance,
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
    network.send_in_range_inclusive_instance(
        from_actor_id,
        instance,
        FromServer::PacketSegment(ipc, from_actor_id),
        DestinationNetwork::ZoneClients,
    );

    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Mount {
        id: request.action_key as u16,
        unk1: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    });
    network.send_in_range_inclusive_instance(
        from_actor_id,
        instance,
        FromServer::PacketSegment(ipc, from_actor_id),
        DestinationNetwork::ZoneClients,
    );

    None
}

// Sends the ActorControls to inform the actor that they're dead.
pub fn kill_actor(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    from_actor_id: ObjectId,
) {
    // TODO: set HP/MP to zero here

    let mut network = network.lock();

    // First, set their state (otherwise they can still walk)
    set_character_mode(
        instance,
        &mut network,
        from_actor_id,
        CharacterMode::Dead,
        0,
    );

    // Then, play the death animation.
    {
        let ac = ActorControlCategory::Kill { animation_id: 0 };

        network.send_ac_in_range_inclusive_instance(instance, from_actor_id, ac);
    }

    // Inform the director that their actor died
    let mut npc_id = None;
    let mut position = None;
    if let Some(actor) = instance.find_actor(from_actor_id)
        && let Some(npc) = actor.get_npc_spawn()
    {
        npc_id = Some(npc.common.layout_id);
    }

    // Transistion into the dead state so they stop moving.
    if let Some(actor) = instance.find_actor_mut(from_actor_id)
        && let NetworkedActor::Npc { state, spawn, .. } = actor
    {
        *state = NpcState::Dead;
        position = Some(spawn.common.position);
    }

    if let Some(npc_id) = npc_id
        && let Some(director) = &mut instance.director
    {
        director.on_actor_death(npc_id, position.unwrap());
    }

    // Cancel existing tasks
    instance.cancel_actor_tasks(from_actor_id);

    // Queue up despawn if this is an NPC
    if let Some(actor) = instance.find_actor_mut(from_actor_id)
        && let NetworkedActor::Npc {
            spawn, timeline, ..
        } = actor
    {
        let mut new_timeline_states = Vec::new();

        // Play any timeline actions on death.
        // TODO: please de-duplicate with the other handler if possible!
        for action in &timeline.on_death {
            match action {
                TimepointData::TimelineState { states } => {
                    // Find the event object bound to our gimmick.
                    let gimmick_id = spawn.gimmick_id;
                    new_timeline_states.push((gimmick_id, states.clone()));
                }
                _ => unimplemented!(),
            }
        }

        for (gimmick_id, states) in new_timeline_states {
            let actor_id;
            {
                actor_id = instance.find_object_by_bind_layout_id(gimmick_id);
            }
            if let Some(actor_id) = actor_id {
                set_shared_group_timeline_state(instance, &mut network, actor_id, &states);
            }
        }

        instance.insert_task(
            ClientId::default(),
            from_actor_id,
            DEAD_FADE_OUT_TIME,
            QueuedTaskData::DeadFadeOut {
                actor_id: from_actor_id,
            },
        );
    }
}

/// Updates other actors about this actor's HP and MP.
pub fn update_actor_hp_mp(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    target_actor_id: ObjectId,
) {
    let mut send_kill_actor = false;
    // Inform the client of the new actor's HP/MP
    {
        let Some(actor) = instance.find_actor(target_actor_id) else {
            return;
        };

        let common_spawn = actor.get_common_spawn();

        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp {
                hp: common_spawn.health_points,
                mp: common_spawn.resource_points,
                unk: 0,
            });
            let mut network = network.lock();
            network.send_in_range_inclusive_instance(
                target_actor_id,
                instance,
                FromServer::PacketSegment(ipc, target_actor_id),
                DestinationNetwork::ZoneClients,
            );
        }

        if common_spawn.health_points == 0 && common_spawn.mode != CharacterMode::Dead {
            send_kill_actor = true;
        }
    }

    if send_kill_actor {
        kill_actor(network.clone(), instance, target_actor_id);
    }
}
