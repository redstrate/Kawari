use std::{
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    sync::Arc,
    time::Duration,
};

use kawari::{
    common::{
        ENEMY_AUTO_ATTACK_RATE, JumpState, MINIMUM_PATHFINDING_DISTANCE, MoveAnimationState,
        MoveAnimationType, ObjectId, ObjectTypeId, ObjectTypeKind, Position, TimepointData,
    },
    ipc::zone::{
        ActionKind, ActionRequest, ActorControlCategory, CharacterDataFlag, ServerZoneIpcData,
        ServerZoneIpcSegment,
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

/// Updates NPCs in this instance.
pub fn npc_behavior(
    network: Arc<Mutex<NetworkState>>,
    lua: Arc<Mutex<KawariLua>>,
    gamedata: Arc<Mutex<GameData>>,
    instance: &mut Instance,
    haters: &mut HashMap<ObjectId, Vec<ObjectId>>,
) {
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
                ..
            } = actor
                && current_target.is_some()
            {
                let current_target = current_target.unwrap();

                let target_pos;
                if let Some(target_actor) = instance.find_actor(current_target) {
                    target_pos = target_actor.get_common_spawn().position;
                } else {
                    // If we can't find the target actor for some reason (despawn, disconnect, left zone), fall back on a sane-ish destination
                    target_pos = last_position.unwrap_or(spawn.common.position);
                }

                let distance = Position::distance(spawn.common.position, target_pos);

                let rotate = |from_pos: Position, to_pos: Position| {
                    let rotation = f32::atan2(to_pos.x - from_pos.x, to_pos.z - from_pos.z);
                    if rotation >= PI { -PI } else { rotation }
                };

                let position;
                let rotation;
                // If we are in distance, rotate towards target
                if distance <= MINIMUM_PATHFINDING_DISTANCE {
                    position = Some(spawn.common.position);
                    rotation = Some(rotate(spawn.common.position, target_pos));
                } else if !current_path.is_empty() {
                    // otherwise, Follow current path
                    let next_position = Position {
                        x: current_path[0][0],
                        y: current_path[0][1],
                        z: current_path[0][2],
                    };

                    let current_position = last_position.unwrap_or(spawn.common.position);

                    position = Some(Position::lerp(
                        current_position,
                        next_position,
                        *current_path_lerp,
                    ));
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
                        MoveAnimationType::RUNNING,
                        MoveAnimationState::None,
                        JumpState::NoneOrFalling,
                    ));
                }
            }
        }

        let mut newly_acquired_targets = Vec::new();
        let mut new_action_requests = Vec::new();
        let mut new_timeline_states = Vec::new();

        // mut pass
        for (id, actor) in &mut instance.actors {
            if let NetworkedActor::Npc {
                state,
                navmesh_path: current_path,
                navmesh_path_lerp: current_path_lerp,
                navmesh_target: current_target,
                spawn,
                last_position,
                timeline_position,
                timeline,
                newly_hated_actor,
                currently_invulnerable,
                ..
            } = actor
                && *state != NpcState::Dead
            {
                // NOTE: this is *intentional* as I believe in retail the timing of actions are dependent on when the actor spawned
                // This doesn't have an effect if you re-aggro them or whatever.
                *timeline_position += 1; // NOTE: change if the length of a server tick changes

                // switch to the next node if we passed this one
                if *current_path_lerp >= 1.0 {
                    *current_path_lerp = 0.0;
                    if !current_path.is_empty() {
                        *last_position = Some(Position {
                            x: current_path[0][0],
                            y: current_path[0][1],
                            z: current_path[0][2],
                        });
                        current_path.pop_front();
                    }
                }

                // Pick up any newly hated actors first.
                if let Some(actor) = newly_hated_actor.take() {
                    *state = NpcState::Hate;
                    *current_target = Some(actor);

                    spawn.common.target_id.object_id = actor;
                    newly_acquired_targets.push(*id);
                }

                if current_target.is_none() {
                    if *state == NpcState::Wander
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
                            if Position::distance(*position, spawn.common.position) < 15.0 {
                                *state = NpcState::Hate;
                                *current_target = Some(*target_id);

                                spawn.common.target_id.object_id = *target_id;
                                newly_acquired_targets.push(*id);
                            }
                        }
                    } else if *state == NpcState::Follow {
                        // Current target always follows its owner
                        *current_target = Some(spawn.common.owner_id);
                    }
                } else if !current_path.is_empty() {
                    let next_position = Position {
                        x: current_path[0][0],
                        y: current_path[0][1],
                        z: current_path[0][2],
                    };
                    let current_position = last_position.unwrap_or(spawn.common.position);
                    let distance = Position::distance(current_position, next_position);

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
                        let distance = Position::distance(spawn.common.position, target_pos);
                        let needs_repath =
                            current_path.is_empty() && distance > MINIMUM_PATHFINDING_DISTANCE;
                        can_take_action = distance <= MINIMUM_PATHFINDING_DISTANCE;

                        let current_pos = spawn.common.position;
                        let path: VecDeque<[f32; 3]> = instance
                            .navmesh
                            .calculate_path(
                                [current_pos.x, current_pos.y, current_pos.z],
                                [target_pos.x, target_pos.y, target_pos.z],
                            )
                            .into();

                        if needs_repath {
                            *current_path = path.clone();
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
                if (*timeline_position % 2) == 0 {
                    // TODO: something worth thinking about is whether to simplify timeline_always_play, and have it always play anyway but skip Action points?

                    // NOTE: the "+ 0.5" is a hack to ensure the last timepoint is always counted
                    let timeline_position_seconds = *timeline_position / 2;
                    let real_timeline_position =
                        timeline_position_seconds as f32 % (timeline.duration() as f32 + 0.5);
                    for timepoint in timeline.points_at(real_timeline_position as i32) {
                        match &timepoint.data {
                            TimepointData::Action { action_id, .. } => {
                                if spawn.common.target_id.object_id.is_valid() && can_take_action {
                                    let cast_time = 2.7; // TODO: grab from excel data
                                    let request = ActionRequest {
                                        action_key: *action_id,
                                        exec_proc: 0,
                                        action_kind: ActionKind::Normal,
                                        request_id: 0,
                                        rotation: spawn.common.rotation,
                                        dir: 0,
                                        dir_target: 0,
                                        target: spawn.common.target_id,
                                        arg: 0,
                                        padding_prob: 0,
                                    };
                                    new_action_requests.push((*id, request, cast_time));
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
                                action_key: timeline.autoattack_action_id,
                                exec_proc: 0,
                                action_kind: ActionKind::Normal,
                                request_id: 0,
                                rotation: spawn.common.rotation,
                                dir: 0,
                                dir_target: 0,
                                target: spawn.common.target_id,
                                arg: 0,
                                padding_prob: 0,
                            };
                            new_action_requests.push((*id, request, 0.0));
                        }
                    }
                }

                if reset_target {
                    *current_target = None;
                    *state = NpcState::natural_state_of(spawn);
                    spawn.common.target_id = ObjectTypeId::default();
                }

                // update common spawn
                for msg in &actor_moves {
                    if let FromServer::ActorMove(
                        msg_id,
                        pos,
                        rotation,
                        MoveAnimationType::RUNNING,
                        MoveAnimationState::None,
                        JumpState::NoneOrFalling,
                    ) = msg
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
            let mut network = network.lock();
            for (handle, _) in network.clients.values_mut() {
                if handle.send(msg.clone()).is_err() {
                    //to_remove.push(id);
                }
            }
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
                    action: request.action_key as u16,
                    action_kind: request.action_kind,
                    action_key: request.action_key,
                    cast_time,
                    dir: request.rotation,
                    unk1: 1436,
                    target: ObjectId::default(),
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

        // create hate list
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

                if let Some(current_target) = current_target {
                    if newly_acquired_targets.contains(id) {
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

                        if let Some(director) = &mut instance.director {
                            director.on_actor_aggro(spawn.common.layout_id);
                        }
                    }

                    haters.entry(*current_target).or_default();
                    haters.get_mut(current_target).unwrap().push(*id);
                }
            }
        }
    }
}
