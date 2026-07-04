//! Executing actions and other related functions.

use std::{sync::Arc, time::Duration};

use glam::Vec3;
use mlua::Function;
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, PlayerData, StatusEffects, TickEffectKind, ToServer,
    lua::{
        EffectsBuilder, EnmityAction, KawariLua, KawariLuaState, LuaContent, LuaPlayer, LuaZone,
        TickKind,
    },
    server::{
        WorldServer,
        actor::{NetworkedActor, NpcState},
        combat_state::PlayerCombatState,
        effect::{gain_effect, send_effects_list},
        instance::{Instance, QueuedTaskData},
        jobs::summoner,
        network::{DestinationNetwork, NetworkState},
        set_character_mode, set_shared_group_timeline_state,
    },
    zone_connection::BaseParameters,
};
use kawari::{
    common::{
        ANIMATION_LOCK_TIME, COMBO_TIMEOUT, CharacterMode, DEAD_FADE_OUT_TIME, ObjectId,
        ObjectTypeId, TimepointData,
    },
    config::FilesystemConfig,
    ipc::zone::{
        ActionEffect, ActionRequest, ActionResult, ActionType, ActorControlCategory, AoeEffect8,
        AoeEffect16, AoeEffect24, AoeEffect32, AoeEffectHeader, DamageType, EffectEntry,
        EffectKind, EffectResult, ServerZoneIpcData, ServerZoneIpcSegment,
    },
};

/// Fraction of healing done that is converted into enmity, then split across every enemy
/// engaged with the heal target. Roughly matches retail, where healing generates about half
/// its value in enmity.
const HEAL_ENMITY_MODIFIER: f32 = 0.5;

const STATUS_FEINT: u16 = 1195;
const STATUS_ADDLE: u16 = 1203;
const STATUS_SWIFTCAST: u16 = 167;

/// The cooldown group used by GCD weaponskills/spells (Action.CooldownGroup). Only this group's
/// recast is shortened by skill/spell speed; oGCD ability cooldowns are fixed.
const GCD_COOLDOWN_GROUP: u8 = 58;
const ADDITIONAL_ACTION_LOCK_100MS: u32 = 10;

/// Retail's action handler accepts a GCD/recast a hair before it technically expires, to absorb the
/// small offset between the client's locally predicted GCD and the server clock. Even with the GCD
/// started at cast time and centisecond-exact recast math, the client still *sends* the next action
/// a few milliseconds before its GCD wheel visually completes (input buffering / sub-frame timing).
/// A strict `elapsed >= duration` check rejects that request as a double-cast, which shows up as the
/// periodic "有伤害/没伤害" dropped-cast loop. A few tens of milliseconds of slack covers the
/// prediction offset without letting a genuine early double-cast (which is hundreds of ms early)
/// through. See [[gcd-cast-timing]].
const COOLDOWN_TOLERANCE: Duration = Duration::from_millis(50);

/// Mounting always uses a fixed 1-second summon cast ("Summoning..."), regardless of which mount or
/// the caster's stats. The client sends the *Mount* sheet row as the action_id, so reading a cast
/// time from the Action sheet would be meaningless, and mount casts aren't shortened by spell/skill
/// speed. Expressed in centiseconds (10ms units) to match the cast-timing pipeline.
const MOUNT_CAST_CENTISEC: u32 = 100;

/// Localhost responses can arrive before the client-side action hook finishes recording
/// `LastUsedActionSequence`. Retail always has at least network/server latency here; keep a small
/// delay so ActionEffect.SourceSequence can be matched without falling back to the 300ms task tick.
/// This is not intended to emulate RTT; it just needs to be long enough for the client to finish
/// request bookkeeping before the action-effect packet is processed.
const INSTANT_ACTION_RESPONSE_DELAY: Duration = Duration::from_millis(10);

fn is_spell_action(game_data: &mut GameData, action_id: u32) -> bool {
    game_data.get_action_category(action_id) == 2
}

fn actor_has_status(actor: &NetworkedActor, status_id: u16) -> bool {
    actor
        .status_effects()
        .is_some_and(|status_effects| status_effects.get(status_id).is_some())
}

fn remove_status_from_actor_instance(
    instance: &mut Instance,
    actor_id: ObjectId,
    status_id: u16,
) -> bool {
    let Some(actor) = instance.find_actor_mut(actor_id) else {
        return false;
    };
    let Some(status_effects) = actor.status_effects_mut() else {
        return false;
    };
    if status_effects.get(status_id).is_none() {
        return false;
    }

    status_effects.remove(status_id);
    instance.retain_tasks(|task| {
        !(task.from_actor_id == actor_id
            && matches!(
                task.data,
                QueuedTaskData::LoseStatusEffect { effect_id, .. } if effect_id == status_id
            ))
    });
    true
}

fn outgoing_damage_multiplier(has_feint: bool, has_addle: bool, damage_type: DamageType) -> f64 {
    let is_magic = damage_type == DamageType::Magic;
    let mut multiplier = 1.0;

    // Feint primarily weakens physical attacks, with a smaller magical reduction.
    if has_feint {
        multiplier *= if is_magic { 0.95 } else { 0.90 };
    }

    // Addle primarily weakens magical attacks, with a smaller physical reduction.
    if has_addle {
        multiplier *= if is_magic { 0.90 } else { 0.95 };
    }

    multiplier
}

fn send_job_gauge_update(
    network: &mut NetworkState,
    from_actor_id: ObjectId,
    classjob_id: u8,
    data: u64,
) {
    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorGauge { classjob_id, data });
    network.send_to_by_actor_id(
        from_actor_id,
        FromServer::PacketSegment(ipc, from_actor_id),
        DestinationNetwork::ZoneClients,
    );
}

/// Maximum number of targets a single `AoeEffect32` packet can carry. Targets beyond this are
/// dropped (their damage is swallowed), matching how retail caps one effect packet.
const MAX_AOE_TARGETS: usize = 32;

/// Build the smallest `AoeEffectN` IPC data that holds `targets`, packing each target's effect into
/// its own 8-slot row. `targets` is `(target, effect)` pairs and must already be capped to
/// [`MAX_AOE_TARGETS`]. Returns `None` if there are no targets.
fn build_aoe_effect_packet(
    header: AoeEffectHeader,
    targets: &[(ObjectTypeId, ActionEffect)],
    center: kawari::common::Position,
) -> Option<ServerZoneIpcData> {
    if targets.is_empty() {
        return None;
    }

    /// Fill a fixed `[[ActionEffect; 8]; N]` / `[ObjectTypeId; N]` pair from `targets`.
    macro_rules! build_variant {
        ($variant:ident, $struct:ident, $n:expr) => {{
            let mut effects = [[ActionEffect::default(); 8]; $n];
            let mut target_ids = [ObjectTypeId::default(); $n];
            for (i, (target, effect)) in targets.iter().enumerate() {
                effects[i][0] = *effect;
                target_ids[i] = *target;
            }
            ServerZoneIpcData::$variant(Box::new($struct {
                header,
                effects,
                target_ids,
                position: center,
            }))
        }};
    }

    Some(match targets.len() {
        0..=8 => build_variant!(AoeEffect8, AoeEffect8, 8),
        9..=16 => build_variant!(AoeEffect16, AoeEffect16, 16),
        17..=24 => build_variant!(AoeEffect24, AoeEffect24, 24),
        _ => build_variant!(AoeEffect32, AoeEffect32, 32),
    })
}

