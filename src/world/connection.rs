use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::{net::TcpStream, sync::mpsc::Sender, task::JoinHandle};

use crate::{
    common::{GameData, ObjectId, Position, timestamp_secs},
    opcodes::ServerZoneIpcType,
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentType, parse_packet,
        send_packet,
    },
};

use super::{
    Actor, Event, Inventory, Item, LuaPlayer, StatusEffects, WorldDatabase, Zone,
    chat_handler::CUSTOMIZE_DATA,
    ipc::{
        ActorControlSelf, ActorMove, ActorSetPos, BattleNpcSubKind, ClientZoneIpcSegment,
        CommonSpawn, ContainerInfo, ContainerType, InitZone, ItemInfo, NpcSpawn, ObjectKind,
        ServerZoneIpcData, ServerZoneIpcSegment, StatusEffect, StatusEffectList, UpdateClassInfo,
        WeatherChange,
    },
};

#[derive(Debug, Default, Clone, Copy)]
pub struct PlayerData {
    // Static data
    pub actor_id: u32,
    pub content_id: u64,
    pub account_id: u32,

    pub classjob_id: u8,
    pub level: u8,
    pub curr_hp: u32,
    pub max_hp: u32,
    pub curr_mp: u16,
    pub max_mp: u16,

    // Dynamic data
    pub position: Position,
    /// In radians.
    pub rotation: f32,
    pub zone_id: u16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

pub enum FromServer {
    /// A chat message.
    Message(String),
    /// An actor has been spawned.
    ActorSpawn(Actor),
    /// An actor moved to a new position.
    ActorMove(u32, Position),
}

#[derive(Debug)]
pub struct ClientHandle {
    pub id: ClientId,
    pub ip: SocketAddr,
    pub channel: Sender<FromServer>,
    pub kill: JoinHandle<()>,
}

impl ClientHandle {
    /// Send a message to this client actor. Will emit an error if sending does
    /// not succeed immediately, as this means that forwarding messages to the
    /// tcp connection cannot keep up.
    pub fn send(&mut self, msg: FromServer) -> Result<(), std::io::Error> {
        if self.channel.try_send(msg).is_err() {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "Can't keep up or dead",
            ))
        } else {
            Ok(())
        }
    }

    /// Kill the actor.
    pub fn kill(self) {
        // run the destructor
        drop(self);
    }
}

pub enum ToServer {
    NewClient(ClientHandle),
    Message(ClientId, String),
    ActorSpawned(ClientId, Actor),
    ActorMoved(ClientId, u32, Position),
    FatalError(std::io::Error),
}

#[derive(Clone, Debug)]
pub struct ServerHandle {
    pub chan: Sender<ToServer>,
    pub next_id: Arc<AtomicUsize>,
}

impl ServerHandle {
    pub async fn send(&mut self, msg: ToServer) {
        if self.chan.send(msg).await.is_err() {
            panic!("Main loop has shut down.");
        }
    }
    pub fn next_id(&self) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        ClientId(id)
    }
}

/// Represents a single connection between an instance of the client and the world server
pub struct ZoneConnection {
    pub socket: TcpStream,

    pub state: PacketState,
    pub player_data: PlayerData,

    pub zone: Option<Zone>,
    pub spawn_index: u8,

    pub inventory: Inventory,
    pub status_effects: StatusEffects,

    pub event: Option<Event>,
    pub actors: Vec<Actor>,

    pub ip: SocketAddr,
    pub id: ClientId,
    pub handle: ServerHandle,

    pub database: Arc<WorldDatabase>,
    pub lua: Arc<Mutex<mlua::Lua>>,
    pub gamedata: Arc<Mutex<GameData>>,
}

impl ZoneConnection {
    pub async fn parse_packet(
        &mut self,
        data: &[u8],
    ) -> (Vec<PacketSegment<ClientZoneIpcSegment>>, ConnectionType) {
        parse_packet(data, &mut self.state).await
    }

