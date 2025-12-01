use mlua::Lua;
use parking_lot::Mutex;
use std::{
    collections::HashMap, env::consts::EXE_SUFFIX, process::Command, sync::Arc, time::Duration,
};
use tokio::sync::mpsc::Receiver;

use crate::{
    common::{
        GameData, JumpState, MoveAnimationState, MoveAnimationType, ObjectId, ObjectTypeId,
        ObjectTypeKind, Position,
    },
    ipc::zone::{
        ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget, BattleNpcSubKind,
        ClientTriggerCommand, CommonSpawn, Conditions, NpcSpawn, ObjectKind,
    },
    world::{
        Navmesh,
        lua::load_init_script,
        server::{
            action::handle_action_messages,
            actor::NetworkedActor,
            chat::handle_chat_messages,
            instance::{Instance, NavmeshGenerationStep},
            network::{DestinationNetwork, NetworkState},
            social::handle_social_messages,
            zone::{change_zone_warp_to_entrance, handle_zone_messages},
        },
    },
};

use super::{Actor, ClientId, FromServer, ToServer};

use crate::world::common::SpawnKind;

mod action;
mod actor;
mod chat;
mod instance;
mod network;
mod social;
mod zone;

#[derive(Default, Debug, Clone)]
struct ClientState {}

#[derive(Default, Debug)]
struct WorldServer {
    /// Indexed by zone id
    instances: HashMap<u16, Instance>,
}

impl WorldServer {
    /// Ensures an instance exists, and creates one if not found.
    fn ensure_exists(&mut self, zone_id: u16, game_data: &mut GameData) {
        // create a new instance if necessary
        self.instances
            .entry(zone_id)
            .or_insert_with(|| Instance::new(zone_id, game_data));
    }

    /// Finds the instance associated with a zone, or None if it doesn't exist yet.
    fn find_instance(&self, zone_id: u16) -> Option<&Instance> {
        self.instances.get(&zone_id)
    }

    /// Finds the instance associated with a zone, or creates it if it doesn't exist yet
    fn find_instance_mut(&mut self, zone_id: u16) -> &mut Instance {
        self.instances.entry(zone_id).or_default()
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance(&self, actor_id: u32) -> Option<&Instance> {
        self.instances
            .values()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }

    /// Finds the instance associated with an actor, or returns None if they are not found.
    fn find_actor_instance_mut(&mut self, actor_id: u32) -> Option<&mut Instance> {
        self.instances
            .values_mut()
            .find(|instance| instance.actors.contains_key(&ObjectId(actor_id)))
    }
}

fn set_player_minion(
    data: &mut WorldServer,
    network: &mut NetworkState,
    to_remove: &mut Vec<ClientId>,
    minion_id: u32,
    from_id: ClientId,
    from_actor_id: u32,
) {
    for (id, (handle, _)) in &mut network.clients {
        let id = *id;

        // Update our common spawn to reflect the new minion
        if id == from_id {
            let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                break;
            };

            let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                break;
            };

            let NetworkedActor::Player(player) = actor else {
                break;
            };

            player.common.active_minion = minion_id as u16;

            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                category: ActorControlCategory::MinionSpawnControl { minion_id },
            });

            if handle.send(msg).is_err() {
                to_remove.push(id);
            }

            // Skip sending the regular ActorControl to ourselves
            continue;
        }

        let msg = FromServer::ActorControl(
            from_actor_id,
            ActorControl {
                category: ActorControlCategory::MinionSpawnControl { minion_id },
            },
        );

        if handle.send(msg).is_err() {
            to_remove.push(id);
        }
    }
}

