use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::{net::TcpStream, sync::mpsc::Sender};

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
    ipc::{
        ActorControlSelf, ActorMove, ActorSetPos, ClientZoneIpcSegment, CommonSpawn, ContainerInfo,
        ContainerType, DisplayFlag, Equip, InitZone, ItemInfo, NpcSpawn, ObjectKind, PlayerSubKind,
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
    pub inventory: Inventory,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ClientId(usize);

pub enum FromServer {
    /// A chat message.
    Message(String),
    /// An actor has been spawned.
    ActorSpawn(Actor, CommonSpawn),
    /// An actor moved to a new position.
    ActorMove(u32, Position, f32),
}

#[derive(Debug, Clone)]
pub struct ClientHandle {
    pub id: ClientId,
    pub ip: SocketAddr,
    pub channel: Sender<FromServer>,
    // TODO: restore, i guess
    //pub kill: JoinHandle<()>,
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
    ActorSpawned(ClientId, Actor, CommonSpawn),
    ActorMoved(ClientId, u32, Position, f32),
    ZoneLoaded(ClientId),
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

    pub async fn send_chat_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Chat,
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

        match connection_type {
            ConnectionType::Zone => {
                tracing::info!("Client {actor_id} is initializing zone session...");

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

                // We have send THEM a keep alive
                {
                    self.send_chat_segment(PacketSegment {
                        source_actor: 0,
                        target_actor: 0,
                        segment_type: SegmentType::KeepAlive {
                            id: 0xE0037603u32,
                            timestamp: timestamp_secs(),
                        },
                    })
                    .await;
                }

                {
                    self.send_chat_segment(PacketSegment {
                        source_actor: 0,
                        target_actor: 0,
                        segment_type: SegmentType::ZoneInitialize {
                            player_id: self.player_data.actor_id,
                            timestamp: timestamp_secs(),
                        },
                    })
                    .await;
                }

                // we need the actor id at this point!
                assert!(self.player_data.actor_id != 0);

                {
                    let ipc = ServerZoneIpcSegment {
                        op_code: ServerZoneIpcType::InitializeChat,
                        timestamp: timestamp_secs(),
                        data: ServerZoneIpcData::InitializeChat { unk: [0; 8] },
                        ..Default::default()
                    };

                    self.send_chat_segment(PacketSegment {
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

    pub async fn set_actor_position(&mut self, actor_id: u32, position: Position, rotation: f32) {
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

    pub async fn spawn_actor(&mut self, actor: Actor, mut common: CommonSpawn) {
        // There is no reason for us to spawn our own player again. It's probably a bug!'
        assert!(actor.id.0 != self.player_data.actor_id);

        common.spawn_index = self.get_free_spawn_index();

        let ipc = ServerZoneIpcSegment {
            unk1: 20,
            unk2: 0,
            op_code: ServerZoneIpcType::NpcSpawn,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                common,
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

    pub async fn update_class_info(&mut self) {
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

    pub async fn change_zone(&mut self, new_zone_id: u16) {
        {
            let mut game_data = self.gamedata.lock().unwrap();
            self.zone = Some(Zone::load(&mut game_data.game_data, new_zone_id));
        }
        self.player_data.zone_id = new_zone_id;

        // Player Class Info
        self.update_class_info().await;

        // link shell information
        /*{
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
        }*/

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

    pub async fn send_inventory(&mut self, send_appearance_update: bool) {
        // page 1
        {
            let extra_slot = self.player_data.inventory.extra_slot;

            let mut send_slot = async |slot_index: u16, item: &Item| {
                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::ItemInfo,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::ItemInfo(ItemInfo {
                        container: ContainerType::Inventory0,
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

            send_slot(0, &extra_slot).await;
        }

        // equipped
        {
            let equipped = self.player_data.inventory.equipped;

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

        // inform the client of page 1
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::ContainerInfo,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::ContainerInfo(ContainerInfo {
                    container: ContainerType::Inventory0,
                    num_items: 1,
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

        // inform the client they have items equipped
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::ContainerInfo,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::ContainerInfo(ContainerInfo {
                    container: ContainerType::Equipped,
                    num_items: self.player_data.inventory.equipped.num_items(),
                    sequence: 1,
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

        // send them an appearance update
        if send_appearance_update {
            let ipc;
            {
                let mut game_data = self.gamedata.lock().unwrap();
                let equipped = &self.player_data.inventory.equipped;

                ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::Equip,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::Equip(Equip {
                        main_weapon_id: 0,
                        sub_weapon_id: 0,
                        crest_enable: 0,
                        pattern_invalid: 0,
                        model_ids: [
                            game_data.get_primary_model_id(equipped.head.id) as u32,
                            game_data.get_primary_model_id(equipped.body.id) as u32,
                            game_data.get_primary_model_id(equipped.hands.id) as u32,
                            game_data.get_primary_model_id(equipped.legs.id) as u32,
                            game_data.get_primary_model_id(equipped.feet.id) as u32,
                            game_data.get_primary_model_id(equipped.ears.id) as u32,
                            game_data.get_primary_model_id(equipped.neck.id) as u32,
                            game_data.get_primary_model_id(equipped.wrists.id) as u32,
                            game_data.get_primary_model_id(equipped.left_ring.id) as u32,
                            game_data.get_primary_model_id(equipped.right_ring.id) as u32,
                        ],
                    }),
                    ..Default::default()
                };
            }

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

        None
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

    pub fn get_player_common_spawn(
        &self,
        exit_position: Option<Position>,
        exit_rotation: Option<f32>,
    ) -> CommonSpawn {
        let mut game_data = self.gamedata.lock().unwrap();

        let chara_details = self.database.find_chara_make(self.player_data.content_id);

        let equipped = &self.player_data.inventory.equipped;
        CommonSpawn {
            class_job: self.player_data.classjob_id,
            name: chara_details.name,
            hp_curr: self.player_data.curr_hp,
            hp_max: self.player_data.max_hp,
            mp_curr: self.player_data.curr_mp,
            mp_max: self.player_data.max_mp,
            level: self.player_data.level,
            object_kind: ObjectKind::Player(PlayerSubKind::Player),
            look: chara_details.chara_make.customize,
            display_flags: DisplayFlag::UNK,
            models: [
                game_data.get_primary_model_id(equipped.head.id) as u32,
                game_data.get_primary_model_id(equipped.body.id) as u32,
                game_data.get_primary_model_id(equipped.hands.id) as u32,
                game_data.get_primary_model_id(equipped.legs.id) as u32,
                game_data.get_primary_model_id(equipped.feet.id) as u32,
                game_data.get_primary_model_id(equipped.ears.id) as u32,
                game_data.get_primary_model_id(equipped.neck.id) as u32,
                game_data.get_primary_model_id(equipped.wrists.id) as u32,
                game_data.get_primary_model_id(equipped.left_ring.id) as u32,
                game_data.get_primary_model_id(equipped.right_ring.id) as u32,
            ],
            pos: exit_position.unwrap_or(Position::default()),
            rotation: exit_rotation.unwrap_or(0.0),
            ..Default::default()
        }
    }
}