    pub async fn send_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Zone,
            CompressionType::Oodle,
            &[segment],
        )
        .await;
    }

    pub async fn initialize(&mut self, connection_type: &ConnectionType, actor_id: u32) {
        // some still hardcoded values
        self.player_data.classjob_id = 1;
        self.player_data.level = 5;
        self.player_data.curr_hp = 100;
        self.player_data.max_hp = 100;
        self.player_data.curr_mp = 10000;
        self.player_data.max_mp = 10000;

        // We have send THEM a keep alive
        {
            self.send_segment(PacketSegment {
                source_actor: 0,
                target_actor: 0,
                segment_type: SegmentType::KeepAlive {
                    id: 0xE0037603u32,
                    timestamp: timestamp_secs(),
                },
            })
            .await;
        }

        match connection_type {
            ConnectionType::Zone => {
                tracing::info!("Client {actor_id} is initializing zone session...");

                self.send_segment(PacketSegment {
                    source_actor: 0,
                    target_actor: 0,
                    segment_type: SegmentType::ZoneInitialize {
                        player_id: self.player_data.actor_id,
                        timestamp: timestamp_secs(),
                    },
                })
                .await;
            }
            ConnectionType::Chat => {
                tracing::info!("Client {actor_id} is initializing chat session...");

                {
                    self.send_segment(PacketSegment {
                        source_actor: 0,
                        target_actor: 0,
                        segment_type: SegmentType::ZoneInitialize {
                            player_id: self.player_data.actor_id,
                            timestamp: timestamp_secs(),
                        },
                    })
                    .await;
                }

                {
                    let ipc = ServerZoneIpcSegment {
                        op_code: ServerZoneIpcType::InitializeChat,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::InitializeChat { unk: [0; 8] },
                        ..Default::default()
                    };

                    self.send_segment(PacketSegment {
                        source_actor: self.player_data.actor_id,
                        target_actor: self.player_data.actor_id,
                        segment_type: SegmentType::Ipc { data: ipc },
                    })
                    .await;
                }
            }
            _ => panic!("The client is trying to initialize the wrong connection?!"),
        }
    }

    pub async fn set_player_position(&mut self, position: Position) {
        // set pos
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::ActorSetPos,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::ActorSetPos(ActorSetPos {
                    unk: 0x020fa3b8,
                    position,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn set_actor_position(&mut self, actor_id: u32, position: Position) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::ActorMove,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::ActorMove(ActorMove {
                speed: 24,
                position,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn spawn_actor(&mut self, actor: Actor) {
        let ipc = ServerZoneIpcSegment {
            unk1: 20,
            unk2: 0,
            op_code: ServerZoneIpcType::NpcSpawn,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                common: CommonSpawn {
                    hp_curr: 100,
                    hp_max: 100,
                    mp_curr: 100,
                    mp_max: 100,
                    look: CUSTOMIZE_DATA,
                    spawn_index: self.get_free_spawn_index(),
                    bnpc_base: 13498,
                    bnpc_name: 10261,
                    object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                    level: 1,
                    models: [
                        0,  // head
                        89, // body
                        89, // hands
                        89, // legs
                        89, // feet
                        0,  // ears
                        0,  // neck
                        0,  // wrists
                        0,  // left finger
                        0,  // right finger
                    ],
                    ..Default::default()
                },
                ..Default::default()
            }),
        };

        self.send_segment(PacketSegment {
            source_actor: actor.id.0,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn change_zone(&mut self, new_zone_id: u16) {
        self.zone = Some(Zone::load(new_zone_id));
        self.player_data.zone_id = new_zone_id;

        // Player Class Info
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::UpdateClassInfo,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                    class_id: self.player_data.classjob_id as u16,
                    unknown: 1,
                    synced_level: self.player_data.level as u16,
                    class_level: self.player_data.level as u16,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // link shell information
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::LinkShellInformation,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::LinkShellInformation { unk: [0; 456] },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // TODO: send unk16?

        // Init Zone
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::InitZone,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::InitZone(InitZone {
                    server_id: 0,
                    zone_id: self.zone.as_ref().unwrap().id,
                    weather_id: 1,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn change_weather(&mut self, new_weather_id: u16) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::WeatherChange,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::WeatherChange(WeatherChange {
                weather_id: new_weather_id,
                transistion_time: 1.0,
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    pub fn get_free_spawn_index(&mut self) -> u8 {
        self.spawn_index += 1;
        self.spawn_index
    }

    pub async fn send_inventory(&mut self) {
        // item list
        {
            let equipped = self.inventory.equipped;

            let mut send_slot = async |slot_index: u16, item: &Item| {
                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::ItemInfo,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::ItemInfo(ItemInfo {
                        container: ContainerType::Equipped,
                        slot: slot_index,
                        quantity: item.quantity,
                        catalog_id: item.id,
                        condition: 30000,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                self.send_segment(PacketSegment {
                    source_actor: self.player_data.actor_id,
                    target_actor: self.player_data.actor_id,
                    segment_type: SegmentType::Ipc { data: ipc },
                })
                .await;
            };

            send_slot(0, &equipped.main_hand).await;
            send_slot(1, &equipped.off_hand).await;
            send_slot(2, &equipped.head).await;
            send_slot(3, &equipped.body).await;
            send_slot(4, &equipped.hands).await;
            send_slot(6, &equipped.legs).await;
            send_slot(7, &equipped.feet).await;
            send_slot(8, &equipped.ears).await;
            send_slot(9, &equipped.neck).await;
            send_slot(10, &equipped.wrists).await;
            send_slot(11, &equipped.right_ring).await;
            send_slot(12, &equipped.left_ring).await;
            send_slot(13, &equipped.soul_crystal).await;
        }

        // inform the client they have items equipped
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::ContainerInfo,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::ContainerInfo(ContainerInfo {
                    container: ContainerType::Equipped,
                    num_items: self.inventory.equipped.num_items(),
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn send_message(&mut self, message: &str) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::ServerChatMessage,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::ServerChatMessage {
                message: message.to_string(),
                unk: 0,
            },
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn process_lua_player(&mut self, player: &mut LuaPlayer) {
        for segment in &player.queued_segments {
            self.send_segment(segment.clone()).await;
        }
        player.queued_segments.clear();

        for task in &player.queued_tasks {
            self.change_zone(task.zone_id).await;
        }
        player.queued_tasks.clear();
    }

    pub async fn process_effects_list(&mut self) {
        // Only update the client if absolutely nessecary (e.g. an effect is added, removed or changed duration)
        if self.status_effects.dirty {
            let mut list = [StatusEffect::default(); 30];
            list[..self.status_effects.status_effects.len()]
                .copy_from_slice(&self.status_effects.status_effects);

            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::StatusEffectList,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::StatusEffectList(StatusEffectList {
                    statues: list,
                    classjob_id: self.player_data.classjob_id,
                    level: self.player_data.level,
                    curr_hp: self.player_data.curr_hp,
                    max_hp: self.player_data.max_hp,
                    curr_mp: self.player_data.curr_mp,
                    max_mp: self.player_data.max_mp,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;

            self.status_effects.dirty = false;
        }
    }

    pub async fn update_hp_mp(&mut self, actor_id: ObjectId, hp: u32, mp: u16) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::UpdateHpMpTp,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::UpdateHpMpTp { hp, mp, unk: 0 },
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: actor_id.0,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }

    pub fn add_actor(&mut self, actor: Actor) {
        self.actors.push(actor);
    }

    pub fn get_actor(&mut self, id: ObjectId) -> Option<&mut Actor> {
        for actor in &mut self.actors {
            if actor.id == id {
                return Some(actor);
            }
        }

        return None;
    }

    pub async fn actor_control_self(&mut self, actor_control: ActorControlSelf) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::ActorControlSelf,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::ActorControlSelf(actor_control),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        })
        .await;
    }
}