fn cooldown_groups_for_action(game_data: &mut GameData, action_id: u32) -> Vec<usize> {
    let mut groups = Vec::new();

    let cooldown_group = game_data.get_action_cooldown_group(action_id);
    if cooldown_group > 0 {
        groups.push((cooldown_group - 1) as usize);
    }

    let additional_cooldown_group = game_data.get_action_additional_cooldown_group(action_id);
    if additional_cooldown_group > 0 && additional_cooldown_group != cooldown_group {
        groups.push((additional_cooldown_group - 1) as usize);
    }

    groups
}

fn action_cooldown_rejections(
    game_data: &mut GameData,
    combat_state: &mut PlayerCombatState,
    action_id: u32,
) -> Vec<(usize, Duration)> {
    cooldown_groups_for_action(game_data, action_id)
        .into_iter()
        .filter_map(|group| {
            if combat_state.cooldown_ready(group, COOLDOWN_TOLERANCE) {
                None
            } else {
                Some((group, combat_state.cooldown_remaining(group)))
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct StartedCooldown {
    cooldown_group: u32,
    action_id: u32,
    duration_centisec: u32,
}

fn start_action_cooldowns(
    actor: &mut NetworkedActor,
    game_data: &mut GameData,
    action_id: u32,
) -> Vec<StartedCooldown> {
    let level = actor.get_common_spawn().level;
    let NetworkedActor::Player {
        combat_state,
        parameters,
        remove_cooldowns,
        ..
    } = actor
    else {
        return Vec::new();
    };

    // GM cheat: never put anything on cooldown so actions can be spammed.
    if *remove_cooldowns {
        return Vec::new();
    }

    let recast_100ms = u32::from(game_data.get_action_recast(action_id));
    if recast_100ms == 0 {
        return Vec::new();
    }

    let primary_group = game_data.get_action_cooldown_group(action_id);
    let additional_group = game_data.get_action_additional_cooldown_group(action_id);
    let max_charges = game_data.get_action_max_charges_at_level(action_id, level);

    // Standard GCD recast (2.5s base) used for the GCD cooldown group whenever it's the
    // *additional* cooldown of an action — most importantly demi summons, which have
    // `CooldownGroup=10 (60s)` plus `AdditionalCooldownGroup=58 (GCD)`. Non-GCD additional groups
    // (e.g. group 71 on multi-charge abilities) are only a short anti-repeat lock; the real charge
    // recovery lives on the primary group.
    const STANDARD_GCD_100MS: u32 = 25;

    // Avoid double-stamping the same group when an action lists it as both primary and
    // additional (shouldn't happen, but the data is external).
    let mut applied: Vec<u8> = Vec::with_capacity(2);
    let mut started = Vec::with_capacity(2);
    for &group_id in &[primary_group, additional_group] {
        if group_id == 0 || applied.contains(&group_id) {
            continue;
        }
        applied.push(group_id);

        // Primary group → action's own Recast100ms. Additional GCD group → the standard 2.5s GCD
        // lock. Other additional groups are short shared locks to avoid immediate double-taps.
        let base_100ms = if group_id == primary_group {
            recast_100ms
        } else if group_id == GCD_COOLDOWN_GROUP {
            STANDARD_GCD_100MS
        } else {
            ADDITIONAL_ACTION_LOCK_100MS
        };

        // Skill/spell speed shortens magic/weaponskill recasts. Abilities stay fixed.
        let base_centisec = base_100ms * 10;
        let recast_centisec = if group_id == GCD_COOLDOWN_GROUP
            || (group_id == primary_group
                && matches!(game_data.get_action_category(action_id), 2 | 3))
        {
            parameters.apply_speed(base_centisec)
        } else {
            base_centisec
        };

        combat_state.start_cooldown(
            (group_id - 1) as usize,
            action_id,
            Duration::from_millis(u64::from(recast_centisec) * 10),
            if group_id == primary_group {
                max_charges
            } else {
                1
            },
            COOLDOWN_TOLERANCE,
        );
        started.push(StartedCooldown {
            // ActorControl uses zero-based cooldown group ids on the wire.
            cooldown_group: u32::from(group_id - 1),
            action_id,
            duration_centisec: recast_centisec,
        });
    }

    started
}

fn reset_client_action_cooldowns(
    network: &mut NetworkState,
    game_data: &mut GameData,
    actor_id: ObjectId,
    action_id: u32,
) {
    for cooldown_group in cooldown_groups_for_action(game_data, action_id) {
        network.send_to_by_actor_id(
            actor_id,
            FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                cooldown_group: cooldown_group as u32,
                elapsed_centisec: 0,
                total_centisec: 0,
            }),
            DestinationNetwork::ZoneClients,
        );
    }
}

pub(super) fn clear_action_cooldowns(
    actor: &mut NetworkedActor,
    game_data: &mut GameData,
    action_id: u32,
) -> Vec<u32> {
    let NetworkedActor::Player { combat_state, .. } = actor else {
        return Vec::new();
    };

    let mut groups = Vec::new();
    for group in cooldown_groups_for_action(game_data, action_id) {
        combat_state.clear_cooldown(group);
        groups.push(group as u32);
    }

    groups
}

fn send_dirty_status_effects(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    actor_id: ObjectId,
) {
    let is_dirty = instance
        .find_actor(actor_id)
        .and_then(NetworkedActor::status_effects)
        .map(StatusEffects::is_dirty)
        .unwrap_or(false);

    if !is_dirty {
        return;
    }

    send_effects_list(network, instance, actor_id);

    if let Some(actor) = instance.find_actor_mut(actor_id)
        && let Some(status_effects) = actor.status_effects_mut()
    {
        status_effects.reset_dirty();
    }
}

fn resolve_player_action_id(
    actor: &NetworkedActor,
    actor_id: ObjectId,
    request: &ActionRequest,
    game_data: &mut GameData,
    check_cooldown: bool,
) -> Option<u32> {
    let NetworkedActor::Player {
        combat_state,
        remove_cooldowns,
        ..
    } = actor
    else {
        return Some(request.action_id);
    };

    let class_job = actor.get_common_spawn().class_job;
    let level = actor.get_common_spawn().level;
    let resolved_action_id =
        if request.action_type == ActionType::Action && summoner::is_summoner(class_job) {
            let resolved =
                summoner::resolve_summoner_action(request, combat_state, level, game_data);
            if !summoner::can_execute_summoner_action(resolved, combat_state, level) {
                tracing::warn!(
                    ?actor_id,
                    action_id = request.action_id,
                    resolved_action_id = resolved,
                    level,
                    state = ?combat_state.summoner,
                    "Rejected Summoner action because the current job state does not allow it",
                );
                return None;
            }
            resolved
        } else {
            request.action_id
        };

    // Only the immediate message handler checks the cooldown (to reject genuine double-casts). The
    // tick-driven execute path passes false, so the 500ms server-tick granularity can't spuriously
    // reject an action whose GCD was already validated and started at cast time.
    if check_cooldown && request.action_type == ActionType::Action && !*remove_cooldowns {
        let mut combat_state = combat_state.clone();
        let rejected_groups =
            action_cooldown_rejections(game_data, &mut combat_state, resolved_action_id);
        if !rejected_groups.is_empty() {
            tracing::warn!(
                ?actor_id,
                action_id = request.action_id,
                resolved_action_id,
                rejected_groups = ?rejected_groups,
                "Rejected action because one or more cooldown groups are not ready",
            );
            return None;
        }
    }

    if request.action_type == ActionType::Action {
        let mp_cost = game_data.get_action_mp_cost(resolved_action_id);
        let current_mp = actor.get_common_spawn().resource_points;
        if mp_cost > u32::from(current_mp) {
            tracing::warn!(
                ?actor_id,
                action_id = request.action_id,
                resolved_action_id,
                current_mp,
                mp_cost,
                "Rejected action because the actor does not have enough MP",
            );
            return None;
        }
    }

    Some(resolved_action_id)
}

/// Process action-related messages.
pub fn handle_action_messages(
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    network: Arc<Mutex<NetworkState>>,
    lua: Arc<Mutex<KawariLua>>,
    msg: &ToServer,
) -> bool {
    if let ToServer::ActionRequest(from_id, from_actor_id, request) = msg {
        let mut resolved_request = request.clone();

        if request.action_type == ActionType::Action {
            let resolved_action_id = {
                let data = data.lock();
                let Some(instance) = data.find_actor_instance(*from_actor_id) else {
                    return true;
                };
                let Some(actor) = instance.find_actor(*from_actor_id) else {
                    return true;
                };

                let mut game_data = game_data.lock();
                resolve_player_action_id(actor, *from_actor_id, request, &mut game_data, true)
            };

            let Some(resolved_action_id) = resolved_action_id else {
                let mut network = network.lock();
                let mut game_data = game_data.lock();
                reset_client_action_cooldowns(
                    &mut network,
                    &mut game_data,
                    *from_actor_id,
                    request.action_id,
                );
                return true;
            };
            resolved_request.action_id = resolved_action_id;
        }

        // Mounts always use a fixed 1s summon cast: the client sends the Mount *sheet* row as the
        // action_id (so get_casttime, which reads the Action sheet, would return a bogus duration),
        // and mount casts aren't affected by spell/skill speed. Everything else reads its cast time
        // from the Action sheet and is shortened by the caster's speed with the client's exact
        // (centisecond) rounding, so the cast finishes at the same instant on both sides.
        let cast_centisec = if resolved_request.action_type == ActionType::Mount {
            MOUNT_CAST_CENTISEC
        } else {
            let (cast_time_100ms, is_spell) = {
                let mut game_data = game_data.lock();
                (
                    game_data
                        .get_casttime(resolved_request.action_id)
                        .unwrap_or_default(),
                    resolved_request.action_type == ActionType::Action
                        && is_spell_action(&mut game_data, resolved_request.action_id),
                )
            };
            let base_centisec = u32::from(cast_time_100ms) * 10;
            let data = data.lock();
            data.find_actor_instance(*from_actor_id)
                .and_then(|instance| instance.find_actor(*from_actor_id))
                .and_then(|actor| match actor {
                    NetworkedActor::Player { parameters, .. } => {
                        if base_centisec > 0
                            && is_spell
                            && actor_has_status(actor, STATUS_SWIFTCAST)
                        {
                            Some(0)
                        } else {
                            Some(parameters.apply_speed(base_centisec))
                        }
                    }
                    _ => None,
                })
                .unwrap_or(base_centisec)
        };
        let delay_milliseconds = u64::from(cast_centisec) * 10;

        let mut world = data.lock();
        let Some(instance) = world.find_actor_instance_mut(*from_actor_id) else {
            return true;
        };

        if cast_centisec > 0 {
            let Some(actor) = instance.find_actor(*from_actor_id) else {
                return true;
            };

            let actor_cast = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorCast {
                spell_id: resolved_request.action_id as u16,
                action_id: resolved_request.action_id,
                action_type: resolved_request.action_type,
                omen_delay: 0,
                cast_time: delay_milliseconds as f32 / 1000.0,
                target: resolved_request.target.object_id,
                rotation: resolved_request.rotation1,
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

        // Start the server-side GCD now, at cast start. This handler runs immediately on the
        // request (not on the 500ms server tick like execute_action), so the anti-double-cast
        // check lines up with the client's locally predicted GCD instead of lagging a whole tick.
        let started_cooldowns = if let Some(actor) = instance.find_actor_mut(*from_actor_id) {
            let mut game_data = game_data.lock();
            start_action_cooldowns(actor, &mut game_data, resolved_request.action_id)
        } else {
            Vec::new()
        };
        if !started_cooldowns.is_empty() {
            let mut network = network.lock();
            for cooldown in started_cooldowns {
                network.send_to_by_actor_id(
                    *from_actor_id,
                    FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimerMax {
                        cooldown_group: cooldown.cooldown_group,
                        action_id: cooldown.action_id,
                        duration_centisec: cooldown.duration_centisec,
                    }),
                    DestinationNetwork::ZoneClients,
                );
            }
        }

        // A cast bar (delay > 0) is interruptible by movement, *except* mounting — in current
        // retail you can move freely while summoning a mount without cancelling it.
        let interruptible =
            delay_milliseconds > 0 && resolved_request.action_type != ActionType::Mount;

        if delay_milliseconds == 0 {
            let from_id = *from_id;
            let from_actor_id = *from_actor_id;
            let request = resolved_request;
            drop(world);
            tokio::task::spawn(async move {
                tokio::time::sleep(INSTANT_ACTION_RESPONSE_DELAY).await;
                execute_action(
                    network,
                    data,
                    game_data,
                    lua,
                    from_id,
                    from_actor_id,
                    request,
                );
            });
            return true;
        }

        instance.insert_task(
            *from_id,
            *from_actor_id,
            Duration::from_millis(delay_milliseconds),
            QueuedTaskData::CastAction {
                request: resolved_request,
                interruptible,
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
    if request.action_type == ActionType::Mount {
        {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                return;
            };

            let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                return;
            };

            let current_mount;
            {
                let common = actor.get_common_spawn_mut();
                common.current_mount = request.action_id as u16;
                common.mode = CharacterMode::Mounted;
                current_mount = common.current_mount;
            }

            let mut network = network.lock();
            network.send_to_by_actor_id(
                from_actor_id,
                FromServer::SetCurrentMount(current_mount),
                DestinationNetwork::ZoneClients,
            );
        }

        {
            let data = data.lock();
            let Some(instance) = data.find_actor_instance(from_actor_id) else {
                return;
            };
            let Some(actor) = instance.find_actor(from_actor_id) else {
                return;
            };

            let _ = execute_mount_action(network.clone(), from_actor_id, &request, actor, instance);
        }

        let mut data = data.lock();
        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
            return;
        };
        let mut network = network.lock();
        summoner::sync_pet_for_mount(&mut network, instance, from_actor_id);
        return;
    }

    let resolved_request = request.clone();
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

    let (mut common_spawn, _combo_action_id, in_combo, remove_cooldowns, class_job) = {
        let data = data.lock();
        let Some(instance) = data.find_actor_instance(from_actor_id) else {
            return;
        };
        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        let NetworkedActor::Player {
            teleport_query,
            parameters,
            status_effects,
            combat_state,
            remove_cooldowns,
            last_combo_action,
            ..
        } = actor
        else {
            return;
        };

        lua_player.player_data.teleport_query = teleport_query.clone();
        lua_player.base_parameters = parameters.clone();
        lua_player.status_effects = status_effects.clone();
        lua_player.combat_state = combat_state.clone();
        lua_player.level = actor.get_common_spawn().level as u16;

        let combo_action_id = {
            let mut game_data = game_data.lock();
            game_data.get_combo_action(resolved_request.action_id)
        };

        (
            actor.get_common_spawn().clone(),
            combo_action_id,
            combo_action_id == *last_combo_action,
            *remove_cooldowns,
            actor.get_common_spawn().class_job,
        )
    };

    let effects_builder = {
        let data = data.lock();
        let Some(instance) = data.find_actor_instance(from_actor_id) else {
            return;
        };
        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        match resolved_request.action_type {
            ActionType::None => None,
            ActionType::Action => {
                execute_normal_action(lua.clone(), &resolved_request, &mut lua_player, in_combo)
            }
            ActionType::Item => execute_item_action(
                game_data.clone(),
                lua.clone(),
                &resolved_request,
                &mut lua_player,
            ),
            ActionType::Mount => execute_mount_action(
                network.clone(),
                from_actor_id,
                &resolved_request,
                actor,
                instance,
            ),
            _ => unimplemented!(),
        }
    };

    if let Some(mut effects_builder) = effects_builder {
        let cleared_cooldown_groups;
        let summoner_gauge_data;
        let action_mp_cost = if resolved_request.action_type == ActionType::Action {
            let mut game_data = game_data.lock();
            game_data.get_action_mp_cost(resolved_request.action_id)
        } else {
            0
        };
        // Captured inside the data block below, used by the AoE fan-out further down.
        let aoe_base_damage: u32;
        let aoe_damage_type: DamageType;
        let aoe_radius: f32;
        let consume_swiftcast: bool;
        // Whether this action summons a generic carbuncle; the spawn is deferred until after the
        // result packet so the client plays the summon animation before the pet appears.
        let mut summon_pet_after = false;
        let has_summoner_pet_transition =
            summoner::has_pet_transition_for_action(resolved_request.action_id);

        {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                return;
            };

            if action_mp_cost > 0 {
                let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                    return;
                };
                let common = actor.get_common_spawn_mut();
                if action_mp_cost > u32::from(common.resource_points) {
                    tracing::warn!(
                        ?from_actor_id,
                        action_id = resolved_request.action_id,
                        current_mp = common.resource_points,
                        action_mp_cost,
                        "Skipped action execution because the actor no longer has enough MP",
                    );
                    return;
                }
                common.resource_points -= action_mp_cost as u16;
                common_spawn.resource_points = common.resource_points;
            }

            if let Some(actor) = instance.find_actor_mut(resolved_request.target.object_id)
                && let NetworkedActor::Npc {
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

            let combo_sequence = if let Some(actor) = instance.find_actor_mut(from_actor_id) {
                if let NetworkedActor::Player {
                    last_combo_action,
                    combo_sequence,
                    ..
                } = actor
                {
                    let sequence = *combo_sequence;
                    if in_combo {
                        *combo_sequence = combo_sequence.saturating_add(1);
                    } else {
                        *combo_sequence = 0;
                    }
                    *last_combo_action = resolved_request.action_id as u16;
                    Some(sequence)
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(sequence) = combo_sequence {
                instance.retain_tasks(|task| {
                    !(task.from_actor_id == from_actor_id
                        && matches!(task.data, QueuedTaskData::ResetCombo))
                });

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
                        action_id: resolved_request.action_id as u16,
                    },
                });
            }

            if summoner::is_summoner(class_job) {
                summoner::augment_action_result_effects(
                    resolved_request.action_id,
                    &mut effects_builder.effects,
                );
            }

            // Capture the base (pre-roll) damage and AoE radius before the loop below rolls the
            // primary target's damage in place, so we can fan the hit out to nearby enemies after.
            (aoe_base_damage, aoe_damage_type) = effects_builder
                .effects
                .iter()
                .find_map(|effect| match effect.kind {
                    EffectKind::Damage {
                        amount,
                        damage_type,
                        ..
                    } => Some((amount, damage_type)),
                    _ => None,
                })
                .unwrap_or((0, DamageType::Physical));
            aoe_radius = {
                let mut game_data = game_data.lock();
                let cast_type = game_data.get_action_cast_type(resolved_request.action_id);
                let effect_range = game_data.get_action_effect_range(resolved_request.action_id);
                // cast_type 1 = single target. Everything else with a radius is some AoE shape; we
                // approximate them all as a circle around the primary target for now.
                if cast_type != 1 && effect_range > 0 {
                    f32::from(effect_range)
                } else {
                    0.0
                }
            };
            consume_swiftcast = {
                let mut game_data = game_data.lock();
                resolved_request.action_type == ActionType::Action
                    && is_spell_action(&mut game_data, resolved_request.action_id)
                    && game_data
                        .get_casttime(resolved_request.action_id)
                        .unwrap_or_default()
                        > 0
                    && instance
                        .find_actor(from_actor_id)
                        .is_some_and(|actor| actor_has_status(actor, STATUS_SWIFTCAST))
            };

            for effect in &mut effects_builder.effects {
                match &mut effect.kind {
                    EffectKind::Damage {
                        amount,
                        damage_kind,
                        damage_type,
                        damage_element,
                        ..
                    } => {
                        // Roll crit/direct-hit/variance from the attacker's stats, and tell the
                        // client the resulting hit severity so it shows the right number style.
                        let (rolled, kind) = lua_player.base_parameters.roll_damage(*amount);
                        *amount = rolled;
                        *damage_kind = kind;

                        // Player targets (PvP/duels) mitigate by their defense; NPCs have none.
                        if let Some(NetworkedActor::Player { parameters, .. }) =
                            instance.find_actor(resolved_request.target.object_id)
                        {
                            let mitigation =
                                parameters.mitigation_against(*damage_type == DamageType::Magic);
                            *amount = ((*amount as f64) * (1.0 - mitigation)).floor() as u32;
                        }

                        if let Some(actor) =
                            instance.find_actor_mut(resolved_request.target.object_id)
                            && let Some(hate_list) = actor.npc_hate_list_mut()
                        {
                            let entry = hate_list.entry(from_actor_id).or_insert(0);
                            *entry = entry.saturating_add(*amount as u32);
                        }

                        let Some(actor) =
                            instance.find_actor_mut(resolved_request.target.object_id)
                        else {
                            return;
                        };
                        actor.apply_damage(*amount);

                        let mut game_data = game_data.lock();
                        *damage_element =
                            game_data.get_action_damage_element(resolved_request.action_id);
                    }
                    EffectKind::Heal { amount, .. } => {
                        let heal_target = resolved_request.target.object_id;

                        // Actually restore the target's HP, clamped to their maximum.
                        if let Some(actor) = instance.find_actor_mut(heal_target) {
                            let common_spawn = actor.get_common_spawn_mut();
                            common_spawn.health_points = common_spawn
                                .health_points
                                .saturating_add(*amount as u32)
                                .min(common_spawn.max_health_points);
                        }

                        // Healing generates enmity for the *healer*, split across every enemy
                        // currently engaged with the heal target. No engaged enemies means no
                        // enmity, so out-of-combat healing never pulls anything.
                        let engaged: Vec<ObjectId> = instance
                            .actors
                            .iter()
                            .filter_map(|(id, actor)| match actor {
                                NetworkedActor::Npc {
                                    hate_list,
                                    state,
                                    spawn,
                                    ..
                                } if *state != NpcState::Dead
                                    && spawn.common.health_points > 0
                                    && hate_list.contains_key(&heal_target) =>
                                {
                                    Some(*id)
                                }
                                _ => None,
                            })
                            .collect();

                        if !engaged.is_empty() {
                            let total = (*amount as f32 * HEAL_ENMITY_MODIFIER).round() as u32;
                            let each = (total / engaged.len() as u32).max(1);
                            for npc_id in engaged {
                                if let Some(actor) = instance.find_actor_mut(npc_id)
                                    && let Some(hate_list) = actor.npc_hate_list_mut()
                                {
                                    let entry = hate_list.entry(from_actor_id).or_insert(0);
                                    *entry = entry.saturating_add(each);
                                }
                            }
                        }
                    }
                    EffectKind::InterruptAction {} => {
                        instance.cancel_actor_tasks(resolved_request.target.object_id);
                    }
                    EffectKind::SummonPet { .. } => {
                        // Defer the actual pet spawn until *after* the result packet is sent, so the
                        // client receives the SummonPet effect (which plays the summon gesture/VFX)
                        // before the pet actor appears. Spawning here would pop the pet in with no
                        // animation. Egi-II summons use the same wire effect but are handled by
                        // the elemental primal transition path below, not the generic carbuncle
                        // spawn path.
                        summon_pet_after =
                            !summoner::is_elemental_primal_summon(resolved_request.action_id);
                    }
                    _ => {}
                }
            }

            // Resolve server-side enmity instructions (provoke / flat enmity / transfers) now
            // that the action's target is known.
            for enmity_action in &effects_builder.enmity_actions {
                match enmity_action {
                    EnmityAction::Add { amount } => {
                        if let Some(actor) =
                            instance.find_actor_mut(resolved_request.target.object_id)
                            && let Some(hate_list) = actor.npc_hate_list_mut()
                        {
                            let entry = hate_list.entry(from_actor_id).or_insert(0);
                            *entry = entry.saturating_add(*amount);
                        }
                    }
                    EnmityAction::Provoke => {
                        if let Some(actor) =
                            instance.find_actor_mut(resolved_request.target.object_id)
                            && let Some(hate_list) = actor.npc_hate_list_mut()
                        {
                            let highest = hate_list.values().copied().max().unwrap_or(0);
                            hate_list.insert(from_actor_id, highest.saturating_add(1));
                        }
                    }
                    EnmityAction::Transfer { percent } => {
                        // Shirk: copy a fraction of the caster's enmity onto the target on
                        // every enemy engaged with the caster. The caster keeps their enmity.
                        let transfer_target = resolved_request.target.object_id;
                        let percent = (*percent).min(100);
                        let sources: Vec<(ObjectId, u32)> = instance
                            .actors
                            .iter()
                            .filter_map(|(id, actor)| match actor {
                                NetworkedActor::Npc { hate_list, .. } => {
                                    hate_list.get(&from_actor_id).map(|hate| (*id, *hate))
                                }
                                _ => None,
                            })
                            .collect();
                        for (npc_id, source_hate) in sources {
                            let transferred = ((source_hate as u64 * percent as u64) / 100) as u32;
                            if transferred == 0 {
                                continue;
                            }
                            if let Some(actor) = instance.find_actor_mut(npc_id)
                                && let Some(hate_list) = actor.npc_hate_list_mut()
                            {
                                let entry = hate_list.entry(transfer_target).or_insert(0);
                                *entry = entry.saturating_add(transferred);
                            }
                        }
                    }
                }
            }

            // Apply the gauge changes the action requested (e.g. Necrotize spending Aetherflow),
            // before the gauge is rebuilt below so the change is reflected immediately.
            if !effects_builder.gauge_actions.is_empty()
                && let Some(NetworkedActor::Player { combat_state, .. }) =
                    instance.find_actor_mut(from_actor_id)
            {
                for gauge_action in &effects_builder.gauge_actions {
                    summoner::apply_gauge_action(combat_state, gauge_action);
                }
            }

            // Register any DoT/HoT ticks the action applied. The status itself was already added to
            // the wire effects (as a normal gain_effect); here we attach the per-tick potency so the
            // 3-second regen tick (see server_logic_tick) can resolve damage/healing each tick.
            for tick_action in &effects_builder.tick_actions {
                let tick_target = if tick_action.on_self {
                    from_actor_id
                } else {
                    resolved_request.target.object_id
                };
                let kind = match tick_action.kind {
                    TickKind::DamageMagic => TickEffectKind::DamageMagic,
                    TickKind::DamagePhysical => TickEffectKind::DamagePhysical,
                    TickKind::Heal => TickEffectKind::Heal,
                    TickKind::RestoreMp => TickEffectKind::RestoreMp,
                };
                if let Some(actor) = instance.find_actor_mut(tick_target)
                    && let Some(status_effects) = actor.status_effects_mut()
                {
                    status_effects.add_tick(
                        tick_action.effect_id,
                        tick_action.param,
                        tick_action.duration,
                        kind,
                        tick_action.potency,
                        from_actor_id,
                    );
                }

                // A damaging DoT must generate enmity the moment it's applied, exactly like a direct
                // hit — otherwise opening on an unaware enemy with a DoT (e.g. SCH Biolysis) would
                // never put the caster in its hate list, and the enemy would never aggro. Use one
                // tick's worth of damage (resolved from the caster's stats) as the initial enmity.
                // HoTs (on_self) and any non-NPC target are skipped.
                if !tick_action.on_self {
                    let initial_enmity = match tick_action.kind {
                        TickKind::DamageMagic => Some(
                            lua_player
                                .base_parameters
                                .calc_magical_damage(tick_action.potency as u32),
                        ),
                        TickKind::DamagePhysical => Some(
                            lua_player
                                .base_parameters
                                .calc_physical_damage(tick_action.potency as u32),
                        ),
                        TickKind::Heal => None,
                        TickKind::RestoreMp => None,
                    };
                    if let Some(amount) = initial_enmity
                        && let Some(actor) = instance.find_actor_mut(tick_target)
                        && let Some(hate_list) = actor.npc_hate_list_mut()
                    {
                        let entry = hate_list.entry(from_actor_id).or_insert(0);
                        *entry = entry.saturating_add(amount as u32);
                    }
                }
            }

            // Register damage barriers requested by the action. The status itself is also sent as a
            // normal gain effect, but the absorb pool lives server-side and is consumed on damage.
            for barrier_action in &effects_builder.barrier_actions {
                let barrier_target = if barrier_action.on_self {
                    from_actor_id
                } else {
                    resolved_request.target.object_id
                };
                if let Some(actor) = instance.find_actor_mut(barrier_target) {
                    let max_health_points = actor.get_common_spawn().max_health_points;
                    if let Some(status_effects) = actor.status_effects_mut() {
                        status_effects.add_barrier(
                            barrier_action.effect_id,
                            barrier_action.param,
                            barrier_action.duration,
                            barrier_action.amount,
                            from_actor_id,
                            max_health_points,
                        );
                    }
                }
            }

            summoner_gauge_data = if let Some(actor) = instance.find_actor_mut(from_actor_id)
                && summoner::is_summoner(class_job)
            {
                summoner::update_summoner_state_after_action(
                    resolved_request.action_id,
                    actor,
                    from_actor_id,
                );
                let level = actor.get_common_spawn().level;
                if let NetworkedActor::Player { combat_state, .. } = actor {
                    Some(summoner::build_summoner_gauge_data(combat_state, level))
                } else {
                    None
                }
            } else {
                None
            };

            if remove_cooldowns {
                if let Some(actor) = instance.find_actor_mut(from_actor_id) {
                    let mut game_data = game_data.lock();
                    cleared_cooldown_groups =
                        clear_action_cooldowns(actor, &mut game_data, resolved_request.action_id);
                } else {
                    cleared_cooldown_groups = Vec::new();
                }
            } else {
                // Normal cooldowns are started at cast start in handle_action_messages (which runs
                // immediately), not here on the 500ms tick, so they stay aligned with the client.
                cleared_cooldown_groups = Vec::new();
            }

            update_actor_hp_mp(network.clone(), instance, resolved_request.target.object_id);
            if from_actor_id != resolved_request.target.object_id && action_mp_cost > 0 {
                update_actor_hp_mp(network.clone(), instance, from_actor_id);
            }
            summoner::register_slipstream_lingering_aoe_after_action(
                instance,
                from_actor_id,
                resolved_request.action_id,
                resolved_request.target.object_id,
            );
            if consume_swiftcast {
                remove_status_from_actor_instance(instance, from_actor_id, STATUS_SWIFTCAST);
            }
            send_dirty_status_effects(network.clone(), instance, from_actor_id);
        }

        {
            let mut network = network.lock();

            // Only the remove-cooldowns cheat pushes explicit cooldown packets; normal GCDs are
            // predicted client-side (see start_action_cooldowns), so we don't echo them back.
            for cooldown_group in cleared_cooldown_groups {
                network.send_to_by_actor_id(
                    from_actor_id,
                    FromServer::ActorControlSelf(ActorControlCategory::SetCooldownTimer {
                        cooldown_group,
                        elapsed_centisec: 0,
                        total_centisec: 0,
                    }),
                    DestinationNetwork::ZoneClients,
                );
            }
        }

        if has_summoner_pet_transition {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                return;
            };
            let mut network = network.lock();
            summoner::prepare_pet_transition_for_action(
                &mut network,
                instance,
                from_actor_id,
                resolved_request.action_id,
            );
        }

        {
            let mut effects = [ActionEffect::default(); 8];
            effects[..effects_builder.effects.len()].copy_from_slice(&effects_builder.effects);

            let action_animation_id = {
                let mut game_data = game_data.lock();
                if resolved_request.action_type == ActionType::Item {
                    game_data
                        .lookup_item_action_data(resolved_request.action_id)
                        .map(|(action_type, _, _)| action_type)
                        .unwrap_or(resolved_request.action_id as u16)
                } else {
                    resolved_request.action_id as u16
                }
            };

            let aoe_damage_element = if aoe_radius > 0.0 && aoe_base_damage > 0 {
                let mut game_data = game_data.lock();
                Some(game_data.get_action_damage_element(resolved_request.action_id))
            } else {
                None
            };

            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                return;
            };

            // Gather every *other* enemy inside the AoE radius (if this action is an AoE at all),
            // rolling and applying each one's damage/enmity/HP now. The primary target occupies
            // slot 0; these are slots 1.. of the same effect packet.
            let mut secondary_targets: Vec<(ObjectTypeId, ActionEffect)> = Vec::new();
            if let Some(damage_element) = aoe_damage_element {
                if let Some(center) = instance
                    .find_actor(resolved_request.target.object_id)
                    .map(|actor| actor.position().0)
                {
                    let mut secondaries: Vec<ObjectId> = instance
                        .actors
                        .iter()
                        .filter_map(|(id, actor)| match actor {
                            NetworkedActor::Npc {
                                spawn,
                                state,
                                targetable,
                                ..
                            } if *id != resolved_request.target.object_id
                                && *state != NpcState::Dead
                                && *targetable
                                && !spawn.common.owner_id.is_valid()
                                && spawn.common.health_points > 0
                                && Vec3::distance(spawn.common.position.0, center)
                                    <= aoe_radius =>
                            {
                                Some(*id)
                            }
                            _ => None,
                        })
                        .collect();

                    // Reserve slot 0 for the primary; secondaries fill the rest, capped at the
                    // largest AoE packet. Anything past that is dropped (its damage swallowed),
                    // matching how retail caps a single effect packet.
                    let secondary_cap = MAX_AOE_TARGETS - 1;
                    if secondaries.len() > secondary_cap {
                        tracing::debug!(
                            "AoE {} hit {} secondaries, capping at {} (dropping {})",
                            resolved_request.action_id,
                            secondaries.len(),
                            secondary_cap,
                            secondaries.len() - secondary_cap,
                        );
                        secondaries.truncate(secondary_cap);
                    }

                    for target_id in secondaries {
                        let (rolled, kind) =
                            lua_player.base_parameters.roll_damage(aoe_base_damage);

                        if let Some(actor) = instance.find_actor_mut(target_id)
                            && let Some(hate_list) = actor.npc_hate_list_mut()
                        {
                            let entry = hate_list.entry(from_actor_id).or_insert(0);
                            *entry = entry.saturating_add(rolled as u32);
                        }

                        if let Some(actor) = instance.find_actor_mut(target_id) {
                            actor.apply_damage(rolled);
                        } else {
                            continue;
                        }

                        secondary_targets.push((
                            ObjectTypeId {
                                object_id: target_id,
                                object_type: resolved_request.target.object_type,
                            },
                            ActionEffect {
                                kind: EffectKind::Damage {
                                    amount: rolled,
                                    damage_kind: kind,
                                    damage_type: aoe_damage_type,
                                    damage_element,
                                    bonus_percent: 0,
                                    unk3: 0,
                                    unk4: 0,
                                },
                            },
                        ));
                    }
                }
            }

            if secondary_targets.is_empty() {
                // Single target (or an AoE that hit nothing else): a plain ActionResult, carrying
                // the primary's full effect set (damage, combo, gained buffs, ...).
                let mut net = network.lock();
                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                        animation_target_id: resolved_request.target,
                        target_id_again: resolved_request.target,
                        action_id: resolved_request.action_id,
                        animation_lock: ANIMATION_LOCK_TIME,
                        rotation: common_spawn.rotation,
                        spell_id: action_animation_id,
                        source_sequence: resolved_request.sequence,
                        effect_count: effects_builder.effects.len() as u8,
                        effects,
                        action_type: resolved_request.action_type,
                        global_sequence: net.global_action_sequence,
                        ..Default::default()
                    }));
                net.global_action_sequence += 1;
                net.send_in_range_inclusive_instance(
                    from_actor_id,
                    instance,
                    FromServer::PacketSegment(ipc, from_actor_id),
                    DestinationNetwork::ZoneClients,
                );
            } else {
                // Multiple targets: one AoeEffectN packet, primary at slot 0 then each secondary.
                let center = instance
                    .find_actor(resolved_request.target.object_id)
                    .map(|actor| actor.position().0)
                    .unwrap_or_default();

                let mut all_targets: Vec<(ObjectTypeId, ActionEffect)> =
                    Vec::with_capacity(secondary_targets.len() + 1);
                let primary_effect = effects_builder
                    .effects
                    .iter()
                    .copied()
                    .find(|e| matches!(e.kind, EffectKind::Damage { .. }))
                    .unwrap_or_default();
                all_targets.push((resolved_request.target, primary_effect));
                all_targets.extend(secondary_targets.iter().copied());

                let mut net = network.lock();
                let header = AoeEffectHeader {
                    animation_target_id: resolved_request.target,
                    action_id: resolved_request.action_id,
                    animation_lock: ANIMATION_LOCK_TIME,
                    rotation: common_spawn.rotation,
                    spell_id: action_animation_id,
                    source_sequence: resolved_request.sequence,
                    action_type: resolved_request.action_type,
                    target_count: all_targets.len() as u8,
                    global_sequence: net.global_action_sequence,
                    ..Default::default()
                };
                if let Some(ipc_data) =
                    build_aoe_effect_packet(header, &all_targets, kawari::common::Position(center))
                {
                    net.global_action_sequence += 1;
                    let ipc = ServerZoneIpcSegment::new(ipc_data);
                    net.send_in_range_inclusive_instance(
                        from_actor_id,
                        instance,
                        FromServer::PacketSegment(ipc, from_actor_id),
                        DestinationNetwork::ZoneClients,
                    );
                }

                // Drop the network lock before update_actor_hp_mp (which locks it internally),
                // then sync each secondary's HP bar (the primary's is synced elsewhere).
                drop(net);
                for (target, _) in &secondary_targets {
                    update_actor_hp_mp(network.clone(), instance, target.object_id);
                }
            }
        }

        if let Some(data) = summoner_gauge_data {
            let mut network = network.lock();
            send_job_gauge_update(&mut network, from_actor_id, class_job, data);
        }

        if has_summoner_pet_transition {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                return;
            };
            let mut network = network.lock();
            let _ = summoner::spawn_pet_after_action(
                &mut network,
                instance,
                from_actor_id,
                resolved_request.action_id,
                resolved_request.target.object_id,
            );
            if summoner::is_demi_summon(resolved_request.action_id) {
                summoner::schedule_demi_auto_attack(instance, from_actor_id);
            }
        }

        // Now that the result packet (carrying the SummonPet effect, which plays the summon
        // gesture/VFX) has been sent, actually spawn the pet so it appears with animation.
        if summon_pet_after {
            let mut data = data.lock();
            if let Some(instance) = data.find_actor_instance_mut(from_actor_id) {
                summoner::apply_summon_pet_effect(network.clone(), instance, from_actor_id);
            }
        }

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
                        resolved_request.target.object_id,
                        effect_id,
                        param,
                        duration,
                        from_actor_id,
                        false,
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
                        false,
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

                if let EffectKind::LoseEffect { .. } = effect.kind {
                    self_entries[num_self_entries as usize] = EffectEntry::default();
                    num_self_entries += 1;
                }
            }

            if num_self_entries > 0 {
                let mut data = data.lock();
                let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                    return;
                };
                let Some(actor) = instance.find_actor(from_actor_id) else {
                    return;
                };
                let current_common_spawn = actor.get_common_spawn().clone();
                let shield = actor.shield_percent();
                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                        unk1: 1,
                        unk2: 776386,
                        target_id: from_actor_id,
                        health_points: current_common_spawn.health_points,
                        max_health_points: current_common_spawn.max_health_points,
                        resource_points: current_common_spawn.resource_points,
                        class_id: current_common_spawn.class_job,
                        shield,
                        entry_count: num_self_entries,
                        statuses: self_entries,
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

            if num_target_entries > 0 {
                let mut data = data.lock();
                let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                    return;
                };

                let Some(actor) = instance.find_actor(resolved_request.target.object_id) else {
                    return;
                };
                let target_common_spawn = actor.get_common_spawn().clone();
                let shield = actor.shield_percent();

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                        unk1: 1,
                        unk2: 776386,
                        target_id: resolved_request.target.object_id,
                        health_points: target_common_spawn.health_points,
                        max_health_points: target_common_spawn.max_health_points,
                        resource_points: target_common_spawn.resource_points,
                        class_id: target_common_spawn.class_job,
                        shield,
                        entry_count: num_target_entries,
                        statuses: target_entries,
                        ..Default::default()
                    }));
                let mut network = network.lock();
                let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                    return;
                };
                network.send_in_range_inclusive_instance(
                    resolved_request.target.object_id,
                    instance,
                    FromServer::PacketSegment(ipc, resolved_request.target.object_id),
                    DestinationNetwork::ZoneClients,
                );
            }
        }
    }

    let mut network = network.lock();
    network.send_to(
        from_id,
        FromServer::NewTasks(lua_player.queued_tasks),
        DestinationNetwork::ZoneClients,
    );
}

