use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use tokio::net::TcpStream;

use crate::{
    OBFUSCATION_ENABLED_MODE,
    common::{GameData, ObjectId, Position, timestamp_secs},
    config::{WorldConfig, get_config},
    inventory::{Inventory, Item},
    ipc::{
        chat::ServerChatIpcSegment,
        zone::{
            ActorControl, ActorControlSelf, ActorControlTarget, ClientZoneIpcSegment, CommonSpawn,
            ContainerInfo, DisplayFlag, Equip, GameMasterRank, InitZone, ItemInfo, Move, NpcSpawn,
            ObjectKind, PlayerStats, PlayerSubKind, ServerZoneIpcData, ServerZoneIpcSegment,
            StatusEffect, StatusEffectList, UpdateClassInfo, Warp, WeatherChange,
        },
    },
    opcodes::ServerZoneIpcType,
    packet::{
        CompressionType, ConnectionType, PacketSegment, PacketState, SegmentData, SegmentType,
        parse_packet, send_packet,
    },
};

use super::{
    Actor, CharacterData, Event, LuaPlayer, StatusEffects, ToServer, WorldDatabase, Zone,
    common::{ClientId, ServerHandle},
    lua::Task,
};

#[derive(Debug, Default, Clone)]
pub struct TeleportQuery {
    pub aetheryte_id: u16,
}

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

    pub teleport_query: TeleportQuery,
    pub gm_rank: GameMasterRank,
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

    pub last_keep_alive: Instant,

    /// Whether the player was gracefully logged out
    pub gracefully_logged_out: bool,
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
                flag1: 128,
                flag2: 60,
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
        // the connection already checks to see if the actor already exists, so it's seems harmless if we do
        if let Some(zone) = &self.zone {
            self.handle
                .send(ToServer::LeftZone(
                    self.id,
                    self.player_data.actor_id,
                    zone.id,
                ))
                .await;
        }

        // load the new zone now
        {
            let mut game_data = self.gamedata.lock().unwrap();
            self.zone = Some(Zone::load(&mut game_data.game_data, new_zone_id));
        }

        self.player_data.zone_id = new_zone_id;

        // Player Class Info
        self.update_class_info().await;

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
            let (pop_range_id, zone_id) = game_data
                .get_warp(warp_id)
                .expect("Failed to find the warp!");

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

    pub async fn warp_aetheryte(&mut self, aetheryte_id: u32) {
        tracing::info!("Warping to aetheryte {}", aetheryte_id);

        let territory_type;
        // find the pop range on the other side
        {
            let mut game_data = self.gamedata.lock().unwrap();
            let (pop_range_id, zone_id) = game_data
                .get_aetheryte(aetheryte_id)
                .expect("Failed to find the aetheryte!");

            let new_zone = Zone::load(&mut game_data.game_data, zone_id);

            // find it on the other side
            if let Some((object, _)) = new_zone.find_pop_range(pop_range_id) {
                // set the exit position
                self.exit_position = Some(Position {
                    x: object.transform.translation[0],
                    y: object.transform.translation[1],
                    z: object.transform.translation[2],
                });
            } else {
                tracing::warn!(
                    "Failed to find pop range in {}. Falling back to 0,0,0!",
                    new_zone.id
                );
            }

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
                Task::WarpAetheryte { aetheryte_id } => {
                    self.warp_aetheryte(*aetheryte_id).await;
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
        self.gracefully_logged_out = true;

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

    pub async fn actor_control(&mut self, actor_id: u32, actor_control: ActorControl) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::ActorControl,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::ActorControl(actor_control),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }

    pub async fn actor_control_target(&mut self, actor_id: u32, actor_control: ActorControlTarget) {
        tracing::info!(
            "we are sending actor control target to {actor_id}: {actor_control:#?} and WE ARE {:#?}",
            self.player_data.actor_id
        );

        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::ActorControlTarget,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::ActorControlTarget(actor_control),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: actor_id,
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
            voice: chara_details.chara_make.voice_id as u8,
            ..Default::default()
        }
    }

    pub async fn send_stats(&mut self, chara_details: &CharacterData) {
        let attributes;
        {
            let mut game_data = self.gamedata.lock().unwrap();

            attributes = game_data
                .get_racial_base_attributes(chara_details.chara_make.customize.subrace)
                .expect("Failed to read racial attributes");
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

    pub async fn send_npc(&mut self, mut npc: NpcSpawn) {
        // the one from the global state is useless, of course
        npc.common.spawn_index = self.get_free_spawn_index();

        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::NpcSpawn,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::NpcSpawn(npc),
            ..Default::default()
        };

        self.send_segment(PacketSegment {
            source_actor: 0x106ad804,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc { data: ipc },
        })
        .await;
    }
}
