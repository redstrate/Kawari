use glam::{Affine3A, EulerRot, Vec3};
use parking_lot::Mutex;
use physis::TerritoryIntendedUse;
use std::{
    collections::HashMap,
    env::consts::EXE_SUFFIX,
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::Receiver;

use crate::{
    GameData, Navmesh, SpawnAllocator,
    lua::KawariLua,
    server::{
        action::{
            execute_action, execute_enemy_action, handle_action_messages, kill_actor,
            update_actor_hp_mp,
        },
        actor::{NetworkedActor, NpcState},
        chat::handle_chat_messages,
        director::{
            DirectorData, PendingAoe, director_tick, encounter_tick, handle_director_messages,
            resolve_aoe,
        },
        effect::{handle_effect_messages, remove_effect, send_effects_list},
        instance::{Instance, NavmeshGenerationStep, QueuedTaskData},
        jobs::summoner,
        linkshell::handle_linkshell_messages,
        network::{DestinationNetwork, NetworkState},
        party::{
            NUM_TARGET_SIGNS, get_party_id_from_actor_id, handle_party_messages,
            send_party_positions, update_party_position, update_party_waymark,
            update_party_waymarks,
        },
        social::handle_social_messages,
        zone::{
            MapGimmick, change_zone_to_player, change_zone_warp_to_entrance,
            change_zone_warp_to_pop_range, handle_zone_messages, restore_carried_combat_state,
            take_combat_state_and_despawn_pets,
        },
    },
};
use kawari::{
    common::{
        CharacterMode, DEAD_DESPAWN_TIME, EventState, HandlerId, HandlerType, MAX_SPAWNED_ACTORS,
        MAX_SPAWNED_OBJECTS, ObjectId, ObjectTypeId, ObjectTypeKind, Position, SLIDECAST_WINDOW,
        SharedGroupTimelineState, determine_initial_pop_range, euler_to_direction, is_private_area,
    },
    config::{FilesystemConfig, get_config},
    ipc::zone::{
        ActorControlCategory, ClientTriggerCommand, Condition, Conditions, EnmityList, Hater,
        HaterList, PlayerEnmity, ServerZoneIpcData, ServerZoneIpcSegment, WarpType, WaymarkPreset,
    },
};

use super::{ClientId, FromServer, ToServer};
use crate::common::PetCommand;

mod action;
mod actor;
mod chat;
pub(crate) mod combat_state;
mod director;
mod effect;
mod instance;
mod jobs;
mod linkshell;
mod network;
mod party;
pub use party::{Party, PartyMember};
mod npc_behavior;
mod social;
mod zone;

#[derive(Default, Debug, Clone)]
struct ClientState {
    actor_allocator: SpawnAllocator<MAX_SPAWNED_ACTORS, 1>, // Indices start at 1 because the player always takes the 0 index.
    object_allocator: SpawnAllocator<MAX_SPAWNED_OBJECTS>,
}

impl ClientState {
    /// Check if this client has spawned said `object_id`.
    pub fn has_spawned(&self, object_id: ObjectId) -> bool {
        self.actor_allocator.contains(object_id) || self.object_allocator.contains(object_id)
    }
}

#[derive(Default, Debug)]
struct WorldServer {
    instances: Vec<Instance>,
    // TODO: Eventually remove these once we can reliably and ergonomically run misc. tasks on slower intervals!
    rested_exp_counter: i32,
    party_positions_counter: i32,
    /// Drives the 3-second DoT/HoT + natural HP/MP regen tick (300ms tick * 10 = 3s), matching retail.
    regen_tick_counter: i32,
}

fn allocate_hate_slots(current: &HashMap<ObjectId, u8>) -> Vec<u8> {
    let mut used: Vec<u8> = current.values().copied().collect();
    used.sort_unstable();

    (0..32)
        .filter(|slot| !used.contains(slot))
        .map(|slot| slot as u8)
        .collect()
}

fn sync_player_hated_by(
    player_actor: &mut NetworkedActor,
    hated_npcs: &[(ObjectId, u32)],
) -> (Vec<(ObjectId, u8)>, Vec<ObjectId>) {
    let Some(hated_by) = player_actor.player_hated_by_mut() else {
        return (Vec::new(), Vec::new());
    };

    let active_ids: Vec<ObjectId> = hated_npcs.iter().map(|(id, _)| *id).collect();
    let removed: Vec<ObjectId> = hated_by
        .keys()
        .copied()
        .filter(|id| !active_ids.contains(id))
        .collect();

    for actor_id in &removed {
        hated_by.remove(actor_id);
    }

    let mut free_slots = allocate_hate_slots(hated_by);
    let mut assigned = Vec::new();
    for (actor_id, _) in hated_npcs {
        let slot = if let Some(slot) = hated_by.get(actor_id) {
            *slot
        } else if let Some(slot) = free_slots.first().copied() {
            free_slots.remove(0);
            hated_by.insert(*actor_id, slot);
            slot
        } else {
            let slot = hated_by.len().min(255) as u8;
            hated_by.insert(*actor_id, slot);
            slot
        };

        assigned.push((*actor_id, slot));
    }

    (assigned, removed)
}

fn refresh_runtime_job_state(instance: &mut Instance, network: &mut NetworkState) -> Vec<ObjectId> {
    let actor_ids: Vec<ObjectId> = instance.actors.keys().copied().collect();
    let mut refreshed = Vec::new();

    for actor_id in actor_ids {
        let Some(actor) = instance.find_actor_mut(actor_id) else {
            continue;
        };

        let Some(common) = actor.common_spawn() else {
            continue;
        };

        let class_job = common.class_job;
        if !summoner::is_summoner(class_job) {
            continue;
        }

        let result = summoner::refresh_summoner_runtime_state_on_actor(actor_id, actor);
        let status_timer_refreshed = result.status_timer_refreshed;
        let gauge_data = if result.changed {
            let level = actor
                .common_spawn()
                .expect("summoner runtime actor has common spawn")
                .level;
            if let NetworkedActor::Player { combat_state, .. } = actor {
                Some(summoner::build_summoner_gauge_data(combat_state, level))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(gauge_data) = gauge_data
            && !result.demi_just_ended
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorGauge {
                classjob_id: class_job,
                data: gauge_data,
            });
            network.send_to_by_actor_id(
                actor_id,
                FromServer::PacketSegment(ipc, actor_id),
                DestinationNetwork::ZoneClients,
            );
        }

        // Demi window just expired this tick — retail kills/despawns the demi actor, clears the
        // demi hotbar, then re-spawns/re-binds carbuncle instead of only flipping UI state.
        if result.demi_just_ended {
            summoner::apply_demi_summon_revert(
                network,
                instance,
                actor_id,
                gauge_data.map(|data| (class_job, data)),
            );
        }

        if status_timer_refreshed {
            refreshed.push(actor_id);
        }
    }

    refreshed
}

impl WorldServer {
    /// Ensures an instance exists, and creates one if not found.
    fn ensure_exists(&mut self, zone_id: u16, game_data: &mut GameData) -> &mut Instance {
        let is_public_instance;
        if let Some(intended_use) = game_data.get_intended_use(zone_id as u32) {
            is_public_instance = !is_private_area(intended_use);
        } else {
            is_public_instance = true; // Fall back to assuming it's public I guess
        }

        if is_public_instance {
            // create a new instance if necessary
            if !self
                .instances
                .iter()
                .any(|x| x.zone.id == zone_id && x.content_finder_condition_id == 0)
            {
                tracing::info!("Creating new public instance for {zone_id}!");
                self.instances.push(Instance::new(zone_id, game_data));
            }

            self.instances
                .iter_mut()
                .find(|x| x.zone.id == zone_id && x.content_finder_condition_id == 0)
                .unwrap()
        } else {
            tracing::info!("Creating new private instance for {zone_id}!");
            self.instances.push(Instance::new(zone_id, game_data));
            self.instances.last_mut().unwrap()
        }
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance(&self, actor_id: ObjectId) -> Option<&Instance> {
        self.instances
            .iter()
            .find(|instance| instance.actors.contains_key(&actor_id))
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance_mut(&mut self, actor_id: ObjectId) -> Option<&mut Instance> {
        self.instances
            .iter_mut()
            .find(|instance| instance.actors.contains_key(&actor_id))
    }

    fn create_instance_for_content(
        &mut self,
        zone_id: u16,
        content_finder_condition: u16,
        game_data: &mut GameData,
    ) -> Option<&mut Instance> {
        let mut instance = Instance::new(zone_id, game_data);
        instance.content_finder_condition_id = content_finder_condition;

        // TODO: This duplicates a lot of code with ZoneConnection::handle_zone_change :-(
        let intended_use = TerritoryIntendedUse::from_repr(instance.zone.intended_use).unwrap();
        let Some(director_type) = HandlerType::from_intended_use(intended_use) else {
            panic!("Unknown director for {intended_use}!");
        };
        let content_id = game_data
            .find_content_for_content_finder_id(content_finder_condition)
            .unwrap();

        let id = HandlerId::new(director_type, content_id);

        // Setup Lua state for our director
        let lua = KawariLua::new();

        // Find the script for this content
        let content_short_name = game_data
            .get_content_short_name(content_finder_condition)
            .unwrap();
        let file_name =
            FilesystemConfig::locate_script_file(&format!("content/{content_short_name}.lua"));

        let result = std::fs::read(&file_name);
        if let Err(err) = result {
            tracing::warn!(
                "Failed to load {}: {:?} instance content won't be scripted!",
                file_name,
                err
            );
        } else {
            let file = result.unwrap();

            if let Err(err) = lua
                .0
                .load(file)
                .set_name("@".to_string() + &file_name)
                .exec()
            {
                tracing::warn!(
                    "Syntax error in {}: {:?} instance content won't be scripted!",
                    file_name,
                    err
                );
            } else {
                let mut director = DirectorData {
                    id,
                    flag: 0,
                    data: [0; 10],
                    lua,
                    tasks: Vec::new(),
                    bosses: HashMap::new(),
                    shortcut_poprange_id: None,
                    battle_started_at: None,
                    elapsed_secs: 0.0,
                    last_tick_at: None,
                    scheduler: Vec::new(),
                    deaggro_since: None,
                    vars: HashMap::new(),
                    player_vars: HashMap::new(),
                    helpers: HashMap::new(),
                    omen_helpers: Vec::new(),
                    omen_rr: 0,
                    clones: Vec::new(),
                    current_actors: Vec::new(),
                    pending_battle_start: false,
                };

                // Call into the onSetup function before returning, as we need the flag to be initialized before any players change zones.
                director.setup();

                instance.director = Some(director);
            }
        }

        // TODO: init director even if script isn't found

        // Ensure we have the entrance set correctly
        let entrance_id = game_data
            .get_content_entrance_id(content_finder_condition)
            .expect("Failed to find entrance ID?");
        for range in &mut instance.zone.map_ranges {
            if range.instance_id == entrance_id {
                range.entrance = true;
                break;
            }
        }

        self.instances.push(instance);

        self.instances.last_mut()
    }

    fn find_actor_by_name(&self, name: &str) -> ObjectId {
        for instance in &self.instances {
            for (id, actor) in &instance.actors {
                if let Some(spawn) = actor.get_player_spawn()
                    && spawn.common.name == name
                {
                    return *id;
                }
            }
        }

        ObjectId::default()
    }

    /// Removes instances without players in them, which wastes resources.
    fn cleanup_dead_instances(&mut self) {
        self.instances.retain(|instance| {
            instance
                .actors
                .iter()
                .any(|x| matches!(x.1, NetworkedActor::Player { .. }))
        });
    }
}

// TODO: move elsewhere...
fn set_player_minion(
    data: &mut WorldServer,
    network: &mut NetworkState,
    minion_id: u32,
    from_actor_id: ObjectId,
) {
    // Update our common spawn to reflect the new minion
    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
        return;
    };

    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
        return;
    };

    let NetworkedActor::Player { spawn, .. } = actor else {
        return;
    };

    spawn.common.active_minion = minion_id as u16;

    network.send_ac_in_range_inclusive(
        data,
        from_actor_id,
        ActorControlCategory::MinionSpawnControl { minion_id },
    );
}

fn set_character_mode(
    instance: &mut Instance,
    network: &mut NetworkState,
    from_actor_id: ObjectId,
    mode: CharacterMode,
    mode_arg: u8,
) {
    // Update internal data model for new spawns
    {
        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        // Skip if this mode is already set.
        if actor.get_common_spawn().mode == mode && actor.get_common_spawn().mode_arg == mode_arg {
            return;
        }

        actor.get_common_spawn_mut().mode = mode;
        actor.get_common_spawn_mut().mode_arg = mode_arg;
    }

    // Inform actors
    network.send_ac_in_range_inclusive_instance(
        instance,
        from_actor_id,
        ActorControlCategory::SetMode {
            mode,
            mode_arg: mode_arg as u32,
        },
    );
}

fn set_shared_group_timeline_state(
    instance: &mut Instance,
    network: &mut NetworkState,
    from_actor_id: ObjectId,
    timelines: &[u32],
) {
    let mut state = SharedGroupTimelineState::empty();
    for timeline in timelines {
        state.toggle(match timeline {
            1 => SharedGroupTimelineState::TIMELINE_1,
            2 => SharedGroupTimelineState::TIMELINE_2,
            3 => SharedGroupTimelineState::TIMELINE_3,
            4 => SharedGroupTimelineState::TIMELINE_4,
            5 => SharedGroupTimelineState::TIMELINE_5,
            6 => SharedGroupTimelineState::TIMELINE_6,
            _ => unimplemented!(),
        });
    }

    // Update internal data model for new spawns
    {
        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
            return;
        };

        let NetworkedActor::Object { object, .. } = actor else {
            return;
        };

        object.args1 = state.bits();
    }

    // Inform actors
    network.send_ac_in_range_inclusive_instance(
        instance,
        from_actor_id,
        ActorControlCategory::SetSharedGroupTimelineState {
            state,
            unk2: 0,
            unk3: 0,
            unk4: 0,
        },
    );
}