/// Executes an action from an enemy.
pub fn execute_enemy_action(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    lua: Arc<Mutex<KawariLua>>,
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
        combat_state: PlayerCombatState::default(),
        level: 0,
    };

    let effects_builder;
    let common_spawn;
    let source_has_feint;
    let source_has_addle;
    {
        let Some(actor) = instance.find_actor(from_actor_id) else {
            return;
        };

        common_spawn = actor.get_common_spawn().clone();
        lua_player.level = common_spawn.level as u16;
        let source_status_effects = actor.status_effects();
        source_has_feint = source_status_effects
            .and_then(|status_effects| status_effects.get(STATUS_FEINT))
            .is_some();
        source_has_addle = source_status_effects
            .and_then(|status_effects| status_effects.get(STATUS_ADDLE))
            .is_some();

        effects_builder = match &request.action_type {
            ActionType::Action => {
                execute_normal_action(lua.clone(), &request, &mut lua_player, false)
            }
            _ => unreachable!(),
        };
    }

    if let Some(mut effects_builder) = effects_builder {
        {
            let Some(actor) = instance.find_actor_mut(request.target.object_id) else {
                return;
            };

            // Player targets mitigate enemy damage by their defense; NPCs have none.
            let (mitigation_phys, mitigation_magic) =
                if let NetworkedActor::Player { parameters, .. } = &*actor {
                    (
                        parameters.mitigation_against(false),
                        parameters.mitigation_against(true),
                    )
                } else {
                    (0.0, 0.0)
                };

            // Apply ±5% variance and the target's defense mitigation to each hit.
            for effect in &mut effects_builder.effects {
                if let EffectKind::Damage {
                    amount,
                    damage_type,
                    ..
                } = &mut effect.kind
                {
                    let mitigation = if *damage_type == DamageType::Magic {
                        mitigation_magic
                    } else {
                        mitigation_phys
                    };
                    let variance = 0.95 + fastrand::f64() * 0.10;
                    let outgoing_multiplier = outgoing_damage_multiplier(
                        source_has_feint,
                        source_has_addle,
                        *damage_type,
                    );
                    *amount =
                        ((*amount as f64) * variance * outgoing_multiplier * (1.0 - mitigation))
                            .floor() as u32;
                }
            }

            for effect in &effects_builder.effects {
                if let EffectKind::Damage { amount, .. } = effect.kind {
                    actor.apply_damage(amount as u32);
                }
            }
        }

        update_actor_hp_mp(network.clone(), instance, request.target.object_id);

        {
            let mut network = network.lock();

            let mut effects = [ActionEffect::default(); 8];
            effects[..effects_builder.effects.len()].copy_from_slice(&effects_builder.effects);

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                animation_target_id: request.target,
                target_id_again: request.target,
                action_id: request.action_id,
                animation_lock: ANIMATION_LOCK_TIME,
                rotation: common_spawn.rotation,
                spell_id: request.action_id as u16,
                source_sequence: request.sequence,
                effect_count: effects_builder.effects.len() as u8,
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
        }

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
                }

                if let EffectKind::LoseEffect { .. } = effect.kind {
                    entries[num_entries as usize] = EffectEntry::default();
                    num_entries += 1;
                }
            }

            let Some(actor) = instance.find_actor(request.target.object_id) else {
                return;
            };
            let target_common_spawn = actor.get_common_spawn().clone();
            let shield = actor.shield_percent();

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                unk1: 1,
                unk2: 776386,
                target_id: request.target.object_id,
                health_points: target_common_spawn.health_points,
                max_health_points: target_common_spawn.max_health_points,
                resource_points: target_common_spawn.resource_points,
                unk3: 0,
                class_id: target_common_spawn.class_job,
                shield,
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

