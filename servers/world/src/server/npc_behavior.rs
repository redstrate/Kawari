use std::{
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    sync::Arc,
    time::Duration,
};

use glam::Vec3;
use kawari::{
    common::{
        ENEMY_AUTO_ATTACK_RATE, JumpState, MINIMUM_PATHFINDING_DISTANCE, MoveAnimationState,
        MoveAnimationType, ObjectId, ObjectTypeId, ObjectTypeKind, Position,
        STRIKING_DUMMY_NAME_ID, TimepointData,
    },
    ipc::zone::{
        ActionRequest, ActionType, ActorControlCategory, CharacterDataFlag, ServerZoneIpcData,
        ServerZoneIpcSegment, SpawnNpc,
    },
};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData,
    lua::KawariLua,
    server::{
        action::execute_enemy_action,
        actor::{NetworkedActor, NpcState},
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
        set_shared_group_timeline_state,
    },
};

/// How far a hating NPC can be dragged from where it spawned before it gives up, returns
/// home, and resets. Kept larger than the aggro sensing range so a reset NPC doesn't instantly
/// re-aggro the same target.
const ENEMY_LEASH_RANGE: f32 = 30.0;

/// How far a pet may drift from its owner before it repositions. Used for the no-navmesh follow,
/// where there's no pathfinding to walk it back smoothly, so it snaps closer instead.
const PET_FOLLOW_DISTANCE: f32 = 4.0;
/// Where the pet comes to rest relative to its owner when it repositions.
const PET_FOLLOW_STOP: f32 = 2.0;