/// Resolves DoT/HoT ticks and applies natural HP/MP regen for one instance. Called on the ~3s
/// regen tick. Matches retail, where the 3-second server tick drives both periodic statuses and
/// passive HP/MP recovery (HP regens even in combat, just more slowly; MP always regens).
fn process_regen_tick(network: Arc<Mutex<NetworkState>>, instance: &mut Instance) {
    /// Fraction of max HP recovered per tick while out of combat.
    const HP_REGEN_OOC: f32 = 0.06;
    /// Fraction of max HP recovered per tick while in combat (much smaller than out of combat).
    const HP_REGEN_COMBAT: f32 = 0.02;
    /// Fraction of max MP recovered per tick (retail regens MP in and out of combat).
    const MP_REGEN: f32 = 0.02;

    // Snapshot what each actor needs this tick. We can't mutate while iterating + reading source
    // parameters, so we gather (id, hp delta, mp delta) first, then apply.
    struct ActorTickPlan {
        id: ObjectId,
        /// Net HP change: negative = DoT damage, positive = HoT + natural regen.
        hp_delta: i64,
        /// Net MP change (natural regen only for now).
        mp_delta: i64,
    }

    // First, resolve a lookup of source actors' parameters for DoT/HoT potency math.
    let mut plans: Vec<ActorTickPlan> = Vec::new();

    // Per-tick DoT enmity to apply after the borrow loop: (enemy carrying the DoT, caster, amount).
    // DoT ticks keep generating enmity for the caster, like retail.
    let mut dot_enmity: Vec<(ObjectId, ObjectId, u32)> = Vec::new();

    // Per-tick floating damage/heal numbers to broadcast after HP is applied:
    // (status id, source/caster, target carrying the status, amount, is_heal). Retail shows a
    // number above the target every tick using the owning Status EXD row id.
    let mut tick_popups: Vec<(u32, ObjectId, ObjectId, u32, bool)> = Vec::new();

    // Collect the ids up-front so we can borrow the instance immutably per-actor below.
    let actor_ids: Vec<ObjectId> = instance.actors.keys().copied().collect();

    for id in actor_ids {
        let Some(actor) = instance.find_actor(id) else {
            continue;
        };

        // Only living characters tick.
        let Some(common) = actor.common_spawn() else {
            continue;
        };
        if common.mode == CharacterMode::Dead || common.health_points == 0 {
            continue;
        }
        let max_hp = common.max_health_points.max(1);
        let cur_hp = common.health_points;
        let cur_mp = common.resource_points;
        let max_mp = common.max_resource_points;

        // Gather this actor's DoT/HoT ticks (cloned so we drop the borrow before looking up sources).
        let ticks: Vec<crate::TickEffect> = actor
            .status_effects()
            .map(|s| s.tick_effects().to_vec())
            .unwrap_or_default();

        let (in_combat, is_player) = match actor {
            NetworkedActor::Player { combat_state, .. } => (combat_state.in_combat, true),
            NetworkedActor::Npc { hate_list, .. } => (!hate_list.is_empty(), false),
            _ => (false, false),
        };

        let mut hp_delta: i64 = 0;
        let mut mp_delta: i64 = 0;

        // Resolve DoT/HoT magnitudes using the *source* actor's parameters (retail computes DoT
        // damage from the caster's stats). NPC sources have no BaseParameters, so fall back to a
        // rough flat value derived from potency.
        for tick in &ticks {
            let source_params = instance.find_actor(tick.source_actor_id).and_then(|a| {
                if let NetworkedActor::Player { parameters, .. } = a {
                    Some(parameters.clone())
                } else {
                    None
                }
            });

            use crate::TickEffectKind;
            let magnitude = match tick.kind {
                TickEffectKind::DamageMagic => source_params
                    .as_ref()
                    .map(|p| p.calc_magical_damage(tick.potency as u32) as i64)
                    .unwrap_or((tick.potency / 2) as i64),
                TickEffectKind::DamagePhysical => source_params
                    .as_ref()
                    .map(|p| p.calc_physical_damage(tick.potency as u32) as i64)
                    .unwrap_or((tick.potency / 2) as i64),
                TickEffectKind::Heal => source_params
                    .as_ref()
                    .map(|p| p.calc_heal_amount(tick.potency as u32) as i64)
                    .unwrap_or((tick.potency / 2) as i64),
                TickEffectKind::RestoreMp => tick.potency as i64,
            };

            match tick.kind {
                TickEffectKind::DamageMagic | TickEffectKind::DamagePhysical => {
                    hp_delta -= magnitude;
                    // The DoT lives on this actor (`id` = the enemy); credit its enmity to the
                    // caster so sustained DoTs keep the caster on the hate list.
                    if magnitude > 0 {
                        dot_enmity.push((id, tick.source_actor_id, magnitude as u32));
                        // Floating damage number above the target each tick (caster -> this actor).
                        tick_popups.push((
                            tick.effect_id as u32,
                            tick.source_actor_id,
                            id,
                            magnitude.min(u32::MAX as i64) as u32,
                            false,
                        ));
                    }
                }
                TickEffectKind::Heal => {
                    hp_delta += magnitude;
                    if magnitude > 0 {
                        // Floating heal number above the target each tick (caster -> this actor).
                        tick_popups.push((
                            tick.effect_id as u32,
                            tick.source_actor_id,
                            id,
                            magnitude.min(u32::MAX as i64) as u32,
                            true,
                        ));
                    }
                }
                TickEffectKind::RestoreMp => {
                    mp_delta += magnitude;
                }
            }
        }

        // Natural regen. Only players regen passively (HP in and out of combat, plus MP). NPCs do
        // NOT naturally regen — out of combat they're snapped back to full by the existing deaggro
        // path, and in combat they only change HP via damage/heal effects.
        if is_player && cur_hp < max_hp {
            let frac = if in_combat {
                HP_REGEN_COMBAT
            } else {
                HP_REGEN_OOC
            };
            hp_delta += (max_hp as f32 * frac).ceil() as i64;
        }
        if is_player && cur_mp < max_mp {
            mp_delta += (max_mp as f32 * MP_REGEN).ceil() as i64;
        }

        if hp_delta != 0 || mp_delta != 0 {
            plans.push(ActorTickPlan {
                id,
                hp_delta,
                mp_delta,
            });
        }
    }

    // Apply per-tick DoT enmity (collected above) to each affected enemy's hate list.
    for (enemy_id, caster_id, amount) in dot_enmity {
        if let Some(actor) = instance.find_actor_mut(enemy_id)
            && let Some(hate_list) = actor.npc_hate_list_mut()
        {
            let entry = hate_list.entry(caster_id).or_insert(0);
            *entry = entry.saturating_add(amount);
        }
    }

    // Apply the plans and notify clients.
    for plan in plans {
        let Some(actor) = instance.find_actor_mut(plan.id) else {
            continue;
        };
        let max_hp = actor.get_common_spawn().max_health_points.max(1);
        let max_mp = actor.get_common_spawn().max_resource_points;

        if plan.hp_delta < 0 {
            let damage = (-plan.hp_delta) as u32;
            actor.apply_damage(damage);
        } else if plan.hp_delta > 0 {
            let common = actor.get_common_spawn_mut();
            common.health_points = common
                .health_points
                .saturating_add(plan.hp_delta as u32)
                .min(max_hp);
        }

        if plan.mp_delta > 0 {
            let common = actor.get_common_spawn_mut();
            common.resource_points =
                ((common.resource_points as i64 + plan.mp_delta).min(max_mp as i64)) as u16;
        }

        update_actor_hp_mp(network.clone(), instance, plan.id);
    }

    // Broadcast a floating damage/heal number above each target for every DoT/HoT tick this round.
    // Retail uses an ActorControl sourced from the *target* actor, carrying the amount and the
    // caster id — this is what gives ticks their distinct number style (a plain ActionResult would
    // render them like a normal action hit instead). DoT and HoT use different categories; the
    // owner status id is carried in param1. `unk2` still has multiple client branches and is
    // not fully identified.
    if !tick_popups.is_empty() {
        let mut net = network.lock();
        for (status_id, source_id, target_id, amount, is_heal) in tick_popups {
            let category = if is_heal {
                ActorControlCategory::TickHeal {
                    status_id,
                    amount: amount as u32,
                    source_actor_id: source_id,
                    unk2: 0,
                    unk3: 0,
                }
            } else {
                ActorControlCategory::TickDamage {
                    status_id,
                    amount: amount as u32,
                    source_actor_id: source_id,
                    unk2: u32::MAX,
                    unk3: 0,
                }
            };
            net.send_in_range_inclusive_instance(
                target_id,
                instance,
                FromServer::ActorControl(target_id, category),
                DestinationNetwork::ZoneClients,
            );
        }
    }
}

/// Schedule each pending AoE on its own precise timer. After `delay` seconds, re-locate the
/// instance (by the AoE's source actor) and resolve it, snapshotting player positions at exactly
/// that moment — independent of the coarse director tick. Mirrors the instant-action `spawn + sleep`
/// pattern so mechanic snapshots land on time.
fn schedule_pending_aoes(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    pending: Vec<PendingAoe>,
) {
    for aoe in pending {
        let data = data.clone();
        let network = network.clone();
        let source_id = aoe.source_id;
        let delay = Duration::from_secs_f32(aoe.delay.max(0.0));
        tokio::task::spawn(async move {
            tokio::time::sleep(delay).await;
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(source_id) else {
                return;
            };
            resolve_aoe(network, instance, aoe);
        });
    }
}