pub fn cancel_action(
    network: Arc<Mutex<NetworkState>>,
    from_id: ClientId,
    log_message_id: Option<u32>,
    action_type: Option<ActionType>,
    action_id: Option<u32>,
    interrupted: Option<bool>,
) {
    let log_message_id = log_message_id.unwrap_or(0);
    let action_type = action_type.unwrap_or(ActionType::None);
    let action_id = action_id.unwrap_or(0);
    let interrupted = interrupted.unwrap_or(false);

    let msg = FromServer::ActorControlSelf(ActorControlCategory::CancelCast {
        log_message_id,
        action_type: action_type as u32,
        action_id,
        interrupted,
    });

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
        let script_bytes = match std::fs::read(action_script) {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::warn!("Failed to read action script {action_script}: {err:?}");
                return None;
            }
        };

        let result = lua.0.scope(|scope| {
            let connection_data = scope.create_userdata_ref_mut(lua_player)?;

            lua.0
                .load(script_bytes)
                .set_name("@".to_string() + action_script)
                .exec()?;

            let func: Function = lua.0.globals().get("doAction")?;

            effects_builder = Some(func.call::<EffectsBuilder>((connection_data, in_combo))?);

            Ok(())
        });
        if let Err(err) = result {
            tracing::warn!("Error executing action script {action_script}: {err:?}");
            return None;
        }
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

    let mut effects_builder = None;
    let result = lua.0.scope(|scope| {
        let connection_data = scope.create_userdata_ref_mut(lua_player)?;

        let func: Function = lua.0.globals().get("dispatchItem")?;

        match func.call::<(String, u32)>((
            &connection_data,
            key,
            action_type,
            action_data,
            additional_data,
            is_misc,
        )) {
            Ok((action_script, arg)) => {
                let path = FilesystemConfig::locate_script_file(&action_script);
                let script_bytes = match std::fs::read(&path) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        tracing::warn!(
                            "Failed to read item action script {action_script}: {err:?}"
                        );
                        return Ok(());
                    }
                };
                lua.0
                    .load(script_bytes)
                    .set_name("@".to_string() + &action_script)
                    .exec()?;

                let func: Function = lua.0.globals().get("doAction")?;

                effects_builder = Some(func.call::<EffectsBuilder>((connection_data, arg))?);
            }
            Err(err) => {
                tracing::error!("Error while calling dispatchItem: {:?}", err);
            }
        }

        Ok(())
    });
    if let Err(err) = result {
        tracing::warn!("Error executing item action {key}: {err:?}");
    }

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