/// Updates NPCs in this instance.
pub fn npc_behavior(
    network: Arc<Mutex<NetworkState>>,
    lua: Arc<Mutex<KawariLua>>,
    gamedata: Arc<Mutex<GameData>>,
    instance: &mut Instance,
    haters: &mut HashMap<ObjectId, Vec<(ObjectId, u32)>>,
) {
    if instance.enemy_ai_disabled {
        return;
    }

    let is_striking_dummy = |spawn: &SpawnNpc| spawn.common.name_id == STRIKING_DUMMY_NAME_ID;

    // Director-registered bosses don't leash by distance — a boss in a sealed arena should stay
    // engaged until all its haters are gone (a wipe), matching retail. Collected here before the
    // mutable actor passes borrow the instance.
    let boss_ids: Vec<ObjectId> = instance
        .director
        .as_ref()
        .map(|d| d.boss_actor_ids())
        .unwrap_or_default();

    // Only pathfind if there's navmesh data available.
    if instance.navmesh.is_available() {
        let mut actor_moves = Vec::new();
        let enemies = instance.find_possible_enemies();

        let mut target_actor_pos = HashMap::new();

        // const pass
        for (id, actor) in &instance.actors {
            if let NetworkedActor::Npc {
                navmesh_path: current_path,
                navmesh_path_lerp: current_path_lerp,
                navmesh_target: current_target,
                spawn,
                last_position,
                state,
                ..
            } = actor
                && current_target.is_some()
                && *state != NpcState::Dead
                && spawn.common.health_points > 0
                && !is_striking_dummy(spawn)
            {
                let current_target = current_target.unwrap();

                let target_pos;
                if let Some(target_actor) = instance.find_actor(current_target) {
                    target_pos = target_actor.get_common_spawn().position.0;
                } else {
                    // If we can't find the target actor for some reason (despawn, disconnect, left zone), fall back on a sane-ish destination
                    target_pos = last_position.unwrap_or(spawn.common.position.0);
                }

                let distance = Vec3::distance(spawn.common.position.0, target_pos);

                let rotate = |from_pos: Vec3, to_pos: Vec3| {
                    let rotation = f32::atan2(to_pos.x - from_pos.x, to_pos.z - from_pos.z);
                    if rotation >= PI { -PI } else { rotation }
                };

                let position;
                let rotation;
                // If we are in distance, rotate towards target
                if distance <= MINIMUM_PATHFINDING_DISTANCE {
                    position = Some(spawn.common.position);
                    rotation = Some(rotate(spawn.common.position.0, target_pos));
                } else if !current_path.is_empty() {
                    // otherwise, Follow current path
                    let next_position = current_path[0];

                    let current_position = last_position.unwrap_or(spawn.common.position.0);

                    position = Some(Position(Vec3::lerp(
                        current_position,
                        next_position,
                        *current_path_lerp,
                    )));
                    rotation = Some(rotate(current_position, next_position));
                } else {
                    position = None;
                    rotation = None;
                }

                target_actor_pos.insert(current_target, target_pos);

                if let Some(position) = position
                    && let Some(rotation) = rotation
                {
                    actor_moves.push(FromServer::ActorMove(
                        *id,
                        position,
                        rotation,
                        MoveAnimationType::empty(),
                        MoveAnimationState::empty(),
                        JumpState::empty(),
                    ));
                }
            }
        }

        let mut newly_acquired_targets = Vec::new();
        let mut new_action_requests = Vec::new();
        let mut new_timeline_states = Vec::new();
        // NPCs that gave up this tick (dragged too far from home): (id, home position).
        let mut leashed: Vec<(ObjectId, Position)> = Vec::new();

        // mut pass
        for (id, actor) in &mut instance.actors {
            if let NetworkedActor::Npc {
                state,
                navmesh_path: current_path,
                navmesh_path_lerp: current_path_lerp,
                navmesh_target: current_target,
                hate_list,
                spawn,
                last_position,
                spawn_position,
                timeline_position,
                timeline,
                currently_invulnerable,
                ai_paused,
                cast_locked,
                ..
            } = actor
                && *state != NpcState::Dead
                && spawn.common.health_points > 0
            {
                // Paused (boss off doing a mechanic) or locked in a cast bar: don't chase, target,
                // or auto-attack — the boss should freeze while casting, not walk up and melee.
                if *ai_paused {
                    continue;
                }
                // Leash: if a hating NPC has been dragged too far from where it spawned, it
                // gives up combat, teleports home, and fully heals. Bosses are exempt — they stay
                // engaged until a wipe.
                if *state == NpcState::Hate
                    && !boss_ids.contains(id)
                    && Vec3::distance(spawn.common.position.0, *spawn_position) > ENEMY_LEASH_RANGE
                {
                    hate_list.clear();
                    *current_target = None;
                    *state = NpcState::natural_state_of(spawn);
                    current_path.clear();
                    *current_path_lerp = 0.0;
                    *last_position = None;
                    spawn.common.position = Position(*spawn_position);
                    spawn.common.health_points = spawn.common.max_health_points;
                    spawn.common.target_id = ObjectTypeId::default();
                    spawn.common.combat_tagger_id = ObjectTypeId::default();
                    spawn.common.combat_tag_type = 0;
                    spawn
                        .character_data_flags
                        .remove(CharacterDataFlag::IN_COMBAT);
                    leashed.push((*id, Position(*spawn_position)));
                    continue;
                }

                if is_striking_dummy(spawn) {
                    current_path.clear();
                    *current_path_lerp = 0.0;
                }

                // NOTE: this is *intentional* as I believe in retail the timing of actions are dependent on when the actor spawned
                // This doesn't have an effect if you re-aggro them or whatever.
                *timeline_position += 1; // NOTE: change if the length of a server tick changes

                // Locked in a cast bar: freeze MOVEMENT only — clear the path so the boss stops where
                // it stands and doesn't chase. Targeting + auto-attack below keep running on the
                // (still-advancing) timeline, so it doesn't walk up during a cast yet its action
                // cadence stays intact — no post-cast pause ("顿") and no drift ("延迟结算").
                if *cast_locked {
                    current_path.clear();
                    *current_path_lerp = 0.0;
                }

                // switch to the next node if we passed this one
                if *current_path_lerp >= 1.0 {
                    *current_path_lerp = 0.0;
                    if !current_path.is_empty() {
                        *last_position = Some(current_path[0]);
                        current_path.pop_front();
                    }
                }

                let highest_hater = hate_list
                    .iter()
                    .filter(|(actor_id, hate)| {
                        **hate > 0 && enemies.iter().any(|(id, _, _)| id == *actor_id)
                    })
                    .max_by_key(|(_, hate)| *hate)
                    .map(|(actor_id, _)| *actor_id);

                if highest_hater != *current_target {
                    if let Some(actor_id) = highest_hater {
                        *state = NpcState::Hate;
                        *current_target = Some(actor_id);
                        spawn.common.target_id.object_id = actor_id;
                        // Persist the combat claim into the spawn so players who enter range later
                        // (a fresh SpawnNpc) also see the enemy as claimed/red, not just those who
                        // received the transient FirstAttack ACT below.
                        spawn.common.combat_tagger_id.object_id = actor_id;
                        spawn.common.combat_tag_type = 1;
                        spawn
                            .character_data_flags
                            .insert(CharacterDataFlag::IN_COMBAT);
                        newly_acquired_targets.push(*id);
                    }
                }

                if current_target.is_none() {
                    // Wander *and* Stay both sense nearby enemies (Stay = fixed-position bosses that
                    // don't roam but still aggro on approach, e.g. trial bosses). The difference is
                    // movement, not aggro range.
                    if matches!(*state, NpcState::Wander | NpcState::Stay)
                        && !is_striking_dummy(spawn)
                        && spawn
                            .character_data_flags
                            .contains(CharacterDataFlag::HOSTILE)
                    {
                        let mut game_data = gamedata.lock();
                        let possible_enemies =
                            game_data.get_battalion_enemies(spawn.common.battalion as u32);

                        // find a player if in range
                        for (target_id, position, battalion) in &enemies {
                            if !possible_enemies[*battalion as usize] {
                                continue;
                            }

                            // TODO: hardcoded sensing range
                            if Vec3::distance(position.0, spawn.common.position.0) < 15.0 {
                                hate_list.entry(*target_id).or_insert(1);
                                *state = NpcState::Hate;
                                *current_target = Some(*target_id);

                                spawn.common.target_id.object_id = *target_id;
                                spawn.common.combat_tagger_id.object_id = *target_id;
                                spawn.common.combat_tag_type = 1;
                                spawn
                                    .character_data_flags
                                    .insert(CharacterDataFlag::IN_COMBAT);
                                newly_acquired_targets.push(*id);
                            }
                        }
                    } else if *state == NpcState::Follow {
                        // Current target always follows its owner
                        *current_target = Some(spawn.common.owner_id);
                    }
                } else if !current_path.is_empty() {
                    let next_position = current_path[0];
                    let current_position = last_position.unwrap_or(spawn.common.position.0);
                    let distance = Vec3::distance(current_position, next_position);

                    *current_path_lerp =
                        f32::clamp(*current_path_lerp + (2.0 / distance), 0.0, 1.0);
                }

                let mut reset_target = false;
                let can_take_action; // FIXME: this is kind of stupid because enemies can do ranged attacks, etc.
                if let Some(current_target) = current_target {
                    // Check if the enemy is still valid
                    reset_target = !enemies.iter().any(|(id, _, _)| *id == *current_target);

                    if !reset_target && target_actor_pos.contains_key(current_target) {
                        let target_pos = target_actor_pos[current_target];
                        let distance = Vec3::distance(spawn.common.position.0, target_pos);
                        let needs_repath =
                            current_path.is_empty() && distance > MINIMUM_PATHFINDING_DISTANCE;
                        can_take_action = distance <= MINIMUM_PATHFINDING_DISTANCE;

                        let current_pos = spawn.common.position.0;
                        let mut path: VecDeque<Vec3> = instance
                            .navmesh
                            .calculate_path(current_pos, target_pos)
                            .into();

                        // Detour's findStraightPath always prepends the start position as the first
                        // node, so `path[0]` is (roughly) where the NPC stood when this was computed.
                        // The NPC keeps moving every tick, so by the time it follows the path it has
                        // already advanced past that point — interpolating toward `path[0]` would make
                        // it crawl *back* to the compute-time start before heading to the real next
                        // waypoint. Drop that redundant leading node so the path begins at the first
                        // waypoint the NPC actually needs to move toward.
                        if path.len() > 1 {
                            path.pop_front();
                        }

                        if needs_repath && !*cast_locked {
                            *current_path = path.clone();
                            // Anchor the interpolation to where the NPC actually is *now* and restart
                            // the segment lerp, so the first segment runs cleanly from the NPC's
                            // current position to the first real waypoint.
                            *last_position = Some(current_pos);
                            *current_path_lerp = 0.0;
                        }

                        // Drop the current target if we can't path to them too
                        if path.is_empty() {
                            reset_target = true;
                        }
                    } else {
                        can_take_action = false;
                    }
                } else {
                    can_take_action = false;
                }

                // Only update the timeline on exact second marks
                if (*timeline_position % 2) == 0 && !is_striking_dummy(spawn) {
                    // TODO: something worth thinking about is whether to simplify timeline_always_play, and have it always play anyway but skip Action points?

                    // NOTE: the "+ 0.5" is a hack to ensure the last timepoint is always counted
                    let timeline_position_seconds = *timeline_position / 2;
                    let real_timeline_position =
                        timeline_position_seconds as f32 % (timeline.duration() as f32 + 0.5);
                    for timepoint in timeline.points_at(real_timeline_position as i32) {
                        match &timepoint.data {
                            TimepointData::Action { action_id, .. } => {
                                if spawn.common.target_id.object_id.is_valid() && can_take_action {
                                    let cast_time;
                                    {
                                        let mut game_data = gamedata.lock();
                                        cast_time = game_data.get_casttime(*action_id).unwrap(); // TODO: take into account the haste stat like the client does
                                    }
                                    let cast_time_seconds = (cast_time as f32 * 100.0) / 1000.0; // TODO: just change how the Duration is interpreted instead of this nonsense
                                    let request = ActionRequest {
                                        action_id: *action_id,
                                        action_type: ActionType::Action,
                                        rotation1: spawn.common.rotation,
                                        target: spawn.common.target_id,
                                        ..Default::default()
                                    };
                                    new_action_requests.push((*id, request, cast_time_seconds));
                                }
                            }
                            TimepointData::TimelineState { states } => {
                                // Find the event object bound to our gimmick.
                                let gimmick_id = spawn.gimmick_id;
                                new_timeline_states.push((gimmick_id, states.clone()));
                            }
                            TimepointData::Invulnerability { invulnerable } => {
                                *currently_invulnerable = *invulnerable;
                            }
                        }
                    }

                    if spawn.common.target_id.object_id.is_valid()
                        && timeline.autoattack_action_id != 0
                        && can_take_action
                    {
                        // Schedule any pending auto-attacks:
                        let should_auto_attack = (timeline_position_seconds
                            % (ENEMY_AUTO_ATTACK_RATE + 1))
                            == ENEMY_AUTO_ATTACK_RATE;
                        if should_auto_attack {
                            let request = ActionRequest {
                                action_id: timeline.autoattack_action_id,
                                action_type: ActionType::Action,
                                rotation1: spawn.common.rotation,
                                target: spawn.common.target_id,
                                ..Default::default()
                            };
                            new_action_requests.push((*id, request, 0.0));
                        }
                    }
                }

                if reset_target {
                    if let Some(current_target) = current_target {
                        hate_list.remove(current_target);
                    }

                    if let Some(next_target) = hate_list
                        .iter()
                        .filter(|(actor_id, hate)| {
                            **hate > 0 && enemies.iter().any(|(id, _, _)| id == *actor_id)
                        })
                        .max_by_key(|(_, hate)| *hate)
                        .map(|(actor_id, _)| *actor_id)
                    {
                        *current_target = Some(next_target);
                        *state = NpcState::Hate;
                        spawn.common.target_id.object_id = next_target;
                        newly_acquired_targets.push(*id);
                    } else {
                        *current_target = None;
                        *state = NpcState::natural_state_of(spawn);
                        spawn.common.target_id = ObjectTypeId::default();
                    }
                }

                // update common spawn
                for msg in &actor_moves {
                    if let FromServer::ActorMove(msg_id, pos, rotation, ..) = msg
                        && *id == *msg_id
                    {
                        spawn.common.position = *pos;
                        spawn.common.rotation = *rotation;
                    }
                }
            }
        }

        // inform clients of the NPCs new positions
        for msg in actor_moves {
            // Skip movement for NPCs that leashed this tick — they're being teleported home.
            if let FromServer::ActorMove(msg_id, ..) = &msg
                && leashed.iter().any(|(leashed_id, _)| leashed_id == msg_id)
            {
                continue;
            }
            let mut network = network.lock();
            for (handle, _) in network.clients.values_mut() {
                if handle.send(msg.clone()).is_err() {
                    //to_remove.push(id);
                }
            }
        }

        // Broadcast deaggro + return-home for any NPCs that leashed this tick.
        for (id, home) in &leashed {
            let mut network = network.lock();
            network.send_in_range_instance(
                *id,
                instance,
                FromServer::ActorMove(
                    *id,
                    *home,
                    0.0,
                    MoveAnimationType::empty(),
                    MoveAnimationState::empty(),
                    JumpState::empty(),
                ),
                DestinationNetwork::ZoneClients,
            );
            network.send_in_range_instance(
                *id,
                instance,
                FromServer::ActorControlTarget(
                    *id,
                    ObjectTypeId::default(),
                    ActorControlCategory::SetTarget {},
                ),
                DestinationNetwork::ZoneClients,
            );
            network.send_ac_in_range_instance(
                instance,
                *id,
                ActorControlCategory::SetBattle { battle: false },
            );
        }

        for (id, request, cast_time) in new_action_requests {
            if cast_time == 0.0 {
                execute_enemy_action(network.clone(), instance, lua.clone(), id, request);
            } else {
                let position;
                {
                    let actor = instance.find_actor(id).unwrap();
                    position = actor.position();
                }

                // inform players that this enemy is casting
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorCast {
                    spell_id: request.action_id as u16,
                    action_type: request.action_type,
                    omen_delay: 0,
                    action_id: request.action_id,
                    cast_time,
                    target: request.target.object_id,
                    rotation: request.rotation1,
                    interruptible: false,
                    ballista_entity_id: ObjectId::default(),
                    position,
                });

                let mut network = network.lock();
                network.send_in_range_instance(
                    id,
                    instance,
                    FromServer::PacketSegment(ipc, id),
                    DestinationNetwork::ZoneClients,
                );

                instance.insert_task(
                    ClientId::default(),
                    id,
                    Duration::from_secs_f32(cast_time),
                    QueuedTaskData::CastEnemyAction {
                        request: request.clone(),
                    },
                );
            }
        }

        for (gimmick_id, states) in new_timeline_states {
            let actor_id;
            {
                actor_id = instance.find_object_by_bind_layout_id(gimmick_id);
            }
            if let Some(actor_id) = actor_id {
                let mut network = network.lock();
                set_shared_group_timeline_state(instance, &mut network, actor_id, &states);
            }
        }

        // Send aggro visuals (target reticle, in-battle flag, director notification) for NPCs that
        // just acquired a target this tick. Enmity itself is published separately below, so it works
        // even without a navmesh / when the NPC never enters the Hate state.
        for (id, actor) in &instance.actors {
            if let NetworkedActor::Npc {
                state,
                navmesh_target: current_target,
                spawn,
                ..
            } = actor
            {
                if *state == NpcState::Dead {
                    continue;
                }

                if *state == NpcState::Hate
                    && let Some(current_target) = current_target
                    && newly_acquired_targets.contains(id)
                {
                    // Send an ACT for a visual indicator, and stuff.
                    let mut network = network.lock();
                    let target = ObjectTypeId {
                        object_id: *current_target,
                        object_type: ObjectTypeKind::None,
                    };
                    network.send_in_range_instance(
                        *id,
                        instance,
                        FromServer::ActorControlTarget(
                            *id,
                            target,
                            ActorControlCategory::SetTarget {},
                        ),
                        DestinationNetwork::ZoneClients,
                    );

                    // TODO: does this need to be set somewhere in CommonSpawn too?
                    network.send_ac_in_range_instance(
                        instance,
                        *id,
                        ActorControlCategory::SetBattle { battle: true },
                    );

                    // ...and claim it for the attacker. Without this combat tag the client shows an
                    // *orange* (unclaimed) nameplate; the claim makes it red. The no-navmesh path
                    // below does the same — the navmesh path was missing it, so bosses (which use
                    // the navmesh path) never turned red.
                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FirstAttack {
                        unk1: 1,
                        unk2: 0,
                        combat_tagger: *current_target,
                        unk3: 0,
                    });
                    network.send_in_range_inclusive_instance(
                        *id,
                        instance,
                        FromServer::PacketSegment(ipc, *id),
                        DestinationNetwork::ZoneClients,
                    );

                    if let Some(director) = &mut instance.director {
                        director.on_actor_aggro(spawn.common.layout_id);
                    }
                }
            }
        }
    }

    // Without a navmesh the chase logic above never runs, so NPCs can't pick a target or show as
    // engaged. Do a lightweight, movement-free version here so attacking something (e.g. a striking
    // dummy in a city / test area) still turns it hostile and makes it target the attacker — it just
    // won't walk toward them. With a navmesh, the block above already handled this.
    if !instance.navmesh.is_available() {
        let enemies = instance.find_possible_enemies();
        let mut newly_acquired = Vec::new();
        // NPCs that dropped combat this tick: (id, refilled hp, mp) — disengage + snap HP to full.
        let mut disengaged = Vec::new();
        // Pets that need to reposition toward their owner this tick: (id, position, rotation).
        let mut follow_moves = Vec::new();

        for (id, actor) in &mut instance.actors {
            if let NetworkedActor::Npc {
                state,
                hate_list,
                navmesh_target: current_target,
                spawn,
                ai_paused,
                cast_locked,
                ..
            } = actor
                && *state != NpcState::Dead
                && spawn.common.health_points > 0
            {
                if *ai_paused || *cast_locked {
                    continue;
                }
                let npc_pos = spawn.common.position.0;
                // Highest hater that's still a valid enemy *within leash range*. A hater that has
                // run off no longer counts, so the NPC drops combat below.
                let highest_hater = hate_list
                    .iter()
                    .filter(|(actor_id, hate)| {
                        **hate > 0
                            && enemies.iter().any(|(eid, pos, _)| {
                                eid == *actor_id
                                    && Vec3::distance(pos.0, npc_pos) <= ENEMY_LEASH_RANGE
                            })
                    })
                    .max_by_key(|(_, hate)| *hate)
                    .map(|(actor_id, _)| *actor_id);

                match highest_hater {
                    Some(target) if *current_target != Some(target) => {
                        *state = NpcState::Hate;
                        *current_target = Some(target);
                        spawn.common.target_id.object_id = target;
                        spawn.common.combat_tagger_id.object_id = target;
                        spawn.common.combat_tag_type = 1;
                        spawn
                            .character_data_flags
                            .insert(CharacterDataFlag::IN_COMBAT);
                        newly_acquired.push((*id, target));
                    }
                    Some(_) => {} // already engaged with this target
                    None if current_target.is_some() => {
                        // Out of combat (attacker gone or ran off): drop hate, disengage, and heal
                        // to full — so the dummy refills the moment enmity clears, not on the next
                        // hit.
                        hate_list.clear();
                        *current_target = None;
                        *state = NpcState::natural_state_of(spawn);
                        spawn.common.target_id = ObjectTypeId::default();
                        spawn.common.combat_tagger_id = ObjectTypeId::default();
                        spawn.common.combat_tag_type = 0;
                        spawn
                            .character_data_flags
                            .remove(CharacterDataFlag::IN_COMBAT);
                        spawn.common.health_points = spawn.common.max_health_points;
                        disengaged.push((
                            *id,
                            spawn.common.health_points,
                            spawn.common.resource_points,
                        ));
                    }
                    None => {}
                }

                // Pet follow (no navmesh): keep a Follow-state pet near its owner, repositioning
                // when it drifts too far. Choppy on the slow tick, but it stays with the player
                // instead of being stranded (real following needs a navmesh to pathfind).
                if *state == NpcState::Follow
                    && let Some((_, owner_pos, _)) = enemies
                        .iter()
                        .find(|(eid, _, _)| *eid == spawn.common.owner_id)
                {
                    let to_pet = npc_pos - owner_pos.0;
                    let dist = to_pet.length();
                    if dist > PET_FOLLOW_DISTANCE {
                        let new_pos = owner_pos.0 + (to_pet / dist) * PET_FOLLOW_STOP;
                        let rot = f32::atan2(owner_pos.0.x - new_pos.x, owner_pos.0.z - new_pos.z);
                        spawn.common.position = Position(new_pos);
                        spawn.common.rotation = rot;
                        follow_moves.push((*id, Position(new_pos), rot));
                    }
                }
            }
        }

        for (npc_id, target_id) in newly_acquired {
            let mut network = network.lock();
            let target = ObjectTypeId {
                object_id: target_id,
                object_type: ObjectTypeKind::None,
            };
            // Select the attacker (target reticle)...
            network.send_in_range_instance(
                npc_id,
                instance,
                FromServer::ActorControlTarget(npc_id, target, ActorControlCategory::SetTarget {}),
                DestinationNetwork::ZoneClients,
            );
            // ...and flag the NPC as in-battle so its nameplate turns hostile.
            network.send_ac_in_range_instance(
                instance,
                npc_id,
                ActorControlCategory::SetBattle { battle: true },
            );
            // ...and claim it for the attacker. Without this combat tag the client shows an
            // *orange* (unclaimed/in-combat-with-someone-else) nameplate; the claim makes it red.
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FirstAttack {
                unk1: 1,
                unk2: 0,
                combat_tagger: target_id,
                unk3: 0,
            });
            network.send_in_range_inclusive_instance(
                npc_id,
                instance,
                FromServer::PacketSegment(ipc, npc_id),
                DestinationNetwork::ZoneClients,
            );
        }

        for (npc_id, hp, mp) in disengaged {
            let mut network = network.lock();
            // Leave battle (nameplate back to neutral)...
            network.send_ac_in_range_instance(
                instance,
                npc_id,
                ActorControlCategory::SetBattle { battle: false },
            );
            // ...drop the combat tag (claim)...
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FirstAttack {
                unk1: 1,
                unk2: 0,
                combat_tagger: ObjectId::default(),
                unk3: 0,
            });
            network.send_in_range_inclusive_instance(
                npc_id,
                instance,
                FromServer::PacketSegment(ipc, npc_id),
                DestinationNetwork::ZoneClients,
            );
            // ...and broadcast the refilled HP so the bar snaps back to full.
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp { hp, mp, unk: 0 });
            network.send_in_range_inclusive_instance(
                npc_id,
                instance,
                FromServer::PacketSegment(ipc, npc_id),
                DestinationNetwork::ZoneClients,
            );
        }

        for (npc_id, position, rotation) in follow_moves {
            let mut network = network.lock();
            network.send_in_range_instance(
                npc_id,
                instance,
                FromServer::ActorMove(
                    npc_id,
                    position,
                    rotation,
                    MoveAnimationType::empty(),
                    MoveAnimationState::empty(),
                    JumpState::empty(),
                ),
                DestinationNetwork::ZoneClients,
            );
        }
    }

    // Publish every living NPC's hate to the actors on its list, so attackers always see their
    // enmity / hater list. Deliberately outside the navmesh block above: pathfinding needs a
    // navmesh, but reading hate to build the enmity list does not — so a striking dummy spawned in
    // a navmesh-less zone (cities, test areas) still shows enmity. Also independent of AI state: a
    // dummy records hate but never enters the Hate/chase state. `haters` is rebuilt fresh each tick.
    for (id, actor) in &instance.actors {
        if let NetworkedActor::Npc {
            state, hate_list, ..
        } = actor
            && *state != NpcState::Dead
        {
            for (hated_actor_id, enmity) in hate_list {
                if *enmity == 0 {
                    continue;
                }

                haters
                    .entry(*hated_actor_id)
                    .or_default()
                    .push((*id, *enmity));
            }
        }
    }
}