fn server_logic_tick(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    lua: Arc<Mutex<KawariLua>>,
    gamedata: Arc<Mutex<GameData>>,
) {
    let mut actors_to_update_hp_mp = Vec::new();
    let mut actors_to_fake_zone_jump = Vec::new();
    let mut actors_to_refresh_effects = Vec::new();
    let mut pending_aoes: Vec<PendingAoe> = Vec::new();

    {
        let mut data = data.lock();
        let rested_exp_counter = data.rested_exp_counter;
        let party_positions_counter = data.party_positions_counter;
        let regen_tick = data.regen_tick_counter == 0;

        data.cleanup_dead_instances();

        // Send a periodic update to all parties about where their members are in the world.
        // TODO: On retail this is sent once every 5 seconds, so sending this at a slower interval would be more ideal.
        if party_positions_counter == 0 {
            let mut network = network.lock();
            send_party_positions(&mut network);
        }

        for instance in &mut data.instances {
            let mut haters = HashMap::new();
            npc_behavior::npc_behavior(
                network.clone(),
                lua.clone(),
                gamedata.clone(),
                instance,
                &mut haters,
            );

            let mut actors_now_gimmick_jumping = Vec::new();
            let mut actors_now_inside_instance_exits = Vec::new();
            let mut actors_now_outside_instance_entrances = Vec::new();

            let player_ids: Vec<ObjectId> = instance
                .actors
                .iter()
                .filter_map(|(id, actor)| match actor {
                    NetworkedActor::Player { .. } => Some(*id),
                    _ => None,
                })
                .collect();

            // Player area stuffs
            for id in &player_ids {
                let Some(NetworkedActor::Player {
                    conditions,
                    executing_gimmick_jump,
                    inside_instance_exit: inside_instance_entrance,
                    distance_range,
                    ..
                }) = instance.find_actor(*id)
                else {
                    continue;
                };
                let conditions = *conditions;
                let executing_gimmick_jump = *executing_gimmick_jump;
                let inside_instance_entrance = *inside_instance_entrance;
                let player_position = instance.find_actor(*id).unwrap().position();
                let player_distance_range = *distance_range;

                // Find the ClientState for this player.
                let mut network = network.lock();

                let hated_npcs = haters.get(id).cloned().unwrap_or_default();

                let (assigned_slots, removed_haters) =
                    if let Some(actor) = instance.find_actor_mut(*id) {
                        sync_player_hated_by(actor, &hated_npcs)
                    } else {
                        (Vec::new(), Vec::new())
                    };

                for actor_id in removed_haters {
                    let msg = FromServer::ActorControlTarget(
                        actor_id,
                        ObjectTypeId::default(),
                        ActorControlCategory::UpdateHater { unk1: 0 },
                    );
                    network.send_to_by_actor_id(*id, msg, DestinationNetwork::ZoneClients);
                }

                for (actor_id, slot) in assigned_slots {
                    let msg = FromServer::ActorControlTarget(
                        actor_id,
                        ObjectTypeId::default(),
                        ActorControlCategory::Unknown {
                            category: 0x1F7,
                            param1: slot as u32,
                            param2: actor_id.0,
                            param3: 0,
                            param4: 0,
                            param5: 0,
                        },
                    );
                    network.send_to_by_actor_id(*id, msg, DestinationNetwork::ZoneClients);
                }

                // The two enmity packets are mirror images (matching Sapphire's sendHateList):
                //  - HaterList: { actor_id = the NPC, enmity = 0-100 percentage }. The percentage
                //    drives the aggro-indicator colour — 100% (you're top of its hate list) shows
                //    the red "you have aggro" state. Sending the raw value (what we did before)
                //    makes the client read garbage as the percent and mis-colour it (green/orange).
                //  - EnmityList: { actor_id = the player themselves, enmity = 0-100 percentage }.
                let player_enmity: Vec<(ObjectId, u32, u32)> = hated_npcs
                    .iter()
                    .map(|(npc_id, enmity)| {
                        let max_enmity = instance
                            .find_actor(*npc_id)
                            .and_then(|actor| actor.npc_hate_list())
                            .and_then(|hate_list| hate_list.values().copied().max())
                            .unwrap_or(*enmity);
                        let rate = if max_enmity == 0 {
                            0
                        } else {
                            ((*enmity as f32 / max_enmity as f32) * 100.0).round() as u32
                        };
                        (*npc_id, *enmity, rate)
                    })
                    .collect();

                // Change detection: only resend the enmity packets when the (npc, rate%) snapshot
                // differs from what we last sent this player. Without this we'd spam both packets
                // (including empty ones for players with no haters) every 300ms tick.
                let mut new_snapshot: Vec<(ObjectId, u8)> = player_enmity
                    .iter()
                    .map(|(npc_id, _raw, rate)| (*npc_id, (*rate).min(100) as u8))
                    .collect();
                new_snapshot.sort_by_key(|(npc_id, _)| npc_id.0);

                let enmity_changed = match instance.find_actor_mut(*id) {
                    Some(NetworkedActor::Player {
                        last_enmity_sent, ..
                    }) => {
                        if *last_enmity_sent != new_snapshot {
                            *last_enmity_sent = new_snapshot;
                            true
                        } else {
                            false
                        }
                    }
                    _ => true,
                };

                if enmity_changed {
                    if !player_enmity.is_empty() {
                        let mut list: Vec<Hater> = player_enmity
                            .iter()
                            .map(|(npc_id, _raw, rate)| Hater {
                                actor_id: *npc_id,
                                enmity: (*rate).min(100) as u8,
                            })
                            .collect();
                        list.truncate(32);
                        let ipc =
                            ServerZoneIpcSegment::new(ServerZoneIpcData::HaterList(HaterList {
                                count: list.len() as u8,
                                list,
                            }));
                        network.send_to_by_actor_id(
                            *id,
                            FromServer::PacketSegment(ipc, *id),
                            DestinationNetwork::ZoneClients,
                        );
                    } else {
                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::HaterList(
                            HaterList::default(),
                        ));
                        network.send_to_by_actor_id(
                            *id,
                            FromServer::PacketSegment(ipc, *id),
                            DestinationNetwork::ZoneClients,
                        );
                    }

                    let mut enmity_list: Vec<PlayerEnmity> = player_enmity
                        .iter()
                        .map(|(_npc_id, _raw, rate)| PlayerEnmity {
                            actor_id: *id,
                            enmity: (*rate).min(100) as u8,
                        })
                        .collect();
                    enmity_list.truncate(8);

                    let ipc =
                        ServerZoneIpcSegment::new(ServerZoneIpcData::EnmityList(EnmityList {
                            count: enmity_list.len() as u8,
                            list: enmity_list,
                        }));
                    network.send_to_by_actor_id(
                        *id,
                        FromServer::PacketSegment(ipc, *id),
                        DestinationNetwork::ZoneClients,
                    );
                }

                let Some((handle, state)) = network.get_by_actor_mut(*id) else {
                    continue;
                };

                // Check for overlapping map ranges
                let overlapping_ranges =
                    instance.zone.get_overlapping_map_ranges(player_position.0);
                let in_sanctuary = overlapping_ranges.iter().filter(|x| x.sanctuary).count() > 0;

                // We're on the 10 second mark, and you're in a sanctuary...
                if in_sanctuary && rested_exp_counter == 0 {
                    // Update rested EXP! (This means it only has a ten second granularity, but who cares?)
                    let msg = FromServer::IncrementRestedExp();
                    if handle.send(msg).is_err() {
                        // TODO: remove as needed
                        //self.to_remove.push(id);
                    }
                }

                let mut inside_any_instance_entrances = false;

                // Process gimmicks
                if !executing_gimmick_jump {
                    for range in &overlapping_ranges {
                        if let Some(gimmick) = &range.gimmick {
                            match gimmick {
                                MapGimmick::Generic {} => {
                                    if let Some(director) = &mut instance.director {
                                        director.on_gimmick_rect(range.instance_id);
                                    }
                                }
                                MapGimmick::Jump {
                                    to_position,
                                    gimmick_jump_type,
                                    sgb_animation_id,
                                    eobj_instance_id,
                                } => {
                                    // Tell the client to execute the gimmick jump
                                    let msg = FromServer::ActorControlSelf(
                                        ActorControlCategory::ExecuteGimmickJump {
                                            landing_position_x: to_position.x,
                                            landing_position_y: to_position.y,
                                            landing_position_z: to_position.z,
                                            gimmick_jump_type: *gimmick_jump_type,
                                            unk1: 0,
                                        },
                                    );
                                    actors_now_gimmick_jumping.push(*id);
                                    if handle.send(msg).is_err() {
                                        // TODO: remove as needed
                                        //self.to_remove.push(id);
                                    }

                                    // Play the corresponding animation for the EObj
                                    if let Some(eobj) = instance.find_object(*eobj_instance_id) {
                                        let msg = FromServer::ActorControl(
                                            eobj,
                                            ActorControlCategory::PlaySharedGroupTimeline {
                                                timeline_id: *sgb_animation_id,
                                            },
                                        );
                                        if handle.send(msg).is_err() {
                                            // TODO: remove as needed
                                            //self.to_remove.push(id);
                                        }
                                    }
                                }
                                MapGimmick::FakeExit { exit_pop_range_id } => {
                                    actors_to_fake_zone_jump.push((*id, *exit_pop_range_id));
                                }
                            }
                        }
                        if range.entrance {
                            inside_any_instance_entrances = true;
                            if !inside_instance_entrance {
                                let msg =
                                    FromServer::EnteredInstanceEntranceRange(range.instance_id);
                                actors_now_inside_instance_exits.push(*id);
                                if handle.send(msg).is_err() {
                                    // TODO: remove as needed
                                    //self.to_remove.push(id);
                                }
                            }
                        }
                    }
                }

                if !inside_any_instance_entrances {
                    actors_now_outside_instance_entrances.push(*id);
                }

                let is_in_duel_area = overlapping_ranges.iter().filter(|x| x.duel).count() > 0;
                let has_duel_condition = conditions.has_condition(Condition::InDuelingArea);

                if is_in_duel_area != has_duel_condition {
                    // Update conditions
                    {
                        let mut conditions = conditions;
                        conditions.toggle_condition(Condition::InDuelingArea, is_in_duel_area);

                        let msg = FromServer::Conditions(conditions);
                        if handle.send(msg).is_err() {
                            // TODO: remove as needed
                            //self.to_remove.push(id);
                        }
                    }

                    // Send log message
                    {
                        let log_message = if is_in_duel_area {
                            2692 // Duels permitted in current area.
                        } else {
                            2693 // Duels not permitted in current area.
                        };

                        let msg = FromServer::ActorControlSelf(ActorControlCategory::LogMessage {
                            log_message,
                            id: 0,
                        });
                        if handle.send(msg).is_err() {
                            // TODO: remove as needed
                            //self.to_remove.push(id);
                        }
                    }
                }

                // We want to prioritize actors closest to us!
                let mut actors_by_distance = instance.actors.iter().collect::<Vec<_>>();
                actors_by_distance.sort_by(|a, b| {
                    let a_position = a.1.position();
                    let b_position = b.1.position();

                    let a_distance = Vec3::distance(player_position.0, a_position.0);
                    let b_distance = Vec3::distance(player_position.0, b_position.0);

                    a_distance.total_cmp(&b_distance)
                });
                for (other_id, other_actor) in actors_by_distance {
                    // We're always in our own view
                    if *id == *other_id {
                        continue;
                    }

                    // If the actor isn't valid, don't bother spawning yet.
                    if !other_actor.is_valid() {
                        continue;
                    }

                    // If the actor _should_ be in the view of the other.
                    let in_range = {
                        let mut other_pos = other_actor.position().0;
                        other_pos.y = 0.0;
                        let mut self_pos = player_position.0;
                        self_pos.y = 0.0;
                        Vec3::distance(self_pos, other_pos) < player_distance_range.distance()
                    };
                    let has_been_spawned = state.has_spawned(*other_id);

                    // There are four states:
                    // Walked out = (Has been spawned, no longer in range)
                    // Walked in = (Hasn't been spawned, in range)
                    // Still in = (Has been spawned, in range)
                    // Still out = (Hasn't been spawned, not in range)

                    let walked_out = has_been_spawned && !in_range;
                    let walked_in = !has_been_spawned && in_range;
                    let still_in = has_been_spawned && in_range;
                    let still_out = !has_been_spawned && !in_range;

                    if walked_out {
                        if let Some(spawn_index) = state.actor_allocator.free(*other_id) {
                            let msg = FromServer::DeleteActor(*other_id, spawn_index);

                            if handle.send(msg).is_err() {
                                // TODO: remove as needed
                                //self.to_remove.push(id);
                            }
                        } else if let Some(spawn_index) = state.object_allocator.free(*other_id) {
                            let msg = FromServer::DeleteObject(spawn_index);

                            if handle.send(msg).is_err() {
                                // TODO: remove as needed
                                //self.to_remove.push(id);
                            }
                        }
                    } else if walked_in {
                        // Spawn this actor
                        if let Some(msg) = NetworkState::spawn_existing_actor_message(
                            state,
                            *other_id,
                            other_actor,
                        ) {
                            if handle.send(msg).is_err() {
                                // TODO: remove as needed
                                //self.to_remove.push(id);
                            }

                            // If this NPC is already in combat, the claim ACTs that turn its
                            // nameplate red (SetTarget/SetBattle/FirstAttack) were broadcast — with
                            // `only_spawned` — back when it first acquired its target, before this
                            // client had spawned it, so they were silently dropped. (Happens when a
                            // player appears inside aggro range before the spawn lands, e.g. a
                            // gm-pos teleport onto a mob.) Re-send them to just this client now,
                            // after the spawn it just received, so the nameplate goes red.
                            if let NetworkedActor::Npc { spawn, .. } = other_actor
                                && spawn.common.combat_tag_type != 0
                                && spawn.common.combat_tagger_id.object_id.is_valid()
                            {
                                let target_id = spawn.common.combat_tagger_id.object_id;
                                let target = ObjectTypeId {
                                    object_id: target_id,
                                    object_type: ObjectTypeKind::None,
                                };
                                let _ = handle.send(FromServer::ActorControlTarget(
                                    *other_id,
                                    target,
                                    ActorControlCategory::SetTarget {},
                                ));
                                let _ = handle.send(FromServer::ActorControl(
                                    *other_id,
                                    ActorControlCategory::SetBattle { battle: true },
                                ));
                                let ipc = ServerZoneIpcSegment::new(
                                    ServerZoneIpcData::FirstAttack {
                                        unk1: 1,
                                        unk2: 0,
                                        combat_tagger: target_id,
                                        unk3: 0,
                                    },
                                );
                                let _ = handle.send(FromServer::PacketSegment(ipc, *other_id));
                            }

                            // The spawn packet has no targetable field, so a fresh client spawn is
                            // always targetable. Visual-only NPCs (e.g. Crimson Cyclone clones) carry
                            // `targetable = false`; re-apply it here so they can't be selected after
                            // each walk-in (they walk out/in every cycle).
                            if let NetworkedActor::Npc {
                                targetable: false, ..
                            } = other_actor
                            {
                                let _ = handle.send(FromServer::ActorControl(
                                    *other_id,
                                    ActorControlCategory::Targetable { targetable: false },
                                ));
                            }
                            // Re-apply visibility: it's a runtime ActorControl (414, the client's
                            // bit16), not part of the spawn packet, so a fresh walked-in spawn would
                            // show a hidden clone at full opacity without this.
                            if let NetworkedActor::Npc { visible: false, .. } = other_actor {
                                let _ = handle.send(FromServer::ActorControl(
                                    *other_id,
                                    ActorControlCategory::ToggleVisibility {
                                        visible: false,
                                        duration: 0.0,
                                    },
                                ));
                            }
                        } else {
                            // Early exit if the client refuses to spawn any more actors
                            continue;
                        }
                    } else if still_in || still_out {
                        // Do nothing
                    } else {
                        unreachable!();
                    }
                }
            }

            // Set players as gimmick jumping, as the client does *not* send position updates during it.
            for actor in &actors_now_gimmick_jumping {
                let Some(NetworkedActor::Player {
                    executing_gimmick_jump,
                    ..
                }) = instance.find_actor_mut(*actor)
                else {
                    continue;
                };

                *executing_gimmick_jump = true;
            }

            // TODO: we probably need a better "we just entered this maprect" event instead of this
            for actor in &actors_now_inside_instance_exits {
                let Some(NetworkedActor::Player {
                    inside_instance_exit,
                    ..
                }) = instance.find_actor_mut(*actor)
                else {
                    continue;
                };

                *inside_instance_exit = true;
            }

            for actor in &actors_now_outside_instance_entrances {
                let Some(NetworkedActor::Player {
                    inside_instance_exit,
                    ..
                }) = instance.find_actor_mut(*actor)
                else {
                    continue;
                };

                *inside_instance_exit = false;
            }

            // NOTE: I know this isn't retail accurate
            for (id, actor) in &mut instance.actors {
                if let NetworkedActor::Player {
                    spawn,
                    combat_state,
                    ..
                } = actor
                {
                    let is_dead = spawn.common.health_points == 0;
                    let in_combat =
                        haters.get(id).is_some_and(|entries| !entries.is_empty()) && !is_dead;

                    // Toggle the player's battle state when their aggro status changes — this is
                    // what makes the client draw the weapon and play combat music. Only send on a
                    // transition so we don't spam the client every tick.
                    if in_combat != combat_state.in_combat {
                        combat_state.in_combat = in_combat;
                        let mut network = network.lock();
                        network.send_to_by_actor_id(
                            *id,
                            FromServer::ActorControlSelf(ActorControlCategory::SetBattle {
                                battle: in_combat,
                            }),
                            DestinationNetwork::ZoneClients,
                        );
                    }

                    // Don't heal people who are in combat or dead, please.
                    if in_combat || is_dead {
                        continue;
                    }

                    let mut updated = false;
                    if spawn.common.health_points != spawn.common.max_health_points {
                        let amount = (spawn.common.max_health_points as f32 * 0.10).round() as u32;
                        spawn.common.health_points = u32::clamp(
                            spawn.common.health_points + amount,
                            0,
                            spawn.common.max_health_points,
                        );
                        updated = true;
                    }

                    if spawn.common.resource_points != spawn.common.max_resource_points {
                        let amount =
                            (spawn.common.max_resource_points as f32 * 0.10).round() as u16;
                        spawn.common.resource_points = u16::clamp(
                            spawn.common.resource_points + amount,
                            0,
                            spawn.common.max_resource_points,
                        );
                        updated = true;
                    }

                    if updated {
                        actors_to_update_hp_mp.push(*id);
                    }
                }
            }

            // generate navmesh if necessary
            match &instance.generate_navmesh {
                NavmeshGenerationStep::None => {}
                NavmeshGenerationStep::Needed(nvm_path) => {
                    tracing::info!(
                        "Missing navmesh {nvm_path:?}, we are going to generate it in the background now..."
                    );

                    let mut dir = std::env::current_exe().unwrap();
                    dir.pop();
                    dir.push(format!("kawari-navimesh{EXE_SUFFIX}"));

                    // start navimesh generator
                    match Command::new(dir)
                        .arg(instance.zone.id.to_string())
                        .arg(nvm_path)
                        .spawn()
                    {
                        Ok(_) => {
                            instance.generate_navmesh =
                                NavmeshGenerationStep::Started(nvm_path.clone())
                        }
                        Err(err) => {
                            tracing::error!(
                                "Unable to run kawari-navimesh due to the following error: {err}"
                            );
                            instance.generate_navmesh = NavmeshGenerationStep::None;
                        }
                    }
                }
                NavmeshGenerationStep::Started(nvm_path) => {
                    if let Ok(nvm_bytes) = std::fs::read(nvm_path) {
                        if let Some(navmesh) = Navmesh::from_existing(&nvm_bytes) {
                            instance.navmesh = navmesh;

                            tracing::info!("Successfully loaded navimesh from {nvm_path:?}");
                        } else {
                            tracing::warn!(
                                "Failed to read {nvm_path:?}, monsters will not function correctly!"
                            );
                        }
                        instance.generate_navmesh = NavmeshGenerationStep::None;
                    }
                }
            }

            // Process any director tasks for this instance.
            pending_aoes.extend(director_tick(network.clone(), gamedata.clone(), instance));

            // Every ~3 seconds: resolve DoT/HoT ticks and natural HP/MP regen, matching retail's
            // 3s server tick (which also drives passive regen).
            if regen_tick {
                process_regen_tick(network.clone(), instance);
            }

            {
                let mut network = network.lock();
                actors_to_refresh_effects.extend(refresh_runtime_job_state(instance, &mut network));
            }
        }
        // Ensure the rested EXP counter only happens approx. every 10 seconds (300ms tick * 33).
        data.rested_exp_counter += 1;
        if data.rested_exp_counter == 33 {
            data.rested_exp_counter = 0;
        }

        // Ensure the party positions counter only happens approx. every 5 seconds (300ms tick * 17).
        data.party_positions_counter += 1;
        if data.party_positions_counter == 17 {
            data.party_positions_counter = 0;
        }

        // Drive the 3-second DoT/HoT + natural regen tick (300ms tick * 10 = 3s).
        data.regen_tick_counter += 1;
        if data.regen_tick_counter == 10 {
            data.regen_tick_counter = 0;
        }
    }

    // Schedule precise timers for any AoEs queued this tick, now that the data lock is released.
    schedule_pending_aoes(data.clone(), network.clone(), pending_aoes);

    for id in actors_to_update_hp_mp {
        let mut data = data.lock();
        let instance = data.find_actor_instance_mut(id).unwrap();
        update_actor_hp_mp(network.clone(), instance, id);
    }

    for id in actors_to_refresh_effects {
        let data = data.lock();
        if let Some(instance) = data.find_actor_instance(id) {
            send_effects_list(network.clone(), instance, id);
        }
    }

    for (id, exit_pop_range_id) in actors_to_fake_zone_jump {
        let mut data = data.lock();
        let mut network = network.lock();
        let mut game_data = gamedata.lock();
        let from_id = network.find_by_actor(id).unwrap();
        change_zone_warp_to_pop_range(
            &mut data,
            &mut network,
            &mut game_data,
            None, // None here means that we don't want to change their current instance
            exit_pop_range_id,
            id,
            from_id,
            WarpType::Event,
            0,
            0,
            0,
        );
    }
}

