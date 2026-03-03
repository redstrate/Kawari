use std::{collections::HashMap, sync::Arc, time::Duration};

use kawari::{
    common::{
        DirectorEvent, HandlerId, InvisibilityFlags, ObjectId, ObjectTypeId, ObjectTypeKind,
        Position,
    },
    ipc::zone::{ActorControlCategory, ActorControlSelf, ServerZoneIpcData, ServerZoneIpcSegment},
};
use mlua::{Function, LuaSerdeExt, UserData, UserDataMethods, Value};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, ToServer,
    lua::KawariLua,
    server::{
        WorldServer,
        actor::NetworkedActor,
        effect::gain_effect_instance,
        instance::{Instance, QueuedTaskData},
        network::{DestinationNetwork, NetworkState},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum LuaDirectorTask {
    HideEObj {
        base_id: u32,
    },
    ShowEObj {
        base_id: u32,
    },
    DeleteEObj {
        base_id: u32,
    },
    SpawnEObj {
        base_id: u32,
        position: Option<Position>,
    },
    SendVariables,
    AbandonDuty {
        actor_id: ObjectId,
    },
    BeginEventAction {
        actor_id: ObjectId,
        target: ObjectId,
        action_id: u32,
    },
    FinishGimmickEvent {
        actor_id: ObjectId,
    },
    LogMessage {
        id: u32,
        params: Vec<u32>,
    },
    SpawnBattleNpc {
        id: u32,
    },
    GainEffect {
        actor_id: ObjectId,
        id: u16,
        param: u16,
        duration: f32,
    },
    SetBGM {
        id: u32,
    },
    SealBossWall {
        actor_id: ObjectId,
        id: u32,
        place_name: u32,
        time_until: u32,
    },
    SpawnBoss {
        bnpc_id: u32,
        wall_id: u32,
        line_id: u32,
        place_name: u32,
    },
}

// TODO: Maybe collapse into DirectorData?
#[derive(Default, Debug)]
pub struct LuaDirector {
    pub data: [u8; 10],
    pub tasks: Vec<LuaDirectorTask>,
    pub bosses: HashMap<u32, DirectorBoss>,
}

impl UserData for LuaDirector {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("hide_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::HideEObj { base_id });
            Ok(())
        });
        methods.add_method_mut("show_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::ShowEObj { base_id });
            Ok(())
        });
        methods.add_method_mut("delete_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::DeleteEObj { base_id });
            Ok(())
        });
        methods.add_method_mut(
            "spawn_eobj",
            |lua, this, (base_id, position): (u32, Value)| {
                let position: Option<Position> = lua.from_value(position).ok();
                this.tasks
                    .push(LuaDirectorTask::SpawnEObj { base_id, position });
                Ok(())
            },
        );
        methods.add_method_mut("set_data", |_, this, (index, data): (u8, u8)| {
            this.data[index as usize] = data;
            Ok(())
        });
        methods.add_method("data", |_, this, index: u8| Ok(this.data[index as usize]));
        methods.add_method_mut("abandon_duty", |_, this, actor_id: u32| {
            this.tasks.push(LuaDirectorTask::AbandonDuty {
                actor_id: ObjectId(actor_id),
            });
            Ok(())
        });
        methods.add_method_mut(
            "event_action",
            |_, this, (action_id, actor_id, target): (u32, u32, u32)| {
                this.tasks.push(LuaDirectorTask::BeginEventAction {
                    actor_id: ObjectId(actor_id),
                    target: ObjectId(target),
                    action_id,
                });
                Ok(())
            },
        );
        methods.add_method_mut("finish_gimmick", |_, this, actor_id: u32| {
            this.tasks.push(LuaDirectorTask::FinishGimmickEvent {
                actor_id: ObjectId(actor_id),
            });
            Ok(())
        });
        methods.add_method_mut("log_message", |_, this, (id, params): (u32, Vec<u32>)| {
            this.tasks.push(LuaDirectorTask::LogMessage { id, params });
            Ok(())
        });
        methods.add_method_mut("spawn_bnpc", |_, this, id: u32| {
            this.tasks.push(LuaDirectorTask::SpawnBattleNpc { id });
            Ok(())
        });
        methods.add_method_mut(
            "gain_effect",
            |_, this, (actor_id, id, param, duration): (u32, u16, u16, f32)| {
                this.tasks.push(LuaDirectorTask::GainEffect {
                    actor_id: ObjectId(actor_id),
                    id,
                    param,
                    duration,
                });
                Ok(())
            },
        );
        methods.add_method_mut("set_bgm", |_, this, id: u32| {
            this.tasks.push(LuaDirectorTask::SetBGM { id });
            Ok(())
        });
        methods.add_method_mut(
            "spawn_boss",
            |_, this, (bnpc_id, wall_id, line_id, place_name): (u32, u32, u32, u32)| {
                this.tasks.push(LuaDirectorTask::SpawnBoss {
                    bnpc_id,
                    wall_id,
                    line_id,
                    place_name,
                });
                Ok(())
            },
        );
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectorBoss {
    actor_id: ObjectId,
    wall_id: u32,
    line_id: u32,
    place_name: u32,
}

