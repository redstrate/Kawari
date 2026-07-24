//! Executing actions and other related functions.

use std::{sync::Arc, time::Duration};

use mlua::Function;
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, PlayerData, StatusEffects, ToServer,
    lua::{EffectsBuilder, KawariLua, KawariLuaState, LuaContent, LuaPlayer, LuaZone},
    server::{
        WorldServer,
        actor::{NetworkedActor, update_actor_hp_mp},
        effect::gain_effect,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
    zone_connection::{BaseParameters, TeleportQuery},
};
use kawari::{
    common::{
        ANIMATION_LOCK_TIME, COMBO_TIMEOUT, CharacterMode, ObjectId,
        STRIKING_DUMMY_NAME_ID,
    },
    config::get_config,
    ipc::zone::{
        ActionEffect, ActionRequest, ActionResult, ActionType, ActorControlCategory,
        BattleNpcSubKind, CommonSpawn, EffectEntry, EffectKind, EffectResult, ObjectKind,
        ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc,
    },
};

/// Process action-related messages.
pub fn handle_action_messages(
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    network: Arc<Mutex<NetworkState>>,
    msg: &ToServer,
) -> bool {
    if let ToServer::ActionRequest(from_id, from_actor_id, request) = msg {
        let cast_time;
        {
            let mut game_data = game_data.lock();
            cast_time = game_data.get_casttime(request.action_id).unwrap(); // TODO: take into account the haste stat like the client does
        }

        let delay_milliseconds = cast_time as u64 * 100;

        let mut data = data.lock();
        let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
            return true;
        };

        if cast_time > 0 {
            let Some(actor) = instance.find_actor(*from_actor_id) else {
                return true;
            };

            let actor_cast = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorCast {
                spell_id: request.action_id as u16,
                action_id: request.action_id,
                action_type: request.action_type,
                omen_delay: 0,
                cast_time: delay_milliseconds as f32 / 1000.0,
                target: request.target.object_id,
                rotation: request.rotation1,
                interruptible: false,
                ballista_entity_id: ObjectId::default(),
                position: actor.position(),
            });
            let mut network = network.lock();
            network.send_in_range_inclusive_instance(
                *from_actor_id,
                instance,
                FromServer::PacketSegment(actor_cast, *from_actor_id),
                DestinationNetwork::ZoneClients,
            );
        }

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
    if request.action_type == ActionType::Mount {
        let mut data = data.lock();
        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let common = actor.get_common_spawn_mut();

        common.current_mount = request.action_id as u16;
        common.mode = CharacterMode::Mounted;

        {
            let mut network = network.lock();
            let msg = FromServer::SetCurrentMount(common.current_mount);
            network.send_to_by_actor_id(from_actor_id, msg, DestinationNetwork::ZoneClients);
        }
    }

    let in_combo;
    let combo_action_id;
    {
        let mut game_data = game_data.lock();
        combo_action_id = game_data.get_combo_action(request.action_id);

        let data = data.lock();
        let Some(instance) = data.find_actor_instance(from_actor_id) else {
            return;
        };

        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        if let NetworkedActor::Player {
            last_combo_action, ..
        } = actor
        {
            in_combo = combo_action_id == *last_combo_action;
        } else {
            in_combo = false;
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
            _ => TeleportQuery::default(),
        };
        lua_player.base_parameters = match actor {
            NetworkedActor::Player { parameters, .. } => parameters.clone(),
            _ => BaseParameters::default(), // TODO: fill for other actors!
        };

        common_spawn = actor.get_common_spawn().clone();

        effects_builder = match &request.action_type {
            ActionType::None => None,
            ActionType::Action => {
                execute_normal_action(lua.clone(), &request, &mut lua_player, in_combo)
            }
            ActionType::Item => {
                execute_item_action(game_data.clone(), lua.clone(), &request, &mut lua_player)
            }
            ActionType::Mount => {
                execute_mount_action(network.clone(), from_actor_id, &request, actor, instance)
            }
            _ => unimplemented!(),
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

            // Handle combos
            {
                // TODO: don't send this for auto-attacks. it should be harmless in the mean time

                let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                    return;
                };

                let sequence;
                if let NetworkedActor::Player {
                    last_combo_action,
                    combo_sequence,
                    ..
                } = actor
                {
                    *last_combo_action = request.action_id as u16;
                    sequence = *combo_sequence;

                    if combo_action_id == *last_combo_action {
                        *combo_sequence += 1;
                    }
                } else {
                    sequence = 0;
                }

                // Ensure we cancel any pending combo resets
                // We *intentionally* cancel all combos here because they're mutually exclusive with each other.
                instance.retain_tasks(|task| {
                    task.from_actor_id == from_actor_id
                        && matches!(task.data, QueuedTaskData::ResetCombo)
                });

                // Add a new combo reset
                instance.insert_task(
                    from_id,
                    from_actor_id,
                    COMBO_TIMEOUT,
                    QueuedTaskData::ResetCombo,
                );

                effects_builder.effects.push(ActionEffect {
                    kind: EffectKind::ExecuteCombo {
                        sequence,
                        unk2: 0,
                        unk3: 0,
                        unk4: 0,
                        unk5: 128,
                        action_id: request.action_id as u16,
                    },
                });
            }

            for effect in &mut effects_builder.effects {
                match &mut effect.kind {
                    EffectKind::Damage {
                        amount,
                        damage_element,
                        ..
                    } => {
                        let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                            return;
                        };
                        let common_spawn = actor.get_common_spawn_mut();
                        if common_spawn.name_id != STRIKING_DUMMY_NAME_ID {
                            common_spawn.health_points =
                                common_spawn.health_points.saturating_sub(*amount as u32);
                        }

                        // Update from game data
                        let mut game_data = game_data.lock();
                        *damage_element = game_data.get_action_damage_element(request.action_id);
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

                        let config = get_config();
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
                            &config,
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
                    action_animation_id = if request.action_type == ActionType::Item
                        && let Some((action_type, _, _)) =
                            game_data.lookup_item_action_data(request.action_id)
                    {
                        action_type
                    } else {
                        // Otherwise, just assume the animation id is the action key for now.
                        request.action_id as u16
                    };
                }

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                        animation_target_id: request.target,
                        target_id_again: request.target,
                        action_id: request.action_id,
                        animation_lock: ANIMATION_LOCK_TIME,
                        rotation: common_spawn.rotation,
                        spell_id: action_animation_id,
                        source_sequence: request.sequence,
                        effect_count: effects_builder.effects.len() as u8,
                        effects,
                        action_type: request.action_type,
                        global_sequence: network.global_action_sequence,
                        ..Default::default()
                    }));
                network.global_action_sequence += 1;

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
            let mut num_self_entries = 0u8;
            let mut self_entries = [EffectEntry::default(); 4];

            let mut num_target_entries = 0u8;
            let mut target_entries = [EffectEntry::default(); 4];

            for effect in &effects_builder.effects {
                if let EffectKind::GainEffect {
                    effect_id,
                    duration,
                    param,
                    ..
                } = effect.kind
                {
                    let index = gain_effect(
                        network.clone(),
                        data.clone(),
                        ClientId::default(),
                        request.target.object_id,
                        effect_id,
                        param,
                        duration,
                        from_actor_id, // It's always given by this player.
                        false,         // EffectsResult will show it for us
                    );

                    target_entries[num_target_entries as usize] = EffectEntry {
                        index,
                        id: effect_id,
                        param,
                        duration,
                        source_actor_id: from_actor_id,
                        ..Default::default()
                    };
                    num_target_entries += 1;
                }

                if let EffectKind::GainEffectSelf {
                    effect_id,
                    duration,
                    param,
                    ..
                } = effect.kind
                {
                    let index = gain_effect(
                        network.clone(),
                        data.clone(),
                        from_id,
                        from_actor_id,
                        effect_id,
                        param,
                        duration,
                        from_actor_id,
                        false, // EffectsResult will show it for us
                    );

                    self_entries[num_self_entries as usize] = EffectEntry {
                        index,
                        id: effect_id,
                        param,
                        duration,
                        source_actor_id: from_actor_id,
                        ..Default::default()
                    };
                    num_self_entries += 1;
                }

                // To lose effects, we just omit them from the list but increase the entry count!
                if let EffectKind::LoseEffect { .. } = effect.kind {
                    self_entries[num_self_entries as usize] = EffectEntry::default();
                    num_self_entries += 1;

                    // TODO: need to re-review and restore this...
                    // self.status_effects.remove(effect_id);
                }
            }

            if num_self_entries > 0 {
                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                        unk1: 1,
                        unk2: 776386,
                        target_id: request.target.object_id,
                        health_points: common_spawn.health_points,
                        max_health_points: common_spawn.max_health_points,
                        resource_points: common_spawn.resource_points,
                        class_id: common_spawn.class_job,
                        entry_count: num_self_entries,
                        statuses: self_entries,
                        ..Default::default()
                    }));
                let mut data = data.lock();
                let mut network = network.lock();
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

            if num_target_entries > 0 {
                let mut data = data.lock();
                let Some(instance) = data.find_actor_instance_mut(request.target.object_id) else {
                    return;
                };

                let Some(actor) = instance.find_actor(request.target.object_id) else {
                    return;
                };

                let common_spawn = actor.get_common_spawn();

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                        unk1: 1,
                        unk2: 776386,
                        target_id: from_actor_id, // TODO: unsure if this is correct?
                        health_points: common_spawn.health_points,
                        max_health_points: common_spawn.max_health_points,
                        resource_points: common_spawn.resource_points,
                        class_id: common_spawn.class_job,
                        entry_count: num_target_entries,
                        statuses: target_entries,
                        ..Default::default()
                    }));
                let mut network = network.lock();
                let Some(instance) = data.find_actor_instance_mut(request.target.object_id) else {
                    return;
                };
                network.send_in_range_inclusive_instance(
                    request.target.object_id,
                    instance,
                    FromServer::PacketSegment(ipc, request.target.object_id),
                    DestinationNetwork::ZoneClients,
                );
            }
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
            let cooldown_group = game_data.get_action_cooldown_group(request.action_id) as u32 - 1;
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
    in_combo: bool,
) -> Option<EffectsBuilder> {
    let mut effects_builder = None;
    let lua = lua.lock();
    let state = lua.0.app_data_ref::<KawariLuaState>().unwrap();

    let key = request.action_id;
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

                effects_builder = Some(
                    func.call::<EffectsBuilder>((connection_data, in_combo))
                        .unwrap(),
                );

                Ok(())
            })
            .unwrap();
    } else {
        tracing::warn!("Action {key} isn't scripted yet!");
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

    let key = request.action_id;
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
                            std::fs::read(
                                get_config().filesystem.locate_script_file(&action_script),
                            )
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
            id: request.action_id as u16,
        },
    };

    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
        animation_target_id: request.target,
        target_id_again: request.target,
        action_id: request.action_id,
        animation_lock: ANIMATION_LOCK_TIME,
        rotation: common_spawn.rotation,
        spell_id: 4,
        source_sequence: request.sequence,
        effect_count: 1,
        effects,
        action_type: request.action_type,
        global_sequence: network.global_action_sequence,
        ..Default::default()
    }));
    network.global_action_sequence += 1;

    network.send_in_range_inclusive_instance(
        from_actor_id,
        instance,
        FromServer::PacketSegment(ipc, from_actor_id),
        DestinationNetwork::ZoneClients,
    );

    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Mount {
        id: request.action_id as u16,
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

