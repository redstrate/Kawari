use std::sync::Arc;

use kawari::{
    common::{HandlerId, InvisibilityFlags, ObjectId},
    ipc::zone::{ActorControlCategory, ServerZoneIpcData, ServerZoneIpcSegment},
};
use mlua::{Function, UserData, UserDataMethods};
use parking_lot::Mutex;

use crate::{
    FromServer, ToServer,
    lua::KawariLua,
    server::{
        WorldServer,
        actor::NetworkedActor,
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum LuaDirectorTask {
    HideEObj { base_id: u32 },
    ShowEObj { base_id: u32 },
    DeleteEObj { base_id: u32 },
    SpawnEObj { base_id: u32 },
    SendVariables,
    AbandonDuty { actor_id: ObjectId },
}

// TODO: Maybe collapse into DirectorData?
#[derive(Default, Debug)]
pub struct LuaDirector {
    pub data: [u8; 10],
    pub tasks: Vec<LuaDirectorTask>,
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
        methods.add_method_mut("spawn_eobj", |_, this, base_id: u32| {
            this.tasks.push(LuaDirectorTask::SpawnEObj { base_id });
            Ok(())
        });
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
    }
}

#[derive(Debug)]
pub struct DirectorData {
    pub id: HandlerId,
    pub flag: u8,
    pub data: [u8; 10],
    /// Lua state for this director.
    pub lua: KawariLua,
    pub tasks: Vec<LuaDirectorTask>,
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

    pub fn gimmick_accessor(&mut self, actor_id: ObjectId, id: u32) {
        let mut run_script = || {
            let mut lua_director = self.create_lua_director();
            let err = self.lua.0.scope(|scope| {
                let data = scope.create_userdata_ref_mut(&mut lua_director)?;

                let func: Function = self.lua.0.globals().get("onGimmickAccessor")?;

                func.call::<()>((data, actor_id.0, id))?;

                Ok(())
            });
            self.apply_lua_director(lua_director);
            err
        };
        if let Err(err) = run_script() {
            tracing::warn!("Syntax error during onGimmickAccessor: {err:?}");
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
        }
    }

    fn apply_lua_director(&mut self, lua: LuaDirector) {
        if self.data != lua.data {
            self.data = lua.data;
            self.tasks.push(LuaDirectorTask::SendVariables {});
        }
        self.tasks.extend_from_slice(&lua.tasks);
    }
}

pub fn director_tick(network: Arc<Mutex<NetworkState>>, instance: &mut Instance) {
    // Perform any queued director tasks
    let tasks = if let Some(director) = &instance.director {
        director.tasks.clone()
    } else {
        Vec::new()
    };

    for task in &tasks {
        match task {
            LuaDirectorTask::HideEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj for HideEObj, it won't despawn!");
                    continue;
                };

                let flags =
                    InvisibilityFlags::UNK1 | InvisibilityFlags::UNK2 | InvisibilityFlags::UNK3;

                let msg = FromServer::ActorControl(
                    actor_id,
                    ActorControlCategory::SetInvisibilityFlags { flags },
                );

                let mut network = network.lock();
                for id in instance.actors.keys() {
                    let Some((handle, _)) = network.get_by_actor_mut(*id) else {
                        continue;
                    };

                    let _ = handle.send(msg.clone()); // TODO: use result
                }

                // Update invisibility flags for next spawn
                if let Some(NetworkedActor::Object { object }) = instance.find_actor_mut(actor_id) {
                    object.visibility = flags;
                }
            }
            LuaDirectorTask::ShowEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj for ShowEObj, it won't despawn!");
                    continue;
                };

                let flags = InvisibilityFlags::VISIBLE;

                let msg = FromServer::ActorControl(
                    actor_id,
                    ActorControlCategory::SetInvisibilityFlags { flags },
                );

                let mut network = network.lock();
                for id in instance.actors.keys() {
                    let Some((handle, _)) = network.get_by_actor_mut(*id) else {
                        continue;
                    };

                    let _ = handle.send(msg.clone()); // TODO: use result
                }

                // Update invisibility flags for next spawn
                if let Some(NetworkedActor::Object { object }) = instance.find_actor_mut(actor_id) {
                    object.visibility = flags;
                }
            }
            LuaDirectorTask::DeleteEObj { base_id } => {
                let Some(actor_id) = instance.find_object_by_eobj_id(*base_id) else {
                    tracing::warn!("Failed to find eobj for DeleteEObj, it won't despawn!");
                    continue;
                };

                let mut network = network.lock();
                network.remove_actor(instance, actor_id);
            }
            LuaDirectorTask::SpawnEObj { base_id } => {
                if let Some(object) = instance.zone.get_event_object(*base_id) {
                    instance.insert_object(object.entity_id, object);
                } else {
                    tracing::warn!("Failed to find eobj for SpawnEObj, it won't spawn!");
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
        }
    }

    if let Some(director) = &mut instance.director {
        director.tasks.clear();
    }
}

/// Process status effect-related messages.
pub fn handle_director_messages(data: Arc<Mutex<WorldServer>>, msg: &ToServer) -> bool {
    match msg {
        ToServer::GimmickAccessor(from_actor_id, id) => {
            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
                tracing::warn!("Somehow failed to find an instance for actor?");
                return true;
            };

            if let Some(director) = &mut instance.director {
                director.gimmick_accessor(*from_actor_id, *id);
            } else {
                tracing::warn!("Expected a director when recieving a GimmickAccessor?");
            }

            true
        }
        _ => false,
    }
}