fn server_logic_tick(data: &mut WorldServer, network: &mut NetworkState) {
    for instance in data.instances.values_mut() {
        // Only pathfind if there's navmesh data available.
        if instance.navmesh.is_available() {
            let mut actor_moves = Vec::new();
            let players = instance.find_all_players();

            let mut target_actor_pos = HashMap::new();

            // const pass
            for (id, actor) in &instance.actors {
                if let NetworkedActor::Npc {
                    current_path,
                    current_path_lerp,
                    current_target,
                    spawn,
                    last_position,
                } = actor
                    && current_target.is_some()
                {
                    let current_target = current_target.unwrap();
                    let needs_repath = current_path.is_empty();
                    if !needs_repath {
                        // follow current path
                        let next_position = Position {
                            x: current_path[0][0],
                            y: current_path[0][1],
                            z: current_path[0][2],
                        };
                        let current_position = last_position.unwrap_or(spawn.common.pos);

                        let dir_x = current_position.x - next_position.x;
                        let dir_z = current_position.z - next_position.z;
                        let rotation = f32::atan2(-dir_z, dir_x).to_degrees();

                        actor_moves.push(FromServer::ActorMove(
                            id.0,
                            Position::lerp(current_position, next_position, *current_path_lerp),
                            rotation,
                            MoveAnimationType::RUNNING,
                            MoveAnimationState::None,
                            JumpState::NoneOrFalling,
                        ));
                    }

                    let target_pos;
                    if let Some(target_actor) = instance.find_actor(current_target) {
                        target_pos = target_actor.get_common_spawn().pos;
                    } else {
                        // If we can't find the target actor for some reason (despawn, disconnect, left zone), fall back on a sane-ish destination
                        target_pos = last_position.unwrap_or(spawn.common.pos);
                    }

                    target_actor_pos.insert(current_target, target_pos);
                }
            }

            // mut pass
            for (id, actor) in &mut instance.actors {
                if let NetworkedActor::Npc {
                    current_path,
                    current_path_lerp,
                    current_target,
                    spawn,
                    last_position,
                } = actor
                {
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

                    if current_target.is_none() {
                        // find a player
                        if !players.is_empty() {
                            *current_target = Some(players[0]);
                        }
                    } else if !current_path.is_empty() {
                        let next_position = Position {
                            x: current_path[0][0],
                            y: current_path[0][1],
                            z: current_path[0][2],
                        };
                        let current_position = last_position.unwrap_or(spawn.common.pos);
                        let distance = Position::distance(current_position, next_position);

                        // TODO: this doesn't work like it should
                        *current_path_lerp += (10.0 / distance).clamp(0.0, 1.0);
                    }

                    if target_actor_pos.contains_key(&current_target.unwrap()) {
                        let target_pos = target_actor_pos[&current_target.unwrap()];
                        let distance = Position::distance(spawn.common.pos, target_pos);
                        let needs_repath = current_path.is_empty() && distance > 5.0; // TODO: confirm distance this in retail
                        if needs_repath && current_target.is_some() {
                            let current_pos = spawn.common.pos;
                            *current_path = instance
                                .navmesh
                                .calculate_path(
                                    [current_pos.x, current_pos.y, current_pos.z],
                                    [target_pos.x, target_pos.y, target_pos.z],
                                )
                                .into();
                        }
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
                            && id.0 == *msg_id
                        {
                            spawn.common.pos = *pos;
                            spawn.common.rotation = *rotation;
                        }
                    }
                }
            }

            // inform clients of the NPCs new positions
            for msg in actor_moves {
                for (handle, _) in network.clients.values_mut() {
                    if handle.send(msg.clone()).is_err() {
                        //to_remove.push(id);
                    }
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
                        instance.generate_navmesh = NavmeshGenerationStep::Started(nvm_path.clone())
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
    }
}

pub async fn server_main_loop(mut recv: Receiver<ToServer>) -> Result<(), std::io::Error> {
    let data = Arc::new(Mutex::new(WorldServer::default()));
    let network = Arc::new(Mutex::new(NetworkState::default()));
    let game_data = Arc::new(Mutex::new(GameData::new()));
    let lua = Arc::new(Mutex::new(Lua::new()));

    // Run Init.lua and set up other Lua state
    {
        let mut lua = lua.lock();
        if let Err(err) = load_init_script(&mut lua, game_data.clone()) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }

    {
        let data = data.clone();
        let network = network.clone();
        tokio::task::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            interval.tick().await;
            loop {
                interval.tick().await;
                let mut data = data.lock();
                let mut network = network.lock();
                server_logic_tick(&mut data, &mut network);
            }
        });
    }

    while let Some(msg) = recv.recv().await {
        let mut to_remove = Vec::new();

        // TODO: return bool if the message was handled
        handle_chat_messages(data.clone(), network.clone(), &msg);
        handle_social_messages(data.clone(), network.clone(), &msg);
        handle_zone_messages(data.clone(), network.clone(), game_data.clone(), &msg);
        handle_action_messages(
            data.clone(),
            network.clone(),
            game_data.clone(),
            lua.clone(),
            &msg,
        );

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
                        if member.actor_id.0 == handle.actor_id {
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
                        if member.actor_id.0 == handle.actor_id {
                            member.chat_client_id = handle.id; // The chat connection doesn't get informed here since it'll happen later.
                            break 'outer;
                        }
                    }
                }

                network
                    .chat_clients
                    .insert(handle.id, (handle, ClientState::default()));
            }
            ToServer::ReadySpawnPlayer(from_id, zone_id, position, rotation) => {
                let mut network = network.lock();
                let mut data = data.lock();

                // create a new instance if necessary
                let mut game_data = game_data.lock();
                data.ensure_exists(zone_id, &mut game_data);
                let target_instance = data.find_instance_mut(zone_id);

                // tell the client to load into the zone
                let msg = FromServer::ChangeZone(
                    zone_id,
                    target_instance.weather_id,
                    position,
                    rotation,
                    target_instance.zone.to_lua_zone(target_instance.weather_id),
                    true, // since this is initial login
                );

                network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::ActorMoved(
                from_id,
                actor_id,
                position,
                rotation,
                anim_type,
                anim_state,
                jump_state,
            ) => {
                let mut data = data.lock();
                let mut network = network.lock();

                if let Some(instance) = data.find_actor_instance_mut(actor_id) {
                    if let Some((_, spawn)) = instance
                        .actors
                        .iter_mut()
                        .find(|actor| *actor.0 == ObjectId(actor_id))
                    {
                        let common = match spawn {
                            NetworkedActor::Player(npc_spawn) => &mut npc_spawn.common,
                            NetworkedActor::Npc { spawn, .. } => &mut spawn.common,
                        };
                        common.pos = position;
                        common.rotation = rotation;
                    }

                    let msg = FromServer::ActorMove(
                        actor_id, position, rotation, anim_type, anim_state, jump_state,
                    );
                    network.send_to_all(Some(from_id), msg, DestinationNetwork::ZoneClients);
                }
            }
            ToServer::ClientTrigger(from_id, from_actor_id, trigger) => {
                let mut network = network.lock();

                for (id, (handle, _)) in &mut network.clients {
                    let id = *id;

                    tracing::info!("{:#X?}", trigger);

                    // handle player-to-server actions
                    if id == from_id {
                        if let ClientTriggerCommand::TeleportQuery { aetheryte_id } =
                            &trigger.trigger
                        {
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::TeleportStart {
                                    insufficient_gil: 0,
                                    aetheryte_id: *aetheryte_id,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::EventRelatedUnk { .. } = &trigger.trigger {
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::MapMarkerUpdateBegin { unk1: 1 },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::MapMarkerUpdateEnd { unk1: 0 },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::WalkInTriggerFinished { .. } = &trigger.trigger
                        {
                            // This is where we finally release the client after the walk-in trigger.
                            let msg = FromServer::Conditions(Conditions::default());

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::WalkInTriggerRelatedUnk1 {
                                    unk1: 0,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            // Yes, this is actually sent every time the trigger event finishes...
                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::CompanionUnlock {
                                    unk1: 0,
                                    unk2: 1,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }

                            let msg = FromServer::ActorControlSelf(ActorControlSelf {
                                category: ActorControlCategory::WalkInTriggerRelatedUnk2 {
                                    unk1: 0,
                                    unk2: 0,
                                    unk3: 0,
                                    unk4: 7,
                                },
                            });

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::SummonMinion { minion_id } = &trigger.trigger {
                            let msg = FromServer::ActorSummonsMinion(*minion_id);

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }

                        if let ClientTriggerCommand::DespawnMinion { .. } = &trigger.trigger {
                            let msg = FromServer::ActorDespawnsMinion();

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        continue;
                    }

                    match &trigger.trigger {
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
                                    // TODO: Are there other types?
                                    tracing::warn!(
                                        "SetTarget: Unknown actor target type {}! Defaulting to None!",
                                        *actor_type
                                    );
                                    ObjectTypeKind::None
                                }
                            };

                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::SetTarget {
                                        target: ObjectTypeId {
                                            object_id: *actor_id,
                                            object_type: actor_type,
                                        },
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ChangePose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Pose {
                                        unk1: *unk1,
                                        pose: *pose,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ReapplyPose { unk1, pose } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::Pose {
                                        unk1: *unk1,
                                        pose: *pose,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::Emote(emote_info) => {
                            let msg = FromServer::ActorControlTarget(
                                from_actor_id,
                                ActorControlTarget {
                                    category: ActorControlCategory::Emote(*emote_info),
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ToggleWeapon { shown, unk_flag } => {
                            let msg = FromServer::ActorControl(
                                from_actor_id,
                                ActorControl {
                                    category: ActorControlCategory::ToggleWeapon {
                                        shown: *shown,
                                        unk_flag: *unk_flag,
                                    },
                                },
                            );

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        ClientTriggerCommand::ManuallyRemoveEffect {
                            effect_id,
                            source_actor_id,
                            ..
                        } => {
                            // TODO: we need to inform the ZoneConnection as well since it keeps track of its own status effect list...

                            let msg =
                                FromServer::LoseEffect(*effect_id as u16, 0, *source_actor_id);

                            if handle.send(msg).is_err() {
                                to_remove.push(id);
                            }
                        }
                        _ => tracing::warn!("Server doesn't know what to do with {:#?}", trigger),
                    }
                }
            }
            ToServer::DebugNewEnemy(_from_id, from_actor_id, id) => {
                let mut data = data.lock();
                let mut network = network.lock();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    let model_chara;
                    {
                        let mut game_data = game_data.lock();
                        model_chara = game_data.find_bnpc(id).unwrap();
                    }

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: CommonSpawn {
                            hp_curr: 91,
                            hp_max: 91,
                            mp_curr: 100,
                            mp_max: 100,
                            spawn_index: 0, // not needed at this level
                            bnpc_base: id,
                            bnpc_name: 405,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            level: 1,
                            battalion: 4,
                            model_chara,
                            pos: player.common.pos,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::DebugSpawnClone(_from_id, from_actor_id) => {
                let mut data = data.lock();
                let mut network = network.lock();

                let actor_id = Instance::generate_actor_id();
                let spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    spawn = NpcSpawn {
                        aggression_mode: 1,
                        common: player.common.clone(),
                        ..Default::default()
                    };

                    instance.insert_npc(ObjectId(actor_id), spawn.clone());
                }

                network.send_actor(
                    Actor {
                        id: ObjectId(actor_id),
                        ..Default::default()
                    },
                    SpawnKind::Npc(spawn),
                );
            }
            ToServer::Config(_from_id, from_actor_id, config) => {
                // update their stored state so it's correctly sent on new spawns
                {
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    player.common.display_flags = config.display_flag.into();
                }

                let mut network = network.lock();
                let msg = FromServer::UpdateConfig(from_actor_id, config.clone());

                network.send_to_all(None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::Equip(_from_id, from_actor_id, main_weapon_id, sub_weapon_id, model_ids) => {
                // update their stored state so it's correctly sent on new spawns
                {
                    let mut data = data.lock();

                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        break;
                    };

                    let Some(actor) = instance.find_actor_mut(ObjectId(from_actor_id)) else {
                        break;
                    };

                    let NetworkedActor::Player(player) = actor else {
                        break;
                    };

                    player.common.main_weapon_model = main_weapon_id;
                    player.common.sec_weapon_model = sub_weapon_id;
                    player.common.models = model_ids;
                }

                // Inform all clients about their new equipped model ids
                let msg =
                    FromServer::ActorEquip(from_actor_id, main_weapon_id, sub_weapon_id, model_ids);

                let mut network = network.lock();
                network.send_to_all(None, msg, DestinationNetwork::ZoneClients);
            }
            ToServer::GainEffect(
                from_id,
                _from_actor_id,
                effect_id,
                effect_duration,
                effect_param,
                effect_source_actor_id,
            ) => {
                let send_lost_effect =
                    |from_id: ClientId,
                     network: Arc<Mutex<NetworkState>>,
                     effect_id: u16,
                     effect_param: u16,
                     effect_source_actor_id: ObjectId| {
                        let mut network = network.lock();

                        tracing::info!("Now losing effect {}!", effect_id);

                        let msg =
                            FromServer::LoseEffect(effect_id, effect_param, effect_source_actor_id);
                        network.send_to(from_id, msg, DestinationNetwork::ZoneClients);
                    };

                // Eventually tell the player they lost this effect
                // NOTE: I know this won't scale, but it's a fine hack for now

                tracing::info!("Effect {effect_id} lasts for {effect_duration} seconds");

                // we have to shadow these variables to tell rust not to move them into the async closure
                let network = network.clone();
                tokio::task::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_millis(
                        (effect_duration * 1000.0) as u64,
                    ));
                    interval.tick().await;
                    interval.tick().await;
                    send_lost_effect(
                        from_id,
                        network,
                        effect_id,
                        effect_param,
                        effect_source_actor_id,
                    );
                });
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
            }
            ToServer::ActorSummonsMinion(from_id, from_actor_id, minion_id) => {
                let mut network = network.lock();
                let mut data = data.lock();

                set_player_minion(
                    &mut data,
                    &mut network,
                    &mut to_remove,
                    minion_id,
                    from_id,
                    from_actor_id,
                );
            }
            ToServer::ActorDespawnsMinion(from_id, from_actor_id) => {
                let mut network = network.lock();
                let mut data = data.lock();

                set_player_minion(
                    &mut data,
                    &mut network,
                    &mut to_remove,
                    0,
                    from_id,
                    from_actor_id,
                );
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

                if let Some(zone_id) = zone_id {
                    let mut data = data.lock();
                    let mut network = network.lock();
                    let mut game_data = game_data.lock();

                    change_zone_warp_to_entrance(
                        &mut data,
                        &mut network,
                        &mut game_data,
                        zone_id,
                        from_actor_id,
                        from_id,
                    );
                } else {
                    tracing::warn!("Failed to find zone id for content?!");
                }
            }
            ToServer::FatalError(err) => return Err(err),
            _ => {}
        }

        // Remove any clients that errored out
        {
            let mut network = network.lock();
            let mut data = data.lock();

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
                        current_instance.actors.remove(&ObjectId(actor_id));
                        network.inform_remove_actor(current_instance, remove_id, actor_id);
                    }
                }

                network.clients.remove(&remove_id);
            }

            for remove_id in network.to_remove_chat.clone() {
                network.chat_clients.remove(&remove_id);
            }
        }
    }
    Ok(())
}