// Sends the ActorControls to inform the actor that they're dead.
pub fn kill_actor(
    network: Arc<Mutex<NetworkState>>,
    instance: &mut Instance,
    from_actor_id: ObjectId,
) {
    let mut network = network.lock();

    set_character_mode(
        instance,
        &mut network,
        from_actor_id,
        CharacterMode::Dead,
        0,
    );

    network.send_ac_in_range_inclusive_instance(
        instance,
        from_actor_id,
        ActorControlCategory::Kill { animation_id: 0 },
    );

    let mut npc_id = None;
    let mut position = None;
    if let Some(actor) = instance.find_actor(from_actor_id)
        && let Some(npc) = actor.get_npc_spawn()
    {
        npc_id = Some(npc.common.layout_id);
    }

    if let Some(actor) = instance.find_actor_mut(from_actor_id)
        && let NetworkedActor::Npc {
            state,
            spawn,
            hate_list,
            ..
        } = actor
    {
        *state = NpcState::Dead;
        position = Some(spawn.common.position);
        // Clear hate so nothing lingers if this actor is ever revived/reset.
        hate_list.clear();
    }

    if let Some(npc_id) = npc_id
        && let Some(director) = &mut instance.director
    {
        director.on_actor_death(npc_id, position.unwrap());
    }

    instance.cancel_actor_tasks(from_actor_id);

    if let Some(actor) = instance.find_actor_mut(from_actor_id)
        && let NetworkedActor::Npc {
            spawn, timeline, ..
        } = actor
    {
        let mut new_timeline_states = Vec::new();

        for action in &timeline.on_death {
            match action {
                TimepointData::TimelineState { states } => {
                    let gimmick_id = spawn.gimmick_id;
                    new_timeline_states.push((gimmick_id, states.clone()));
                }
                _ => unimplemented!(),
            }
        }

        for (gimmick_id, states) in new_timeline_states {
            let actor_id = instance.find_object_by_bind_layout_id(gimmick_id);
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

    send_dirty_status_effects(network.clone(), instance, target_actor_id);

    if send_kill_actor {
        kill_actor(network, instance, target_actor_id);
    }
}