#[derive(Debug)]
pub struct DirectorData {
    pub id: HandlerId,
    pub flag: u8,
    pub data: [u8; 10],
    /// Lua state for this director.
    pub lua: KawariLua,
    pub tasks: Vec<LuaDirectorTask>,
    /// List of alive bosses and their data.
    pub bosses: HashMap<u32, DirectorBoss>,
}

impl DirectorData {
    pub fn setup(&mut self) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onSetup")?;

                func.call::<()>(data)?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onSetup: {err:?}");
        }
    }

    pub fn gimmick_accessor(&mut self, actor_id: ObjectId, id: u32, params: &[i32]) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onGimmickAccessor")?;

                func.call::<()>((data, actor_id.0, id, params))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onGimmickAccessor: {err:?}");
        }
    }

    pub fn event_action_cast(&mut self, actor_id: ObjectId, target: ObjectId) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onEventActionCast")?;

                func.call::<()>((data, actor_id.0, target.0))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onEventActionCast: {err:?}");
        }
    }

    pub fn on_actor_death(&mut self, bnpc_id: u32, position: Position) {
        if let Some(boss) = self.bosses.get(&bnpc_id) {
            self.unseal_boss_wall(boss.wall_id, boss.line_id, boss.place_name);
        }

        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onActorDeath")?;

                func.call::<()>((data, bnpc_id, position))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onActorDeath: {err:?}");
        }
    }

    pub fn build_var_segment(&self) -> ServerZoneIpcSegment {
        ServerZoneIpcSegment::new(ServerZoneIpcData::DirectorVars {
            handler_id: self.id,
            flag: self.flag,
            branch: 0,
            data: self.data,
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
        })
    }

    fn create_lua_director(&self) -> LuaDirector {
        LuaDirector {
            data: self.data,
            tasks: Vec::new(),
            bosses: self.bosses.clone(),
        }
    }

    fn apply_lua_director(&mut self, lua: LuaDirector) {
        if self.data != lua.data {
            self.data = lua.data;
            self.tasks.push(LuaDirectorTask::SendVariables {});
        }
        if self.bosses != lua.bosses {
            self.bosses = lua.bosses;
        }
        self.tasks.extend_from_slice(&lua.tasks);
    }

    /// Actually insert tasks to seal the boss wall.
    pub fn seal_boss_wall(&mut self, id: u32, place_name: u32) {
        self.tasks.push(LuaDirectorTask::LogMessage {
            id: 2013,
            params: vec![place_name],
        });
        self.tasks.push(LuaDirectorTask::ShowEObj { base_id: id });
    }

    /// Actually insert tasks to unseal the boss wall.
    pub fn unseal_boss_wall(&mut self, wall_id: u32, line_id: u32, place_name: u32) {
        self.tasks
            .push(LuaDirectorTask::HideEObj { base_id: wall_id });
        self.tasks
            .push(LuaDirectorTask::HideEObj { base_id: line_id });
        self.tasks.push(LuaDirectorTask::LogMessage {
            id: 2014,
            params: vec![place_name],
        });
    }

    pub fn on_actor_aggro(&mut self, id: u32) {
        if let Some(boss) = self.bosses.get(&id) {
            // TODO: is there times that are longer than 15 secs?
            self.tasks.push(LuaDirectorTask::LogMessage {
                id: 2012,
                params: vec![boss.place_name, 15],
            });
            self.tasks.push(LuaDirectorTask::SealBossWall {
                actor_id: boss.actor_id,
                id: boss.wall_id,
                place_name: boss.place_name,
                time_until: 15,
            });
        }
    }

    pub fn on_gimmick_rect(&mut self, id: u32) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onGimmickRect")?;

                func.call::<()>((data, id))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onGimmickRect: {err:?}");
        }
    }
}

