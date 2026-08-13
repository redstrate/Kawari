use std::sync::Arc;

use kawari::{
    common::{
        HandlerId, HandlerType, ObjectId,
    },
    config::get_config,
    ipc::zone::{
        ActorControlCategory, ActorControlSelf, DisplayFlag, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};
use mlua::{Function, UserData, UserDataMethods};
use parking_lot::Mutex;

use crate::{
    FromServer,
    lua::KawariLua,
    server::{
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub enum LuaFateTask {
    SpawnBattleNpc { id: u32 },
    SetMotivationNpc { id: u32 },
}

// TODO: Maybe collapse into FateData?
#[derive(Default, Debug)]
pub struct LuaFate {
    pub tasks: Vec<LuaFateTask>,
}

impl UserData for LuaFate {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("spawn_bnpc", |_, this, id: u32| {
            this.tasks.push(LuaFateTask::SpawnBattleNpc { id });
            Ok(())
        });
        methods.add_method_mut("set_motivation_npc", |_, this, id: u32| {
            this.tasks.push(LuaFateTask::SetMotivationNpc { id });
            Ok(())
        });
    }
}

#[derive(Debug, Default, Clone)]
pub struct FateData {
    /// Lua state for this FATE.
    pub lua: KawariLua,
    pub tasks: Vec<LuaFateTask>,
}

impl FateData {
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

    fn create_lua_director(&self) -> LuaFate {
        LuaFate { tasks: Vec::new() }
    }

    fn apply_lua_director(&mut self, lua: LuaFate) {
        self.tasks.extend_from_slice(&lua.tasks);
    }
}

/// Perform any queued FATE tasks
pub fn fate_tick(network: Arc<Mutex<NetworkState>>, instance: &mut Instance) {
    let mut fates = instance.fates.clone();
    for fate in &mut fates {
        let tasks = fate.data.tasks.clone();
        let fate_id = fate.fate_id;

        for task in &tasks {
            match task {
                LuaFateTask::SpawnBattleNpc { id } => {
                    if let Some(mut npc) = instance.zone.get_battle_npc(*id) {
                        npc.common.fate_id = fate_id as u16;
                        npc.common.handler_id = HandlerId::new(HandlerType::Fate, 65535);
                        npc.common.display_flags = DisplayFlag::FATE_START_NPC;
                        let config = get_config();
                        instance.insert_npc(ObjectId(fastrand::u32(..)), npc, &config);
                    } else {
                        tracing::warn!(
                            "Failed to find bnpc {id} for SpawnBattleNpc, it won't spawn!"
                        );
                    }
                }
                LuaFateTask::SetMotivationNpc { id } => {
                    if let Some((object_id, position)) = instance.find_npc(*id) {
                        // TODO: consolidate this code with the one in Instance
                        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(
                            ActorControlSelf {
                                category: ActorControlCategory::SetupMotivationNpc {
                                    fate_id,
                                    motivation_npc: object_id,
                                    unk1: 2175,
                                    position_x: (position.0.x * 1000.0) as i32,
                                    position_y: (position.0.y * 1000.0) as i32,
                                    position_z: (position.0.z * 1000.0) as i32,
                                },
                            },
                        ));

                        let mut network = network.lock();
                        network.send_to_instance(
                            ObjectId::default(),
                            instance,
                            FromServer::PacketSegment(ipc, ObjectId::default()),
                            DestinationNetwork::ZoneClients,
                        );

                        // Set the motivation NPC for new players spawning in
                        fate.motivation_npc = Some((object_id, position));
                    } else {
                        tracing::warn!(
                            "Failed to find bnpc {id} for SetMotivationNpc, it won't work!"
                        );
                    }
                }
            }
        }
    }

    for (fate, new_fate) in instance.fates.iter_mut().zip(fates) {
        fate.data.tasks.clear();
        fate.motivation_npc = new_fate.motivation_npc;
    }
}
