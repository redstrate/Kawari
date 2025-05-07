use std::{
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use tokio::{net::TcpStream, sync::mpsc::Sender};

use crate::{
    OBFUSCATION_ENABLED_MODE,
    common::{GameData, ObjectId, Position, timestamp_secs},
    config::{WorldConfig, get_config},
    inventory::{Inventory, Item},
    ipc::{
        chat::ServerChatIpcSegment,
        zone::{
            ActorControlSelf, ClientZoneIpcSegment, CommonSpawn, ContainerInfo, DisplayFlag, Equip,
            InitZone, ItemInfo, Move, NpcSpawn, ObjectKind, PlayerStats, PlayerSubKind,
            ServerZoneIpcData, ServerZoneIpcSegment, StatusEffect, StatusEffectList,
            UpdateClassInfo, Warp, WeatherChange,
        },
    },
    opcodes::ServerZoneIpcType,
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentData, SegmentType,
        parse_packet, send_packet,
    },
};

use super::{
    Actor, CharacterData, Event, LuaPlayer, StatusEffects, WorldDatabase, Zone, lua::Task,
};

#[derive(Debug, Default, Clone)]
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
    // An actor has despawned.
    ActorDespawn(u32),
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
    ActorDespawned(ClientId, u32),
    ZoneLoaded(ClientId),
    Disconnected(ClientId),
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
    pub config: WorldConfig,
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

    pub exit_position: Option<Position>,
    pub exit_rotation: Option<f32>,
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
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        )
        .await;
    }

    pub async fn send_chat_segment(&mut self, segment: PacketSegment<ServerChatIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Chat,
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        )
        .await;
    }

    pub async fn initialize(&mut self, actor_id: u32) {
        // some still hardcoded values
        self.player_data.classjob_id = 1;
        self.player_data.level = 5;
        self.player_data.curr_hp = 100;
        self.player_data.max_hp = 100;
        self.player_data.curr_mp = 10000;
        self.player_data.max_mp = 10000;

        tracing::info!("Client {actor_id} is initializing zone session...");

        // We have send THEM a keep alive
        {
            self.send_segment(PacketSegment {
                segment_type: SegmentType::KeepAliveRequest,
                data: SegmentData::KeepAliveRequest {
                    id: 0xE0037603u32,
                    timestamp: timestamp_secs(),
                },
                ..Default::default()
            })
            .await;
        }

        self.send_segment(PacketSegment {
            segment_type: SegmentType::Initialize,
            data: SegmentData::Initialize {
                player_id: self.player_data.actor_id,
                timestamp: timestamp_secs(),
            },
            ..Default::default()
        })
        .await;
    }

    pub async fn set_player_position(&mut self, position: Position) {
        // set pos
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::Warp,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::Warp(Warp {
                    position,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn set_actor_position(&mut self, actor_id: u32, position: Position, rotation: f32) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::Move,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::Move(Move {
                rotation,
                dir_before_slip: 0x7F,
                flag1: 0,
                flag2: 0,
                speed: 0x3C,
                position,
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn spawn_actor(&mut self, mut actor: Actor, mut common: CommonSpawn) {
        // There is no reason for us to spawn our own player again. It's probably a bug!'
        assert!(actor.id.0 != self.player_data.actor_id);

        actor.spawn_index = self.get_free_spawn_index() as u32;
        common.spawn_index = actor.spawn_index as u8;

        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::NpcSpawn,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::NpcSpawn(NpcSpawn {
                common,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: actor.id.0,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;

        self.actors.push(actor);
    }

    pub async fn remove_actor(&mut self, actor_id: u32) {
        if let Some(actor) = self.get_actor(ObjectId(actor_id)).cloned() {
            tracing::info!("Removing actor {actor_id} {}!", actor.spawn_index);

            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::Delete,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::Delete {
                    spawn_index: actor.spawn_index as u8,
                    actor_id,
                },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: actor.id.0,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            })
            .await;

            self.actors.remove(
                self.actors
                    .iter()
                    .position(|actor| actor.id == ObjectId(actor_id))
                    .unwrap(),
            );
        }
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
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn change_zone(&mut self, new_zone_id: u16) {
        // tell everyone we're gone
        // TODO: check if we ever sent an initial ActorSpawn packet first, before sending this.
        // the connection already checks to see if the actor already exists, so it's seems harmless if we do
        self.handle
            .send(ToServer::ActorDespawned(self.id, self.player_data.actor_id))
            .await;

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
            let config = get_config();

            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::InitZone,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::InitZone(InitZone {
                    territory_type: self.zone.as_ref().unwrap().id,
                    weather_id: 1,
                    obsfucation_mode: if config.world.enable_packet_obsfucation {
                        OBFUSCATION_ENABLED_MODE
                    } else {
                        0
                    },
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn warp(&mut self, warp_id: u32) {
        let territory_type;
        // find the pop range on the other side
        {
            let mut game_data = self.gamedata.lock().unwrap();
            let (pop_range_id, zone_id) = game_data.get_warp(warp_id);

            let new_zone = Zone::load(&mut game_data.game_data, zone_id);

            // find it on the other side
            let (object, _) = new_zone.find_pop_range(pop_range_id).unwrap();

            // set the exit position
            self.exit_position = Some(Position {
                x: object.transform.translation[0],
                y: object.transform.translation[1],
                z: object.transform.translation[2],
            });

            territory_type = zone_id;
        }

        self.change_zone(territory_type as u16).await;
    }

    pub async fn change_weather(&mut self, new_weather_id: u16) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::WeatherId,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::WeatherId(WeatherChange {
                weather_id: new_weather_id,
                transistion_time: 1.0,
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub fn get_free_spawn_index(&mut self) -> u8 {
        self.spawn_index += 1;
        self.spawn_index
    }

    pub async fn send_inventory(&mut self, send_appearance_update: bool) {
        let mut sequence = 0;

        for (container_type, container) in &self.player_data.inventory.clone() {
            let mut send_slot = async |slot_index: u16, item: &Item| {
                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::UpdateItem,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::UpdateItem(ItemInfo {
                        sequence,
                        container: container_type,
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
                    segment_type: SegmentType::Ipc,
                    data: SegmentData::Ipc { data: ipc },
                })
                .await;
            };

            for i in 0..container.max_slots() {
                send_slot(i as u16, container.get_slot(i as u16)).await;
            }

            // inform the client of container state
            {
                let ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::ContainerInfo,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::ContainerInfo(ContainerInfo {
                        container: container_type,
                        num_items: container.num_items(),
                        sequence,
                        ..Default::default()
                    }),
                    ..Default::default()
                };

                self.send_segment(PacketSegment {
                    source_actor: self.player_data.actor_id,
                    target_actor: self.player_data.actor_id,
                    segment_type: SegmentType::Ipc,
                    data: SegmentData::Ipc { data: ipc },
                })
                .await;
            }

            sequence += 1;
        }

        // send them an appearance update
        if send_appearance_update {
            let ipc;
            {
                let mut game_data = self.gamedata.lock().unwrap();
                let inventory = &self.player_data.inventory;

                ipc = ServerZoneIpcSegment {
                    op_code: ServerZoneIpcType::Equip,
                    timestamp: timestamp_secs(),
                    data: ServerZoneIpcData::Equip(Equip {
                        main_weapon_id: inventory.get_main_weapon_id(&mut game_data),
                        sub_weapon_id: 0,
                        crest_enable: 0,
                        pattern_invalid: 0,
                        model_ids: inventory.get_model_ids(&mut game_data),
                    }),
                    ..Default::default()
                };
            }

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
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
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn process_lua_player(&mut self, player: &mut LuaPlayer) {
        for segment in &player.queued_segments {
            self.send_segment(segment.clone()).await;
        }
        player.queued_segments.clear();

        for task in &player.queued_tasks {
            match task {
                Task::ChangeTerritory { zone_id } => self.change_zone(*zone_id).await,
                Task::SetRemakeMode(remake_mode) => self
                    .database
                    .set_remake_mode(player.player_data.content_id, *remake_mode),
                Task::Warp { warp_id } => {
                    self.warp(*warp_id).await;
                }
                Task::BeginLogOut => self.begin_log_out().await,
                Task::FinishEvent { handler_id } => self.event_finish(*handler_id).await,
                Task::SetClassJob { classjob_id } => {
                    self.player_data.classjob_id = *classjob_id;
                    self.update_class_info().await;
                }
            }
        }
        player.queued_tasks.clear();
    }

    pub async fn event_finish(&mut self, handler_id: u32) {
        // sent event finish
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::EventFinish,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::EventFinish {
                    handler_id,
                    event: 1,
                    result: 1,
                    arg: 0,
                },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            })
            .await;
        }

        // give back control to the player
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::Unk18,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::Unk18 { unk: [0; 16] },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub async fn begin_log_out(&mut self) {
        // write the player back to the database
        self.database.commit_player_data(&self.player_data);

        // tell the client we're ready to disconnect at any moment'
        {
            let ipc = ServerZoneIpcSegment {
                op_code: ServerZoneIpcType::LogOutComplete,
                timestamp: timestamp_secs(),
                data: ServerZoneIpcData::LogOutComplete { unk: [0; 8] },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
            })
            .await;
        }
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
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc { data: ipc },
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
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub fn get_actor_mut(&mut self, id: ObjectId) -> Option<&mut Actor> {
        self.actors.iter_mut().find(|actor| actor.id == id)
    }

    pub fn get_actor(&self, id: ObjectId) -> Option<&Actor> {
        self.actors.iter().find(|actor| actor.id == id)
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
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
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

        let inventory = &self.player_data.inventory;

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
            main_weapon_model: inventory.get_main_weapon_id(&mut game_data),
            models: inventory.get_model_ids(&mut game_data),
            pos: exit_position.unwrap_or_default(),
            rotation: exit_rotation.unwrap_or(0.0),
            ..Default::default()
        }
    }

    pub async fn send_stats(&mut self, chara_details: &CharacterData) {
        let attributes;
        {
            let mut game_data = self.gamedata.lock().unwrap();

            attributes =
                game_data.get_racial_base_attributes(chara_details.chara_make.customize.subrace);
        }

        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::PlayerStats,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::PlayerStats(PlayerStats {
                strength: attributes.strength,
                dexterity: attributes.dexterity,
                vitality: attributes.vitality,
                intelligence: attributes.intelligence,
                mind: attributes.mind,
                hp: self.player_data.max_hp,
                mp: self.player_data.max_mp as u32,
                ..Default::default()
            }),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }
}