/// Perform any queued director tasks
pub fn director_tick(network: Arc<Mutex<NetworkState>>, instance: &mut Instance) {
    let tasks = if let Some(director) = &instance.director {
        director.tasks.clone()
    } else {
        return;
    };

    let mut bosses = if let Some(director) = &instance.director {
        director.bosses.clone()
    } else {
        return;
    };

    let director_id = instance.director.as_ref().unwrap().id;

    for task in &tasks {
        match task {
            LuaDirectorTask::HideEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj {base_id} for HideEObj, it won't despawn!");
                    continue;
                };

                let flags =
                    InvisibilityFlags::UNK1 | InvisibilityFlags::UNK2 | InvisibilityFlags::UNK3;

                let mut network = network.lock();
                network.send_ac_in_range_instance(
                    instance,
                    actor_id,
                    ActorControlCategory::SetInvisibilityFlags { flags },
                );

                // Update invisibility flags for next spawn
                if let Some(NetworkedActor::Object { object }) = instance.find_actor_mut(actor_id) {
                    object.visibility = flags;
                }
            }
            LuaDirectorTask::ShowEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj {base_id} for ShowEObj, it won't despawn!");
                    continue;
                };

                let flags = InvisibilityFlags::VISIBLE;

                let mut network = network.lock();
                network.send_ac_in_range_instance(
                    instance,
                    actor_id,
                    ActorControlCategory::SetInvisibilityFlags { flags },
                );

                // Update invisibility flags for next spawn
                if let Some(NetworkedActor::Object { object }) = instance.find_actor_mut(actor_id) {
                    object.visibility = flags;
                }
            }
            LuaDirectorTask::DeleteEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!(
                        "Failed to find eobj {base_id} for DeleteEObj, it won't despawn!"
                    );
                    continue;
                };

                let mut network = network.lock();
                network.remove_actor(instance, actor_id);
            }
            LuaDirectorTask::SpawnEObj { base_id, position } => {
                if let Some(mut object) = instance.zone.get_event_object(*base_id) {
                    if let Some(position) = position {
                        object.position = *position;
                    }
                    instance.insert_object(object.entity_id, object);
                } else {
                    tracing::warn!("Failed to find eobj {base_id} for SpawnEObj, it won't spawn!");
                }
            }
            LuaDirectorTask::SendVariables => {
                let vars = if let Some(director) = &instance.director {
                    director.build_var_segment()
                } else {
                    panic!("There's no way this could've happened!");
                };

                let mut network = network.lock();
                for id in instance.actors.keys() {
                    let Some((handle, _)) = network.get_by_actor_mut(*id) else {
                        continue;
                    };

                    let msg = FromServer::PacketSegment(vars.clone(), *id);
                    let _ = handle.send(msg.clone()); // TODO: use result
                }
            }
            LuaDirectorTask::AbandonDuty { actor_id } => {
                let mut network = network.lock();
                network.send_to_by_actor_id(
                    *actor_id,
                    FromServer::LeaveContent(),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::BeginEventAction {
                actor_id,
                target,
                action_id,
            } => {
                let act = ActorControlCategory::EventAction {
                    unk1: 1,
                    id: *action_id,
                };

                let mut network = network.lock();
                network.send_to_by_actor_id(
                    *actor_id,
                    FromServer::ActorControlTarget(
                        *actor_id,
                        ObjectTypeId {
                            object_id: *target,
                            object_type: ObjectTypeKind::None,
                        },
                        act,
                    ),
                    DestinationNetwork::ZoneClients,
                );

                // TODO: set OccupiedInEvent?

                // TODO: don't hardcode this duration, take it from the EventAction sheet!
                instance.insert_task(
                    ClientId::default(),
                    *actor_id,
                    Duration::from_secs(2),
                    QueuedTaskData::CastEventAction { target: *target },
                );
            }
            LuaDirectorTask::FinishGimmickEvent { actor_id } => {
                let mut network = network.lock();
                network.send_to_by_actor_id(
                    *actor_id,
                    FromServer::FinishEvent(),
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::LogMessage { id, params } => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LogMessage {
                    handler_id: director_id,
                    message_type: *id,
                    params_count: params.len() as u32,
                    item_id: params.first().copied().unwrap_or_default(),
                    item_quantity: params.get(1).copied().unwrap_or_default(),
                });

                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SpawnBattleNpc { id } => {
                if let Some(mut npc) = instance.zone.get_battle_npc(*id) {
                    npc.common.handler_id = director_id;
                    instance.insert_npc(ObjectId(fastrand::u32(..)), npc);
                } else {
                    tracing::warn!("Failed to find bnpc {id} for SpawnBattleNpc, it won't spawn!");
                }
            }
            LuaDirectorTask::GainEffect {
                actor_id,
                id,
                param,
                duration,
            } => {
                gain_effect_instance(
                    network.clone(),
                    ClientId::default(),
                    instance,
                    *actor_id,
                    *id,
                    *param,
                    *duration,
                    ObjectId::default(),
                    false,
                );
            }
            LuaDirectorTask::SetBGM { id } => {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                    ActorControlSelf {
                        category: ActorControlCategory::DirectorEvent {
                            handler_id: director_id,
                            event: DirectorEvent::SetBGM,
                            arg: *id,
                            unk1: 0,
                        },
                    },
                ));

                let mut network = network.lock();
                network.send_to_instance(
                    ObjectId::default(),
                    instance,
                    FromServer::PacketSegment(ipc, ObjectId::default()), // TODO: how do we just send it from the player?
                    DestinationNetwork::ZoneClients,
                );
            }
            LuaDirectorTask::SealBossWall {
                actor_id,
                id,
                place_name,
                time_until,
            } => {
                instance.insert_task(
                    ClientId::default(),
                    *actor_id,
                    Duration::from_secs(*time_until as u64),
                    QueuedTaskData::SealBossWall {
                        id: *id,
                        place_name: *place_name,
                    },
                );
            }
            LuaDirectorTask::SpawnBoss {
                bnpc_id,
                wall_id,
                line_id,
                place_name,
            } => {
                if let Some(mut npc) = instance.zone.get_battle_npc(*bnpc_id) {
                    npc.common.handler_id = director_id;

                    let actor_id = ObjectId(fastrand::u32(..));
                    instance.insert_npc(actor_id, npc);
                    bosses.insert(
                        *bnpc_id,
                        DirectorBoss {
                            actor_id,
                            wall_id: *wall_id,
                            line_id: *line_id,
                            place_name: *place_name,
                        },
                    );
                } else {
                    tracing::warn!("Failed to find bnpc {bnpc_id} for SpawnBoss, it won't spawn!");
                }
            }
        }
    }

    if let Some(director) = &mut instance.director {
        director.tasks.clear();
        director.bosses = bosses;
    }
}

/// Process director-related messages.
pub fn handle_director_messages(data: Arc<Mutex<WorldServer>>, msg: &ToServer) -> bool {
    match msg {
        ToServer::GimmickAccessor(from_actor_id, from_object_id, params) => {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
                tracing::warn!("Somehow failed to find an instance for actor?");
                return true;
            };

            let id = instance.find_base_id_by_actor_id(*from_object_id).unwrap();

            if let Some(director) = &mut instance.director {
                director.gimmick_accessor(*from_actor_id, id, params);
            } else {
                tracing::warn!("Expected a director when recieving a GimmickAccessor?");
            }

            true
        }
        _ => false,
    }
}