pub async fn server_main_loop(
    game_data: GameData,
    parties: HashMap<u64, Party>,
    linkshells: HashMap<u64, Vec<ObjectId>>,
    mut recv: Receiver<ToServer>,
) -> Result<(), std::io::Error> {
    let data = Arc::new(Mutex::new(WorldServer::default()));
    let network = Arc::new(Mutex::new(NetworkState {
        parties,
        linkshells,
        ..Default::default()
    }));
    let game_data = Arc::new(Mutex::new(game_data));
    let lua = Arc::new(Mutex::new(KawariLua::new()));

    // Run Init.lua and set up other Lua state
    {
        let mut lua = lua.lock();
        if let Err(err) = lua.init(game_data.clone()) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }

    {
        let data = data.clone();
        let network = network.clone();
        let game_data = game_data.clone();
        let lua = lua.clone();
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(300)); // Be careful when changing this, as the rested EXP may become whacky. Counters in server_logic_tick are sized off this interval.
            interval.tick().await;
            loop {
                interval.tick().await;

                // Execute general server logic
                server_logic_tick(
                    data.clone(),
                    network.clone(),
                    lua.clone(),
                    game_data.clone(),
                );

                // Execute list of queued tasks
                {
                    let mut tasks_to_execute = Vec::new();

                    // Gather list of tasks to execute
                    {
                        let mut data = data.lock();
                        for (i, instance) in data.instances.iter_mut().enumerate() {
                            for task in &instance.queued_task {
                                if task.point <= Instant::now() {
                                    tasks_to_execute.push((i, task.clone()));
                                }
                            }
                            // Keep all tasks that happen in the future.
                            instance.queued_task.retain(|x| x.point > Instant::now());
                        }
                    }

                    for (instance_index, task) in &tasks_to_execute {
                        match &task.data {
                            QueuedTaskData::CastAction { request, .. } => {
                                execute_action(
                                    network.clone(),
                                    data.clone(),
                                    game_data.clone(),
                                    lua.clone(),
                                    task.from_id,
                                    task.from_actor_id,
                                    request.clone(),
                                );
                            }
                            QueuedTaskData::CastEnemyAction { request, .. } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    execute_enemy_action(
                                        network.clone(),
                                        instance,
                                        lua.clone(),
                                        task.from_actor_id,
                                        request.clone(),
                                    );
                                }
                            }
                            QueuedTaskData::LoseStatusEffect {
                                effect_id,
                                effect_param,
                                effect_source_actor_id,
                            } => {
                                remove_effect(
                                    network.clone(),
                                    data.clone(),
                                    lua.clone(),
                                    task.from_id,
                                    task.from_actor_id,
                                    *effect_id,
                                    *effect_param,
                                    *effect_source_actor_id,
                                );
                            }
                            QueuedTaskData::DeadFadeOut { actor_id } => {
                                let mut data = data.lock();

                                let mut network = network.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    network.send_ac_in_range_instance(
                                        instance,
                                        *actor_id,
                                        ActorControlCategory::DeadFadeOut {},
                                    );

                                    instance.insert_task(
                                        ClientId::default(),
                                        ObjectId::default(),
                                        DEAD_DESPAWN_TIME,
                                        QueuedTaskData::DeadDespawn {
                                            actor_id: *actor_id,
                                        },
                                    );
                                }
                            }
                            QueuedTaskData::DeadDespawn { actor_id } => {
                                // despawn
                                let mut data = data.lock();
                                if let Some(instance) = data.find_actor_instance_mut(*actor_id) {
                                    let mut network = network.lock();
                                    network.remove_actor(instance, *actor_id);
                                }
                            }
                            QueuedTaskData::CastEventAction { target } => {
                                let mut data = data.lock();
                                if let Some(instance) =
                                    data.find_actor_instance_mut(task.from_actor_id)
                                    && let Some(director) = &mut instance.director
                                {
                                    director.event_action_cast(task.from_actor_id, *target);
                                }
                            }
                            QueuedTaskData::FishBite => {
                                let mut network = network.lock();
                                network.send_to(
                                    task.from_id,
                                    FromServer::FishBite(),
                                    DestinationNetwork::ZoneClients,
                                );
                            }
                            QueuedTaskData::SealBossWall { id, place_name } => {
                                let mut data = data.lock();
                                if let Some(instance) =
                                    data.find_actor_instance_mut(task.from_actor_id)
                                    && let Some(director) = &mut instance.director
                                {
                                    director.seal_boss_wall(*id, *place_name);
                                }
                            }
                            QueuedTaskData::PacketSegment { segment } => {
                                let mut network = network.lock();
                                network.send_to(
                                    task.from_id,
                                    FromServer::PacketSegment(segment.clone(), task.from_actor_id),
                                    DestinationNetwork::ZoneClients,
                                );
                            }
                            QueuedTaskData::WarpToPopRange { id } => {
                                let mut data = data.lock();
                                let mut network = network.lock();
                                let mut game_data = game_data.lock();

                                let from_id = network.find_by_actor(task.from_actor_id).unwrap();

                                change_zone_warp_to_pop_range(
                                    &mut data,
                                    &mut network,
                                    &mut game_data,
                                    None, // Means we don't want to change their current instance
                                    *id,
                                    task.from_actor_id,
                                    from_id,
                                    WarpType::Normal,
                                    0,
                                    0,
                                    0,
                                );
                            }
                            QueuedTaskData::RevealPet { actor_id } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.find_actor_instance_mut(*actor_id) {
                                    let mut network = network.lock();
                                    summoner::send_retail_pet_reveal_controls(
                                        &mut network,
                                        instance,
                                        *actor_id,
                                    );
                                }
                            }
                            QueuedTaskData::SummonerPrimalFinisher {
                                owner_id,
                                pet_id,
                                preferred_target_id,
                                action_id,
                                potency,
                                expires_at,
                            } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    if let Some(target_id) =
                                        summoner::process_elemental_primal_finisher(
                                            network.clone(),
                                            instance,
                                            *owner_id,
                                            *pet_id,
                                            *preferred_target_id,
                                            *action_id,
                                            *potency,
                                            *expires_at,
                                        )
                                    {
                                        update_actor_hp_mp(network.clone(), instance, target_id);
                                    }
                                }
                            }
                            QueuedTaskData::SummonerPrimalRevert { owner_id, pet_id } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    let mut network = network.lock();
                                    summoner::apply_elemental_primal_revert(
                                        &mut network,
                                        instance,
                                        *owner_id,
                                        *pet_id,
                                    );
                                }
                            }
                            QueuedTaskData::SummonerDemiAutoAttack { owner_id } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index)
                                    && let Some(target_id) = summoner::process_demi_auto_attack(
                                        network.clone(),
                                        instance,
                                        *owner_id,
                                    )
                                {
                                    update_actor_hp_mp(network.clone(), instance, target_id);
                                }
                            }
                            QueuedTaskData::SummonerSlipstreamTick {
                                owner_id,
                                center,
                                radius,
                                potency,
                                ticks_remaining,
                            } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    let hit_targets = {
                                        let mut network = network.lock();
                                        summoner::process_slipstream_lingering_tick(
                                            &mut network,
                                            instance,
                                            *owner_id,
                                            *center,
                                            *radius,
                                            *potency,
                                            *ticks_remaining,
                                        )
                                    };
                                    for target_id in hit_targets {
                                        update_actor_hp_mp(network.clone(), instance, target_id);
                                    }
                                }
                            }
                            QueuedTaskData::SummonerSlipstreamGroundVfx { owner_id, center } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    let object_id = ObjectId(fastrand::u32(..));
                                    {
                                        let mut network = network.lock();
                                        summoner::spawn_slipstream_ground_vfx(
                                            &mut network,
                                            instance,
                                            *owner_id,
                                            *center,
                                            object_id,
                                        );
                                    }
                                    instance.insert_task(
                                        ClientId::default(),
                                        *owner_id,
                                        summoner::slipstream_ground_vfx_duration(),
                                        QueuedTaskData::SummonerSlipstreamGroundVfxCleanup {
                                            object_id,
                                        },
                                    );
                                }
                            }
                            QueuedTaskData::SummonerSlipstreamGroundVfxCleanup { object_id } => {
                                let mut data = data.lock();
                                if let Some(instance) = data.instances.get_mut(*instance_index) {
                                    let mut network = network.lock();
                                    summoner::despawn_slipstream_ground_vfx(
                                        &mut network,
                                        instance,
                                        *object_id,
                                    );
                                }
                            }
                            QueuedTaskData::ResetCombo => {
                                let mut data = data.lock();
                                if let Some(instance) =
                                    data.find_actor_instance_mut(task.from_actor_id)
                                    && let Some(NetworkedActor::Player {
                                        last_combo_action,
                                        combo_sequence,
                                        ..
                                    }) = instance.find_actor_mut(task.from_actor_id)
                                {
                                    *last_combo_action = 0;
                                    *combo_sequence = 0;
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    // High-frequency (~125ms / 8Hz) encounter loop. Advances only instances that have a director:
    // their scheduled timeline events and `onTick`, then drains the director tasks those produce.
    // Kept separate from the 300ms loop so mechanic timing isn't bound to that coarse tick. Locks
    // data→network, matching the unified lock order across the codebase, so it can't deadlock with
    // the recv-loop handlers. Player action/position handling is untouched (event-driven).
    {
        let data = data.clone();
        let network = network.clone();
        let gamedata = game_data.clone();
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(125));
            interval.tick().await;
            loop {
                interval.tick().await;

                let now = Instant::now();
                let mut pending_aoes: Vec<PendingAoe> = Vec::new();
                {
                    let mut data = data.lock();
                    for instance in data.instances.iter_mut() {
                        if instance.director.is_none() {
                            continue;
                        }

                        encounter_tick(instance, now);
                        pending_aoes.extend(director_tick(
                            network.clone(),
                            gamedata.clone(),
                            instance,
                        ));
                    }
                }
                // Spawn precise AoE timers after releasing the data lock.
                schedule_pending_aoes(data.clone(), network.clone(), pending_aoes);
            }
        });
    }

    while let Some(msg) = recv.recv().await {
        let mut to_remove = Vec::new();

        let mut handled = handle_chat_messages(
            data.clone(),
            network.clone(),
            game_data.clone(),
            lua.clone(),
            &msg,
        );
        handled |= handle_social_messages(data.clone(), network.clone(), &msg);
        handled |= handle_zone_messages(data.clone(), network.clone(), game_data.clone(), &msg);
        handled |= handle_action_messages(
            data.clone(),
            game_data.clone(),
            network.clone(),
            lua.clone(),
            &msg,
        );
        handled |= handle_effect_messages(data.clone(), network.clone(), lua.clone(), &msg);
        handled |= handle_director_messages(data.clone(), &msg);
        handled |= handle_party_messages(data.clone(), network.clone(), &msg);
        handled |= handle_linkshell_messages(network.clone(), &msg);

        if !handled {
            match msg {
                ToServer::NewClient(handle) => {
                    tracing::info!(
                        "New zone client {:?} is connecting with actor id {}",
                        handle.id,
                        handle.actor_id
                    );

                    let mut network = network.lock();
                    let mut party_id = None;
                    let handle_id = handle.id;

                    // Refresh the party member's client id, if applicable.
                    'outer: for (id, party) in &mut network.parties {
                        for member in &mut party.members {
                            if member.actor_id == handle.actor_id {
                                member.zone_client_id = handle.id;
                                party_id = Some(*id);
                                break 'outer;
                            }
                        }
                    }

                    network
                        .clients
                        .insert(handle.id, (handle.clone(), ClientState::default()));

                    if let Some(party_id) = party_id {
                        tracing::info!("{} is rejoining party {}", handle.actor_id, party_id);
                        network.send_to(
                            handle_id,
                            FromServer::RejoinPartyAfterDisconnect(party_id),
                            DestinationNetwork::ZoneClients,
                        );
                    } else {
                        tracing::info!("{} was not in a party before connecting.", handle.actor_id);
                    }
                }
                ToServer::NewChatClient(handle) => {
                    tracing::info!(
                        "New chat client {:?} is connecting with actor id {}",
                        handle.id,
                        handle.actor_id
                    );

                    let mut network = network.lock();

                    // Refresh the party member's client id, if applicable.
                    'outer: for party in &mut network.parties.values_mut() {
                        for member in &mut party.members {
                            if member.actor_id == handle.actor_id {
                                member.chat_client_id = handle.id; // The chat connection doesn't get informed here since it'll happen later.
                                break 'outer;
                            }
                        }
                    }

                    network
                        .chat_clients
                        .insert(handle.id, (handle, ClientState::default()));
                }
                ToServer::ReadySpawnPlayer(
                    from_id,
                    from_actor_id,
                    zone_id,
                    position,
                    rotation,
                    city_state_opening,
                ) => {
                    tracing::info!("Player {from_id:?} is now spawning into {zone_id}....");

                    let mut data = data.lock();
                    let mut network = network.lock();

                    // create a new instance if necessary
                    let instance;
                    {
                        let mut game_data = game_data.lock();
                        instance = data.ensure_exists(zone_id, &mut game_data);
                    }

                    instance.insert_empty_actor(from_actor_id);

                    // TODO: de-duplicate with other ChangeZone call-sites
                    let director_vars = instance
                        .director
                        .as_ref()
                        .map(|director| director.build_var_segment());

                    let exit_position;
                    let exit_rotation;
                    if let Some(city_state) = city_state_opening {
                        // If spawning for the initial opening, we need to spawn them at this pop range *as soon as possible*
                        // The reason being is that this helps loading times and the initial camera rotation.
                        // Doing it in the opening Lua script happens far too late, as EnterTerritoryEvent will only be fired after ZoneInit is sent.
                        if let Some((object, _)) = instance
                            .zone
                            .find_pop_range(determine_initial_pop_range(city_state))
                        {
                            let (_, rotation, translation) =
                                Affine3A::from(object.transform).to_scale_rotation_translation();
                            exit_position = Position(translation);
                            exit_rotation = euler_to_direction(rotation.to_euler(EulerRot::XYZ));
                        } else {
                            exit_position = position;
                            exit_rotation = rotation;
                        }
                    } else {
                        exit_position = position;
                        exit_rotation = rotation;
                    }

                    // tell the client to load into the zone
                    let msg = FromServer::ChangeZone(
                        zone_id,
                        instance.content_finder_condition_id,
                        instance.weather_id,
                        exit_position,
                        exit_rotation,
                        instance.zone.to_lua_zone(instance.weather_id),
                        true, // since this is initial login
                        director_vars,
                    );

                    network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                }
                ToServer::ActorMoved(
                    actor_id,
                    position,
                    rotation,
                    anim_type,
                    anim_state,
                    jump_state,
                    party_id,
                ) => {
                    let mut data = data.lock();

                    if let Some(instance) = data.find_actor_instance_mut(actor_id) {
                        let mut moved = false;
                        if let Some((_, spawn)) = instance
                            .actors
                            .iter_mut()
                            .find(|actor| *actor.0 == actor_id)
                        {
                            let common = spawn.get_common_spawn_mut();
                            moved = common.position != position;
                            common.position = position;
                            common.rotation = rotation;
                        }

                        // Send actor move!
                        {
                            let mut network = network.lock();
                            let msg = FromServer::ActorMove(
                                actor_id, position, rotation, anim_type, anim_state, jump_state,
                            );
                            network.send_in_range_instance(
                                actor_id,
                                instance,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );
                        }

                        if moved {
                            // Check if the actor has any in-progress actions, and cancel them if so.
                            // Slidecast: once a cast is within its final window (SLIDECAST_WINDOW)
                            // it's locked in, so movement no longer interrupts it — the task is left
                            // in the queue and still fires for full effect. Moving any earlier than
                            // that interrupts as usual.
                            let now = Instant::now();
                            for task in instance.find_tasks(actor_id) {
                                if let QueuedTaskData::CastAction { interruptible, .. } = task.data
                                    && interruptible
                                    && task.point.saturating_duration_since(now) > SLIDECAST_WINDOW
                                {
                                    let mut game_data = game_data.lock();
                                    instance.cancel_task(network.clone(), &mut game_data, &task);
                                }
                            }

                            // If the actor moved, and they're in a party, we need to update our information.
                            if let Some(party_id) = party_id {
                                let mut network = network.lock();
                                update_party_position(
                                    &mut network,
                                    &mut data,
                                    party_id,
                                    actor_id,
                                    position,
                                );
                            }
                        }
                    }
                }
                ToServer::PetCommand(_from_id, from_actor_id, command) => {
                    let mut data = data.lock();
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    let mut network = network.lock();
                    if !summoner::apply_pet_command(&mut network, instance, from_actor_id, command)
                    {
                        tracing::debug!(
                            "Pet command {:?} ignored for actor {}",
                            command,
                            from_actor_id
                        );
                    }
                }
                ToServer::ClientTrigger(from_id, from_actor_id, trigger) => {
                    match &trigger.trigger {
                        ClientTriggerCommand::TeleportQuery { aetheryte_id, .. } => {
                            let msg =
                                FromServer::ActorControlSelf(ActorControlCategory::TeleportStart {
                                    insufficient_gil: 0,
                                    aetheryte_id: *aetheryte_id,
                                });

                            {
                                let mut network = network.lock();
                                network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                            }

                            let mut data = data.lock();
                            if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                && let Some(actor) = instance.find_actor_mut(from_actor_id)
                            {
                                match actor {
                                    NetworkedActor::Player { teleport_query, .. } => {
                                        teleport_query.aetheryte_id = *aetheryte_id as u16
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        }
                        ClientTriggerCommand::WalkInTriggerFinished { .. } => {
                            // This is where we finally release the client after the walk-in trigger.
                            let msg = FromServer::Conditions(Conditions::default());

                            let mut network = network.lock();
                            network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                        }
                        ClientTriggerCommand::SummonMinion { minion_id } => {
                            let msg = FromServer::ActorSummonsMinion(*minion_id);

                            let mut network = network.lock();
                            network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                        }
                        ClientTriggerCommand::DespawnMinion { .. } => {
                            let msg = FromServer::ActorDespawnsMinion();

                            let mut network = network.lock();
                            network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                        }
                        ClientTriggerCommand::PetAction { action_id } => {
                            let pet_command = match *action_id {
                                1 => Some(PetCommand::Recall),
                                2 => Some(PetCommand::Follow),
                                4 => Some(PetCommand::Stay),
                                _ => None,
                            };

                            if let Some(pet_command) = pet_command {
                                let mut data = data.lock();
                                let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                else {
                                    continue;
                                };

                                let mut network = network.lock();
                                if !summoner::apply_pet_command(
                                    &mut network,
                                    instance,
                                    from_actor_id,
                                    pet_command,
                                ) {
                                    tracing::debug!(
                                        "Pet action {} ignored for actor {}",
                                        action_id,
                                        from_actor_id
                                    );
                                }
                            } else if *action_id != 3 {
                                tracing::debug!("Client executed unknown pet action {}", action_id);
                            }
                        }
                        ClientTriggerCommand::SetTarget {
                            actor_id,
                            actor_type,
                        } => {
                            // For whatever reason these don't match what the server has to send back, so they cannot be directly reused.
                            let actor_type = match *actor_type {
                                0 => ObjectTypeKind::None,
                                1 => ObjectTypeKind::EObjOrNpc,
                                2 => ObjectTypeKind::Minion,
                                _ => {
                                    tracing::warn!(
                                        "SetTarget: Unknown actor target type {}! Defaulting to None!",
                                        *actor_type
                                    );
                                    ObjectTypeKind::None
                                }
                            };

                            let target = ObjectTypeId {
                                object_id: *actor_id,
                                object_type: actor_type,
                            };
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                target,
                                ActorControlCategory::SetTarget {},
                            );

                            let mut data = data.lock();
                            if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                && let Some(actor) = instance.find_actor_mut(from_actor_id)
                            {
                                actor.get_common_spawn_mut().target_id = target;
                            }
                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );
                        }
                        ClientTriggerCommand::SetSoftTarget {} => {
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                trigger.target.unwrap_or_default(),
                                ActorControlCategory::SetSoftTarget {},
                            );

                            let data = data.lock();
                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );
                        }
                        ClientTriggerCommand::ChangePose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControlCategory::Pose {
                                    unk1: *unk1,
                                    pose: *pose,
                                },
                            );

                            let mut data = data.lock();
                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );

                            // Update data for respawns
                            {
                                if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                    && let Some(actor) = instance.find_actor_mut(from_actor_id)
                                    && let NetworkedActor::Player { spawn, .. } = actor
                                {
                                    spawn.pose = *pose as u8;
                                }
                            }
                        }
                        ClientTriggerCommand::ReapplyPose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControlCategory::Pose {
                                    unk1: *unk1,
                                    pose: *pose,
                                },
                            );

                            let mut data = data.lock();
                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );

                            // Update data for respawns
                            {
                                if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                    && let Some(actor) = instance.find_actor_mut(from_actor_id)
                                    && let NetworkedActor::Player { spawn, .. } = actor
                                {
                                    spawn.pose = *pose as u8;
                                }
                            }
                        }
                        ClientTriggerCommand::ExitIdlePosture {} => {}
                        ClientTriggerCommand::Emote { emote, hide_text } => {
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                trigger.target.unwrap(),
                                ActorControlCategory::Emote {
                                    emote: *emote,
                                    hide_text: *hide_text,
                                },
                            );

                            let mut data = data.lock();
                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );

                            // setup persistence if looping
                            let emote_mode;
                            {
                                let mut game_data = game_data.lock();
                                emote_mode = game_data.get_emote_mode(*emote);
                            }

                            if let Some(mode) = emote_mode
                                && let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                            {
                                set_character_mode(
                                    instance,
                                    &mut network,
                                    from_actor_id,
                                    CharacterMode::EmoteLoop,
                                    mode,
                                );
                            }
                        }
                        ClientTriggerCommand::ToggleWeapon { shown, immediately } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControlCategory::ToggleWeapon {
                                    shown: *shown,
                                    immediately: *immediately,
                                },
                            );

                            let data = data.lock();
                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );
                        }
                        ClientTriggerCommand::ManuallyRemoveEffect {
                            effect_id,
                            source_actor_id,
                            effect_param,
                        } => {
                            // If there is a scheduled task to remove it, cancel it!
                            // This is harmless to keep, but it's better not to clog the queue.
                            {
                                let mut data = data.lock();
                                if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                {
                                    for task in instance.find_tasks(from_actor_id) {
                                        let target_effect_id = *effect_id as u16;
                                        let target_actor_id = *source_actor_id;
                                        // NOTE: I intentionally don't match on effect_param, I don't think that's truly reflective from CT?
                                        if let QueuedTaskData::LoseStatusEffect {
                                            effect_id,
                                            effect_source_actor_id,
                                            ..
                                        } = task.data
                                            && effect_id == target_effect_id
                                            && effect_source_actor_id == target_actor_id
                                        {
                                            instance.retain_tasks(|queued| queued != &task);
                                        }
                                    }
                                }
                            }

                            remove_effect(
                                network.clone(),
                                data.clone(),
                                lua.clone(),
                                from_id,
                                from_actor_id,
                                *effect_id as u16,
                                *effect_param as u16,
                                *source_actor_id,
                            );
                        }
                        ClientTriggerCommand::SetDistanceRange { range } => {
                            let mut data = data.lock();
                            if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                && let Some(actor) = instance.find_actor_mut(from_actor_id)
                            {
                                match actor {
                                    NetworkedActor::Player { distance_range, .. } => {
                                        *distance_range = *range;
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        }
                        ClientTriggerCommand::GimmickJumpLanded { .. } => {
                            let mut data = data.lock();
                            if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                && let Some(actor) = instance.find_actor_mut(from_actor_id)
                            {
                                match actor {
                                    NetworkedActor::Player {
                                        executing_gimmick_jump,
                                        ..
                                    } => {
                                        *executing_gimmick_jump = false;
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        }
                        ClientTriggerCommand::SetTitle { title_id } => {
                            let mut data = data.lock();
                            if let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                && let Some(actor) = instance.find_actor_mut(from_actor_id)
                            {
                                match actor {
                                    NetworkedActor::Player { spawn, .. } => {
                                        spawn.title_id = *title_id as u16;
                                    }
                                    _ => unreachable!(),
                                }
                            }

                            // inform other players
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControlCategory::SetTitle {
                                    title_id: *title_id,
                                },
                            );

                            let mut network = network.lock();
                            network.send_in_range(
                                from_actor_id,
                                &data,
                                msg,
                                DestinationNetwork::ZoneClients,
                            );
                        }
                        ClientTriggerCommand::PlaceWaymark { id, pos } => {
                            let data = data.lock();
                            let mut network = network.lock();

                            update_party_waymark(
                                &mut network,
                                &data,
                                from_actor_id,
                                *id,
                                Some(*pos),
                            );
                        }
                        ClientTriggerCommand::ClearWaymark { id } => {
                            let data = data.lock();
                            let mut network = network.lock();

                            update_party_waymark(&mut network, &data, from_actor_id, *id, None);
                        }
                        ClientTriggerCommand::ClearAllWaymarks {} => {
                            let data = data.lock();
                            let mut network = network.lock();

                            // Clearing all waymarks is equivalent to sending a completely blank preset.
                            update_party_waymarks(
                                &mut network,
                                &data,
                                from_actor_id,
                                WaymarkPreset::default(),
                            );
                        }
                        ClientTriggerCommand::ToggleSign { sign_id, .. } => {
                            let Some(target_actor) = trigger.target else {
                                continue;
                            };

                            let mut network = network.lock();
                            let msg = FromServer::TargetSignToggled(
                                *sign_id,
                                from_actor_id,
                                target_actor,
                            );

                            network.send_to_party_or_self(from_actor_id, msg);

                            // If we're in a party, keep track of the actor that was just marked.
                            if let Some(party_id) =
                                get_party_id_from_actor_id(&network, from_actor_id)
                            {
                                let party = network.parties.get_mut(&party_id).unwrap();
                                let sign_id = *sign_id as usize;
                                if sign_id < NUM_TARGET_SIGNS {
                                    party.target_signs[sign_id] = target_actor;
                                } else {
                                    tracing::error!(
                                        "Client tried to assign target sign id {sign_id}, but there are only {NUM_TARGET_SIGNS} currently known. Update the constant if necessary!"
                                    );
                                }
                            }
                        }
                        ClientTriggerCommand::RequestDuel { actor_id } => {
                            let mut data = data.lock();
                            let mut network = network.lock();
                            network.send_to_by_actor_id(
                                from_actor_id,
                                FromServer::ActorControlSelf(ActorControlCategory::SetPvPState {
                                    state: 2,
                                }),
                                DestinationNetwork::ZoneClients,
                            );

                            let account_id;
                            {
                                let Some((handle, _)) = network.get_by_actor_mut(from_actor_id)
                                else {
                                    continue;
                                };
                                account_id = handle.account_id;
                            }

                            let opponent_content_id;
                            let opponent_object_id;
                            let opponent_name;
                            {
                                let Some((handle, _)) = network.get_by_actor_mut(*actor_id) else {
                                    continue;
                                };
                                opponent_content_id = handle.content_id;
                                opponent_object_id = *actor_id;

                                let Some(instance) = data.find_actor_instance(*actor_id) else {
                                    continue;
                                };

                                let Some(actor) = instance.find_actor(*actor_id) else {
                                    continue;
                                };

                                opponent_name = actor.get_common_spawn().name.clone();
                            }

                            let config = get_config();

                            let ipc =
                                ServerZoneIpcSegment::new(ServerZoneIpcData::DuelInformation {
                                    account_id,
                                    opponent_content_id,
                                    opponent_object_id,
                                    world_id: config.world.world_id,
                                    unk1: 7957,
                                    unk2: 1,
                                    opponent_name,
                                });
                            network.send_to_by_actor_id(
                                from_actor_id,
                                FromServer::PacketSegment(ipc, from_actor_id),
                                DestinationNetwork::ZoneClients,
                            );

                            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                                continue;
                            };

                            // Update our player
                            {
                                let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                                    continue;
                                };

                                match actor {
                                    NetworkedActor::Player {
                                        dueling_opponent_id,
                                        ..
                                    } => *dueling_opponent_id = *actor_id,
                                    _ => unreachable!(),
                                };
                            }

                            // Update the opponent
                            {
                                let Some(actor) = instance.find_actor_mut(*actor_id) else {
                                    continue;
                                };

                                match actor {
                                    NetworkedActor::Player {
                                        dueling_opponent_id,
                                        ..
                                    } => *dueling_opponent_id = from_actor_id,
                                    _ => unreachable!(),
                                };
                            }
                        }
                        ClientTriggerCommand::RequestDuelResponse { cancel } => {
                            if *cancel {
                                let mut data = data.lock();
                                let Some(instance) = data.find_actor_instance_mut(from_actor_id)
                                else {
                                    continue;
                                };

                                let other_actor_id;

                                // Update our player
                                {
                                    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                                        continue;
                                    };

                                    match actor {
                                        NetworkedActor::Player {
                                            dueling_opponent_id,
                                            ..
                                        } => {
                                            other_actor_id = *dueling_opponent_id;
                                            *dueling_opponent_id = ObjectId::default();
                                        }
                                        _ => unreachable!(),
                                    };
                                }

                                // Update the opponent
                                {
                                    let Some(actor) = instance.find_actor_mut(other_actor_id)
                                    else {
                                        continue;
                                    };

                                    match actor {
                                        NetworkedActor::Player {
                                            dueling_opponent_id,
                                            ..
                                        } => *dueling_opponent_id = ObjectId::default(),
                                        _ => unreachable!(),
                                    };
                                }
                            } else {
                                // If not cancelling, then we need to send a confirmation to the opponent...
                                let data = data.lock();

                                let mut network = network.lock();
                                let Some(instance) = data.find_actor_instance(from_actor_id) else {
                                    continue;
                                };

                                let Some(actor) = instance.find_actor(from_actor_id) else {
                                    continue;
                                };

                                let dueling_opponent_id = match actor {
                                    NetworkedActor::Player {
                                        dueling_opponent_id,
                                        ..
                                    } => dueling_opponent_id,
                                    _ => unreachable!(),
                                };

                                let account_id;
                                {
                                    let Some((handle, _)) =
                                        network.get_by_actor_mut(*dueling_opponent_id)
                                    else {
                                        continue;
                                    };
                                    account_id = handle.account_id;
                                }

                                let opponent_content_id;
                                let opponent_object_id;
                                let opponent_name;
                                {
                                    let Some((handle, _)) = network.get_by_actor_mut(from_actor_id)
                                    else {
                                        continue;
                                    };
                                    opponent_content_id = handle.content_id;
                                    opponent_object_id = from_actor_id;

                                    opponent_name = actor.get_common_spawn().name.clone();
                                }

                                let config = get_config();

                                let ipc =
                                    ServerZoneIpcSegment::new(ServerZoneIpcData::DuelInformation {
                                        account_id,
                                        opponent_content_id,
                                        opponent_object_id,
                                        world_id: config.world.world_id,
                                        unk1: 7957,
                                        unk2: 0,
                                        opponent_name,
                                    });
                                network.send_to_by_actor_id(
                                    *dueling_opponent_id,
                                    FromServer::ActorControlSelf(
                                        ActorControlCategory::SetPvPState { state: 3 },
                                    ),
                                    DestinationNetwork::ZoneClients,
                                );
                                network.send_to_by_actor_id(
                                    *dueling_opponent_id,
                                    FromServer::ActorControlSelf(
                                        ActorControlCategory::SetPvPState { state: 4 },
                                    ),
                                    DestinationNetwork::ZoneClients,
                                );
                                network.send_to_by_actor_id(
                                    *dueling_opponent_id,
                                    FromServer::PacketSegment(ipc, *dueling_opponent_id),
                                    DestinationNetwork::ZoneClients,
                                );
                            }
                        }
                        ClientTriggerCommand::DuelDecision { decline } => {
                            // TODO: what happens if they do decline?
                            if !*decline {
                                let data = data.lock();
                                let Some(instance) = data.find_actor_instance(from_actor_id) else {
                                    continue;
                                };

                                let Some(actor) = instance.find_actor(from_actor_id) else {
                                    continue;
                                };

                                let dueling_opponent_id = match actor {
                                    NetworkedActor::Player {
                                        dueling_opponent_id,
                                        ..
                                    } => dueling_opponent_id,
                                    _ => unreachable!(),
                                };

                                tracing::info!(
                                    "Duel has begun between {from_actor_id} and {dueling_opponent_id}!"
                                );

                                let mut network = network.lock();
                                // unknown
                                network.send_ac_in_range_inclusive(
                                    &data,
                                    *dueling_opponent_id,
                                    ActorControlCategory::SetPvPState { state: 5 },
                                );
                                network.send_ac_in_range_inclusive(
                                    &data,
                                    from_actor_id,
                                    ActorControlCategory::SetPvPState { state: 5 },
                                );

                                // unknown ver. 2
                                network.send_ac_in_range_inclusive(
                                    &data,
                                    *dueling_opponent_id,
                                    ActorControlCategory::SetPvPState { state: 6 },
                                );
                                network.send_ac_in_range_inclusive(
                                    &data,
                                    from_actor_id,
                                    ActorControlCategory::SetPvPState { state: 6 },
                                );

                                // begin countdown
                                network.send_to_by_actor_id(
                                    *dueling_opponent_id,
                                    FromServer::ActorControlSelf(
                                        ActorControlCategory::StartDuelCountdown {
                                            opponent_id: from_actor_id,
                                        },
                                    ),
                                    DestinationNetwork::ZoneClients,
                                );
                                network.send_to_by_actor_id(
                                    from_actor_id,
                                    FromServer::ActorControlSelf(
                                        ActorControlCategory::StartDuelCountdown {
                                            opponent_id: *dueling_opponent_id,
                                        },
                                    ),
                                    DestinationNetwork::ZoneClients,
                                );

                                // BATTLE
                                network.send_to_by_actor_id(
                                    *dueling_opponent_id,
                                    FromServer::ActorControlSelf(ActorControlCategory::SetBattle {
                                        battle: true,
                                    }),
                                    DestinationNetwork::ZoneClients,
                                );
                                network.send_to_by_actor_id(
                                    from_actor_id,
                                    FromServer::ActorControlSelf(ActorControlCategory::SetBattle {
                                        battle: true,
                                    }),
                                    DestinationNetwork::ZoneClients,
                                );
                            }
                        }
                        ClientTriggerCommand::ResetStrikingDummy { id } => {
                            let mut data = data.lock();
                            let Some(instance) = data.find_actor_instance_mut(*id) else {
                                continue;
                            };

                            let hated_players = instance
                                .find_actor(*id)
                                .and_then(|actor| actor.npc_hate_list())
                                .map(|hate_list| hate_list.keys().copied().collect::<Vec<_>>())
                                .unwrap_or_default();

                            let Some(actor) = instance.find_actor_mut(*id) else {
                                continue;
                            };

                            let NetworkedActor::Npc {
                                state,
                                navmesh_path,
                                navmesh_path_lerp,
                                navmesh_target: current_target,
                                hate_list,
                                spawn,
                                ..
                            } = actor
                            else {
                                continue;
                            };

                            *state = NpcState::Wander;
                            navmesh_path.clear();
                            *navmesh_path_lerp = 0.0;
                            *current_target = None;
                            hate_list.clear();
                            spawn.common.target_id = ObjectTypeId::default();
                            spawn.common.health_points = spawn.common.max_health_points;
                            let reset_hp = spawn.common.health_points;
                            let reset_mp = spawn.common.resource_points;

                            for player_id in hated_players {
                                if let Some(NetworkedActor::Player { hated_by, .. }) =
                                    instance.find_actor_mut(player_id)
                                {
                                    hated_by.remove(id);
                                }
                            }

                            // TODO: throw this into a generic de-aggro thing eventually
                            let mut network = network.lock();
                            network.send_in_range(
                                *id,
                                &data,
                                FromServer::ActorControlTarget(
                                    *id,
                                    ObjectTypeId::default(),
                                    ActorControlCategory::SetTarget {},
                                ),
                                DestinationNetwork::ZoneClients,
                            );
                            network.send_ac_in_range(
                                &data,
                                *id,
                                ActorControlCategory::SetBattle { battle: false },
                            );
                            // Drop the combat tag (claim) so the nameplate returns to neutral.
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FirstAttack {
                                unk1: 1,
                                unk2: 0,
                                combat_tagger: ObjectId::default(),
                                unk3: 0,
                            });
                            network.send_in_range(
                                *id,
                                &data,
                                FromServer::PacketSegment(ipc, *id),
                                DestinationNetwork::ZoneClients,
                            );
                            // Broadcast the refilled HP. The reset restores HP server-side, but
                            // without this the client only sees full HP on the *next* attack.
                            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp {
                                hp: reset_hp,
                                mp: reset_mp,
                                unk: 0,
                            });
                            network.send_in_range(
                                *id,
                                &data,
                                FromServer::PacketSegment(ipc, *id),
                                DestinationNetwork::ZoneClients,
                            );
                        }
                        ClientTriggerCommand::EmoteInterrupted {} => {
                            let data = data.lock();
                            let mut network = network.lock();
                            network.send_ac_in_range_inclusive(
                                &data,
                                from_actor_id,
                                ActorControlCategory::InterruptEmote {},
                            );
                        }
                        ClientTriggerCommand::LoopingEmoteInterrupted {} => {
                            let mut data = data.lock();
                            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                                continue;
                            };

                            let mut network = network.lock();
                            set_character_mode(
                                instance,
                                &mut network,
                                from_actor_id,
                                CharacterMode::Normal,
                                0,
                            );
                        }
                        _ => tracing::warn!("Server doesn't know what to do with {:#?}", trigger),
                    }
                }
                ToServer::Config(_from_id, from_actor_id, config) => {
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                        continue;
                    };

                    let NetworkedActor::Player { spawn, .. } = actor else {
                        continue;
                    };

                    // update their stored state so it's correctly sent on new spawns
                    spawn.common.display_flags = config.display_flag.into();

                    let mut network = network.lock();
                    let msg = FromServer::UpdateConfig(from_actor_id, config.clone());

                    network.send_in_range_inclusive_instance(
                        from_actor_id,
                        instance,
                        msg,
                        DestinationNetwork::ZoneClients,
                    );
                }
                ToServer::Equip(
                    from_actor_id,
                    main_weapon_id,
                    sub_weapon_id,
                    model_ids,
                    second_model_stain_ids,
                ) => {
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                        continue;
                    };

                    let NetworkedActor::Player { spawn, .. } = actor else {
                        continue;
                    };

                    // update their stored state so it's correctly sent on new spawns
                    spawn.common.main_weapon_model = main_weapon_id;
                    spawn.common.sec_weapon_model = sub_weapon_id;
                    spawn.common.models = model_ids;
                    spawn.common.second_model_stain_ids = second_model_stain_ids;

                    // Inform all clients about their new equipped model ids
                    let msg = FromServer::ActorEquip(
                        from_actor_id,
                        main_weapon_id,
                        sub_weapon_id,
                        model_ids,
                        second_model_stain_ids,
                    );

                    let mut network = network.lock();
                    network.send_in_range_inclusive_instance(
                        from_actor_id,
                        instance,
                        msg,
                        DestinationNetwork::ZoneClients,
                    );
                }
                ToServer::Disconnected(from_id, from_actor_id) => {
                    let mut network = network.lock();
                    network.to_remove.push(from_id);

                    // Tell our sibling chat connection that it's time to go too.
                    network.send_to_by_actor_id(
                        from_actor_id,
                        FromServer::ChatDisconnected(),
                        DestinationNetwork::ChatClients,
                    );

                    // Remove them from any relevant linkshells.
                    network
                        .linkshells
                        .iter_mut()
                        .for_each(|(_, linkshell)| linkshell.retain(|m| *m != from_actor_id));
                    // Clean up any empty linkshells.
                    network.linkshells.retain(|_, shell| !shell.is_empty());
                }
                ToServer::ActorSummonsMinion(from_actor_id, minion_id) => {
                    let mut data = data.lock();
                    let mut network = network.lock();

                    set_player_minion(&mut data, &mut network, minion_id, from_actor_id);
                }
                ToServer::ActorDespawnsMinion(from_actor_id) => {
                    let mut data = data.lock();
                    let mut network = network.lock();

                    set_player_minion(&mut data, &mut network, 0, from_actor_id);
                }
                ToServer::ChatDisconnected(from_id) => {
                    let mut network = network.lock();
                    network.to_remove_chat.push(from_id);
                }
                ToServer::JoinContent(from_id, from_actor_id, content_id) => {
                    // For now, just send them to do the zone if they do anything
                    let zone_id;
                    {
                        let mut game_data = game_data.lock();
                        zone_id = game_data.find_zone_for_content(content_id);
                    }

                    let mut actor_ids = Vec::new();

                    // Send all party members to this instanced content
                    let mut data = data.lock();
                    let mut network = network.lock();
                    if let Some(party_id) = get_party_id_from_actor_id(&network, from_actor_id) {
                        if let Some(party) = network.parties.get(&party_id) {
                            for member in &party.members {
                                if member.is_valid() && member.is_online() {
                                    actor_ids.push((member.zone_client_id, member.actor_id));
                                }
                            }
                        }
                    } else {
                        actor_ids.push((from_id, from_actor_id));
                    }

                    if let Some(zone_id) = zone_id {
                        // Carry each player's combat state (job gauge, cooldowns, summoned-pet flag)
                        // into the duty. Without this the destination instance gets a brand-new
                        // actor with default state, so a summoner's pet would fail to re-spawn while
                        // the client still shows it — desyncing the pet UI.
                        let mut carried_states = Vec::new();
                        for (_, actor_id) in &actor_ids {
                            // inform the players in this zone that this actor left
                            if let Some(current_instance) = data.find_actor_instance_mut(*actor_id)
                            {
                                let state = take_combat_state_and_despawn_pets(
                                    current_instance,
                                    &mut network,
                                    *actor_id,
                                );
                                carried_states.push((*actor_id, state));
                                network.remove_actor(current_instance, *actor_id);
                            }
                        }

                        // then find or create a new instance with the zone id and content finder condition
                        let mut game_data = game_data.lock();
                        if let Some(target_instance) =
                            data.create_instance_for_content(zone_id, content_id, &mut game_data)
                        {
                            for (client_id, actor_id) in &actor_ids {
                                target_instance.insert_empty_actor(*actor_id);

                                let carried = carried_states
                                    .iter()
                                    .find(|(id, _)| id == actor_id)
                                    .and_then(|(_, state)| state.clone());
                                restore_carried_combat_state(target_instance, *actor_id, carried);

                                change_zone_warp_to_entrance(
                                    &mut network,
                                    target_instance,
                                    true, // TODO: this shouldn't be hardcoded
                                    *client_id,
                                );
                            }
                        } else {
                            tracing::warn!("Failed to create a new instance for content?!");
                        }
                    } else {
                        tracing::warn!("Failed to find zone id for content?!");
                    }
                }
                ToServer::LeaveContent(
                    from_client_id,
                    from_actor_id,
                    old_zone_id,
                    old_position,
                    old_rotation,
                ) => {
                    let mut data = data.lock();
                    let mut network = network.lock();

                    // Carry the player's combat state (job gauge, cooldowns, summoned-pet flag) back
                    // out of the duty so the summoner's pet re-spawns in the overworld instead of
                    // leaving the client's pet UI desynced.
                    let mut carried_combat_state = None;

                    // Inform the players in this zone that this actor left
                    if let Some(current_instance) = data.find_actor_instance_mut(from_actor_id) {
                        carried_combat_state = take_combat_state_and_despawn_pets(
                            current_instance,
                            &mut network,
                            from_actor_id,
                        );
                        network.remove_actor(current_instance, from_actor_id);
                    }

                    // create a new instance if necessary
                    let instance;
                    {
                        let mut game_data = game_data.lock();
                        instance = data.ensure_exists(old_zone_id, &mut game_data);
                    }

                    instance.insert_empty_actor(from_actor_id);
                    restore_carried_combat_state(instance, from_actor_id, carried_combat_state);

                    let director_vars = instance
                        .director
                        .as_ref()
                        .map(|director| director.build_var_segment());

                    // tell the client to load into the zone
                    let msg = FromServer::ChangeZone(
                        old_zone_id,
                        instance.content_finder_condition_id,
                        instance.weather_id,
                        old_position,
                        old_rotation,
                        instance.zone.to_lua_zone(instance.weather_id),
                        false,
                        director_vars,
                    );
                    network.send_to(from_client_id, msg, DestinationNetwork::ZoneClients);
                }
                ToServer::UpdateConditions(from_actor_id, new_conditions) => {
                    // update their stored state
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                        continue;
                    };

                    let NetworkedActor::Player { conditions, .. } = actor else {
                        continue;
                    };

                    *conditions = new_conditions;
                }
                ToServer::CommenceDuty(from_actor_id) => {
                    let mut data = data.lock();
                    let entrance_actor_id;
                    let state = EventState::UNK1 | EventState::UNK2 | EventState::UNK3;

                    {
                        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                            continue;
                        };

                        // Find the spawned entrance circle
                        let Some(actor_id) = instance.find_entrance_circle() else {
                            tracing::warn!("Failed to find entrance circle, it won't despawn!");
                            continue;
                        };
                        entrance_actor_id = actor_id;

                        // Update invisibility flags for next spawn
                        if let Some(NetworkedActor::Object { object, .. }) =
                            instance.find_actor_mut(entrance_actor_id)
                        {
                            object.event_state = state;
                            object.targetable_status = 1;
                        }
                    }

                    // Make the entrance circle invisible.
                    let msg = FromServer::ActorControl(
                        entrance_actor_id,
                        ActorControlCategory::SetEventState { state },
                    );

                    let mut network = network.lock();
                    network.send_in_range(
                        entrance_actor_id,
                        &data,
                        msg,
                        DestinationNetwork::ZoneClients,
                    );
                }
                ToServer::Kill(_from_id, from_actor_id) => {
                    let mut data = data.lock();
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };
                    kill_actor(network.clone(), instance, from_actor_id)
                }
                ToServer::SetHP(_from_id, from_actor_id, hp) => {
                    let mut data = data.lock();
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                        continue;
                    };

                    actor.get_common_spawn_mut().health_points = hp;

                    update_actor_hp_mp(network.clone(), instance, from_actor_id);
                }
                ToServer::SetMP(_from_id, from_actor_id, mp) => {
                    let mut data = data.lock();
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                        continue;
                    };

                    actor.get_common_spawn_mut().resource_points = mp;

                    update_actor_hp_mp(network.clone(), instance, from_actor_id);
                }
                ToServer::SetNewStatValues(from_actor_id, level, class_job, new_parameters) => {
                    // Update internal data model
                    {
                        let mut data = data.lock();
                        let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                            continue;
                        };

                        let Some(actor) = instance.find_actor_mut(from_actor_id) else {
                            continue;
                        };

                        actor.get_common_spawn_mut().level = level;
                        actor.get_common_spawn_mut().max_health_points = new_parameters.hp;
                        actor.get_common_spawn_mut().max_resource_points = new_parameters.mp as u16;
                        actor.get_common_spawn_mut().class_job = class_job;

                        if let NetworkedActor::Player { parameters, .. } = actor {
                            *parameters = new_parameters.clone();
                        }

                        // The only way the game can reliably set these stats is via StatusEffectList (REALLY)
                        send_effects_list(network.clone(), instance, from_actor_id);
                    }
                }
                ToServer::Fish(from_client_id, from_actor_id) => {
                    let mut data = data.lock();
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    instance.insert_task(
                        from_client_id,
                        from_actor_id,
                        Duration::from_secs(2),
                        QueuedTaskData::FishBite {},
                    );
                }
                ToServer::ReloadScripts => {
                    let mut lua = lua.lock();
                    if let Err(err) = lua.init(game_data.clone()) {
                        tracing::warn!("Failed to load Init.lua: {:?}", err);
                    }
                }
                ToServer::Dismounted(from_actor_id, party_id) => {
                    let mut data = data.lock();
                    let mut network = network.lock();

                    let mut ids_to_dismount = Vec::new();
                    ids_to_dismount.push(from_actor_id);

                    if let Some(party_id) = party_id
                        && let Some(party) = network.parties.get_mut(&party_id)
                    {
                        for member in &mut party.members {
                            // If the dismounting player is this member's driver, this member needs to be dismounted too.
                            if member.pillion_driver_id == from_actor_id {
                                ids_to_dismount.push(member.actor_id);
                            }
                            // If this member is dismounting manually while riding pillion, there is no longer a driver for them.
                            if member.actor_id == from_actor_id
                                && member.pillion_driver_id != ObjectId::default()
                            {
                                member.pillion_driver_id = ObjectId::default();
                            }
                        }
                    }

                    for id in ids_to_dismount {
                        let Some(instance) = data.find_actor_instance_mut(id) else {
                            continue;
                        };

                        if let Some(actor) = instance.find_actor_mut(id) {
                            let common = actor.get_common_spawn_mut();
                            common.current_mount = 0;
                            common.mode = CharacterMode::Normal;
                            common.mode_arg = 0;
                        }

                        let msg = FromServer::ActorDismounted(id);
                        network.send_in_range_inclusive_instance(
                            id,
                            instance,
                            msg,
                            DestinationNetwork::ZoneClients,
                        );
                        summoner::sync_pet_after_dismount(&mut network, instance, id);
                    }
                }
                ToServer::SetOnlineStatus(from_actor_id, online_status) => {
                    let data = data.lock();
                    let mut network = network.lock();
                    network.send_ac_in_range_inclusive(
                        &data,
                        from_actor_id,
                        ActorControlCategory::SetStatusIcon {
                            icon: online_status,
                        },
                    );
                }
                ToServer::SetCharacterMode(from_actor_id, mode, arg) => {
                    // ACS is sent by the ZoneConnection
                    let mut data = data.lock();
                    let mut network = network.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    set_character_mode(instance, &mut network, from_actor_id, mode, arg);
                }
                ToServer::BroadcastActorControl(from_actor_id, actor_control) => {
                    let data = data.lock();
                    let mut network = network.lock();
                    network.send_ac_in_range(&data, from_actor_id, actor_control);
                }
                ToServer::RemoveCooldowns(actor_id) => {
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(actor_id) else {
                        continue;
                    };

                    let Some(actor) = instance.find_actor_mut(actor_id) else {
                        continue;
                    };

                    let NetworkedActor::Player {
                        remove_cooldowns, ..
                    } = actor
                    else {
                        continue;
                    };

                    *remove_cooldowns = true;
                }
                ToServer::Jump(from_id, name) => {
                    let mut data = data.lock();
                    let mut network = network.lock();
                    let mut game_data = game_data.lock();

                    let to_actor_id = data.find_actor_by_name(&name);

                    change_zone_to_player(
                        &mut network,
                        &mut data,
                        &mut game_data,
                        from_id,
                        to_actor_id,
                    );
                }
                ToServer::Call(from_actor_id, name) => {
                    let mut data = data.lock();
                    let mut network = network.lock();
                    let mut game_data = game_data.lock();

                    let actor_id = data.find_actor_by_name(&name);
                    if let Some(client_id) = network.find_by_actor(actor_id) {
                        change_zone_to_player(
                            &mut network,
                            &mut data,
                            &mut game_data,
                            client_id,
                            from_actor_id,
                        );
                    }
                }
                ToServer::SpawnLayoutNpc(from_actor_id, layout_id) => {
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        continue;
                    };

                    if let Some(mut npc) = instance.zone.get_battle_npc(layout_id) {
                        npc.common.handler_id = HandlerId::new(HandlerType::GoldSaucer, 1319); // TODO: hardcoded to gold saucer for now
                        instance.insert_npc(ObjectId(fastrand::u32(..)), npc);
                    } else {
                        tracing::warn!(
                            "Failed to find npc {layout_id} for SpawnLayoutNpc, it won't spawn!"
                        );
                    }
                }
                ToServer::FatalError(err) => return Err(err),
                _ => {
                    tracing::error!("Received a ToServer message we don't handle yet: {msg:#?}");
                }
            }
        }

        // Remove any clients that errored out
        {
            let mut data = data.lock();
            let mut network = network.lock();

            network.to_remove.append(&mut to_remove);

            for remove_id in network.to_remove.clone() {
                // remove any actors they had
                let mut actor_id = None;
                for (id, (handle, _)) in &mut network.clients {
                    if *id == remove_id {
                        actor_id = Some(handle.actor_id);
                    }
                }

                if let Some(actor_id) = actor_id {
                    // remove them from the instance
                    if let Some(current_instance) = data.find_actor_instance_mut(actor_id) {
                        network.remove_actor(current_instance, actor_id);
                    }
                }

                network.clients.remove(&remove_id);
            }

            for remove_id in network.to_remove_chat.clone() {
                network.chat_clients.remove(&remove_id);
            }
        }

        // Commit parties back to database as necessary
        {
            let mut network = network.lock();

            // This may seem weird, but currently only the database connection exists on the ZoneConnection side. So we hijack the first client to do our dirty work.
            if network.commit_parties && !network.clients.is_empty() {
                let parties = network.parties.clone();
                let client_id = network.clients.keys().copied().collect::<Vec<ClientId>>()[0];
                network.send_to(
                    client_id,
                    FromServer::CommitParties(parties),
                    DestinationNetwork::ZoneClients,
                );
                network.commit_parties = false;
            }
        }
    }
    Ok(())
}
