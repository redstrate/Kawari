use std::sync::Arc;

use kawari::{
    common::{
        FATE_TIME_LIMIT, FateRule, FateState, HandlerId, HandlerType, ObjectId, Position,
        timestamp_secs,
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
    FromServer, GameData,
    lua::KawariLua,
    server::{
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
    },
};

#[derive(Debug, Clone)]
pub struct FateInstance {
    /// Index into the Fate Excel sheet.
    pub fate_id: u32,
    /// When this FATE was started.
    pub start_timestamp: u32,
    /// The current state of the FATE.
    pub fate_state: FateState,
    pub data: FateData,
    /// The start NPC for this FATE, if applicable.
    pub motivation_npc: Option<(ObjectId, Position)>,
}

impl FateInstance {
    pub fn new(fate_id: u32, game_data: &mut GameData) -> Option<Self> {
        let fate_rule = game_data.get_fate_rule(fate_id).unwrap_or_default();
        let fate_state = match fate_rule {
            FateRule::Invalid => return None,
            FateRule::Gathering => FateState::Preparing,
            _ => FateState::Running,
        };

        // Setup Lua state
        let lua = KawariLua::new();

        // Find the script for this FATE
        let file_name = get_config()
            .filesystem
            .locate_script_file("content/test_fate_soul.lua");

        let mut data = FateData::default();

        // HACK: hardcoded to this FATE for now
        if fate_id == 603 {
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
                    data.lua = lua;

                    // Call into the onSetup function before returning, as we need the flag to be initialized before any players change zones.
                    data.setup();
                }
            }
        }

        Some(Self {
            fate_id,
            start_timestamp: timestamp_secs(),
            fate_state,
            data,
            motivation_npc: None,
        })
    }

    pub fn create_unk_fate_packet(&self) -> ServerZoneIpcSegment {
        ServerZoneIpcSegment::new(ServerZoneIpcData::UnkFate {
            fate_id: self.fate_id,
            unk1: 0,
            start_timestamp: self.start_timestamp,
            unk3: 0,
            time_limit: FATE_TIME_LIMIT.as_secs() as u32,
            unk5: 0,
        })
    }

    pub fn create_fate_init_ac(&self) -> ActorControlCategory {
        ActorControlCategory::FateInit {
            fate_id: self.fate_id,
            fate_state: self.fate_state,
        }
    }
}

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
                                    x: position.0.x,
                                    y: position.0.y,
                                    z: position.0.z,
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

pub fn inform_fate_spawn(network: &mut NetworkState, from_actor_id: ObjectId, fate: &FateInstance) {
    let send_motivation_npc = |network: &mut NetworkState| {
        if let Some((object_id, position)) = fate.motivation_npc {
            network.send_to_by_actor_id(
                from_actor_id,
                FromServer::ActorControlSelf(ActorControlCategory::SetupMotivationNpc {
                    fate_id: fate.fate_id,
                    motivation_npc: object_id,
                    unk1: 2175,
                    x: position.0.x,
                    y: position.0.y,
                    z: position.0.z,
                }),
                DestinationNetwork::ZoneClients,
            );
        }
    };

    // TODO: maybe only for newly spawned fates?
    network.send_to_by_actor_id(
        from_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::FateInit {
            fate_id: fate.fate_id,
            fate_state: FateState::Unk1,
        }),
        DestinationNetwork::ZoneClients,
    );

    send_motivation_npc(network);

    network.send_to_by_actor_id(
        from_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::CreateFateContext {
            fate_id: fate.fate_id,
            is_bonus: 0,
        }),
        DestinationNetwork::ZoneClients,
    );

    if fate.fate_state == FateState::Running {
        network.send_to_by_actor_id(
            from_actor_id,
            FromServer::PacketSegment(fate.create_unk_fate_packet(), from_actor_id),
            DestinationNetwork::ZoneClients,
        );
    }

    network.send_to_by_actor_id(
        from_actor_id,
        FromServer::ActorControlSelf(fate.create_fate_init_ac()),
        DestinationNetwork::ZoneClients,
    );

    // Yes, retail does send it twice.
    send_motivation_npc(network);

    network.send_to_by_actor_id(
        from_actor_id,
        FromServer::ActorControlSelf(ActorControlCategory::FateUpdateTargetableStatus {
            fate_id: fate.fate_id,
        }),
        DestinationNetwork::ZoneClients,
    );
}

pub fn inform_fate_spawn_globally(
    instance: &mut Instance,
    network: &mut NetworkState,
    fate: &FateInstance,
) {
    for actor in instance.actors.keys() {
        inform_fate_spawn(network, *actor, fate);
    }
}

/// Move a FATE from Preparing to Running.
pub fn start_fate(fate: &mut FateInstance) {
    fate.fate_state = FateState::Running;
    fate.start_timestamp = timestamp_secs();
}

/// Move a FATE to Ending.
pub fn end_fate(fate: &mut FateInstance) {
    fate.fate_state = FateState::Ending;
}

/// Move a FATE to Ended.
pub fn ended_fate(fate: &mut FateInstance) {
    fate.fate_state = FateState::Ended;
}

/// Move a FATE to Unk10.
pub fn unk10_fate(fate: &mut FateInstance) {
    fate.fate_state = FateState::Unk10;
}
