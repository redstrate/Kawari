use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use mlua::Function;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

use crate::{
    AETHERYTE_UNLOCK_BITMASK_SIZE, CLASSJOB_ARRAY_SIZE, COMPLETED_LEVEQUEST_BITMASK_SIZE,
    COMPLETED_QUEST_BITMASK_SIZE, DUNGEON_ARRAY_SIZE, ERR_INVENTORY_ADD_FAILED,
    GUILDHEST_ARRAY_SIZE, LogMessageType, PVP_ARRAY_SIZE, RAID_ARRAY_SIZE, TRIAL_ARRAY_SIZE,
    UNLOCK_BITMASK_SIZE,
    common::{
        EquipDisplayFlag, GameData, INVALID_OBJECT_ID, InstanceContentType, ItemInfoQuery,
        JumpState, MoveAnimationSpeed, MoveAnimationState, MoveAnimationType, ObjectId,
        ObjectTypeId, ObjectTypeKind, Position, timestamp_secs, value_to_flag_byte_index_value,
    },
    config::{WorldConfig, get_config},
    inventory::{BuyBackList, ContainerType, Inventory, Item, Storage},
    ipc::{
        chat::ServerChatIpcSegment,
        kawari::CustomIpcSegment,
        zone::{
            DisplayFlag,
            client::{ActionRequest, ClientZoneIpcSegment},
            server::{
                ActionEffect, ActionResult, ActorControl, ActorControlCategory, ActorControlSelf,
                ActorControlTarget, ActorMove, CommonSpawn, Condition, Conditions, Config,
                ContainerInfo, CurrencyInfo, EffectEntry, EffectKind, EffectResult, Equip,
                EventScene, EventStart, GameMasterRank, InitZone, ItemInfo, NpcSpawn, ObjectKind,
                PlayerStats, PlayerSubKind, QuestActiveList, ServerZoneIpcData,
                ServerZoneIpcSegment, StatusEffect, StatusEffectList, UpdateClassInfo, Warp,
                WeatherChange,
            },
        },
    },
    opcodes::ServerZoneIpcType,
    packet::{
        CompressionType, ConnectionState, ConnectionType, OBFUSCATION_ENABLED_MODE, PacketSegment,
        ScramblerKeyGenerator, SegmentData, SegmentType, parse_packet, send_packet,
    },
};

use super::{
    Actor, CharacterData, Event, EventFinishType, StatusEffects, ToServer, WorldDatabase,
    common::{ClientId, ServerHandle},
    lua::{EffectsBuilder, ExtraLuaState, LuaPlayer, Task, load_init_script},
};

#[derive(Debug, Default, Clone)]
pub struct TeleportQuery {
    pub aetheryte_id: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockData {
    pub unlocks: Vec<u8>,
    pub aetherytes: Vec<u8>,
    pub completed_quests: Vec<u8>,
    pub unlocked_raids: Vec<u8>,
    pub unlocked_dungeons: Vec<u8>,
    pub unlocked_guildhests: Vec<u8>,
    pub unlocked_trials: Vec<u8>,
    pub unlocked_pvp: Vec<u8>,
    pub cleared_raids: Vec<u8>,
    pub cleared_dungeons: Vec<u8>,
    pub cleared_guildhests: Vec<u8>,
    pub cleared_trials: Vec<u8>,
    pub cleared_pvp: Vec<u8>,
}

impl Default for UnlockData {
    fn default() -> Self {
        Self {
            unlocks: vec![0x0; UNLOCK_BITMASK_SIZE],
            aetherytes: vec![0x0; AETHERYTE_UNLOCK_BITMASK_SIZE],
            completed_quests: vec![0x0; COMPLETED_QUEST_BITMASK_SIZE],
            unlocked_raids: vec![0x0; RAID_ARRAY_SIZE],
            unlocked_dungeons: vec![0x0; DUNGEON_ARRAY_SIZE],
            unlocked_guildhests: vec![0x0; GUILDHEST_ARRAY_SIZE],
            unlocked_trials: vec![0x0; TRIAL_ARRAY_SIZE],
            unlocked_pvp: vec![0x0; PVP_ARRAY_SIZE],
            cleared_raids: vec![0x0; RAID_ARRAY_SIZE],
            cleared_dungeons: vec![0x0; DUNGEON_ARRAY_SIZE],
            cleared_guildhests: vec![0x0; GUILDHEST_ARRAY_SIZE],
            cleared_trials: vec![0x0; TRIAL_ARRAY_SIZE],
            cleared_pvp: vec![0x0; PVP_ARRAY_SIZE],
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum TeleportReason {
    #[default]
    NotSpecified,
    /// Teleporting/Returning to an Aetheryte or shared
    Aetheryte,
}

#[derive(Debug, Default, Clone)]
pub struct PlayerData {
    // Static data
    pub actor_id: u32,
    pub content_id: u64,
    pub account_id: u32,

    pub classjob_id: u8,
    pub classjob_levels: [i32; CLASSJOB_ARRAY_SIZE],
    pub classjob_exp: [u32; CLASSJOB_ARRAY_SIZE],
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
    pub gm_invisible: bool,

    pub item_sequence: u32,
    pub shop_sequence: u32,
    /// Store the target actor id for the purpose of chaining cutscenes.
    pub target_actorid: ObjectTypeId,
    /// The server-side copy of NPC shop buyback lists.
    pub buyback_list: BuyBackList,
    pub unlocks: UnlockData,
    pub saw_inn_wakeup: bool,
    pub display_flags: EquipDisplayFlag,
    pub teleport_reason: TeleportReason,
    pub active_minion: u32,
}

/// Various obsfucation-related bits like the seeds and keys for this connection.
#[derive(Debug, Default, Clone)]
pub struct ObsfucationData {
    pub seed1: u8,
    pub seed2: u8,
    pub seed3: u32,
}

/// Represents a single connection between an instance of the client and the world server.
pub struct ZoneConnection {
    pub config: WorldConfig,
    pub socket: TcpStream,

    pub state: ConnectionState,
    pub player_data: PlayerData,

    pub spawn_index: u8,

    pub status_effects: StatusEffects,

    pub event: Option<Event>,
    pub event_type: u8,
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

    // TODO: really needs to be moved somewhere else
    pub weather_id: u16,

    pub obsfucation_data: ObsfucationData,

    // TODO: support more than one content in the queue
    pub queued_content: Option<u16>,

    pub conditions: Conditions,
}

impl ZoneConnection {
    pub fn parse_packet(
        &mut self,
        data: &[u8],
    ) -> (Vec<PacketSegment<ClientZoneIpcSegment>>, ConnectionType) {
        parse_packet(data, &mut self.state)
    }

    /// Sends an IPC segment to the player, where the source actor is also the player.
    pub async fn send_ipc_self(&mut self, ipc: ServerZoneIpcSegment) {
        let segment = PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        };

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

    // TODO: Get rid of this? Lua.rs doesn't really need it but we'll continue using it for now.
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

    pub async fn send_custom_response(&mut self, segment: PacketSegment<CustomIpcSegment>) {
        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::None,
            CompressionType::Uncompressed,
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
        self.player_data.curr_hp = 100;
        self.player_data.max_hp = 100;
        self.player_data.curr_mp = 10000;
        self.player_data.max_mp = 10000;
        self.player_data.item_sequence = 0;
        self.player_data.shop_sequence = 0;

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
                actor_id: self.player_data.actor_id,
                timestamp: timestamp_secs(),
            },
            ..Default::default()
        })
        .await;
    }

    pub async fn set_player_position(&mut self, position: Position) {
        // set pos
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Warp(Warp {
                position,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn set_actor_position(
        &mut self,
        actor_id: u32,
        position: Position,
        rotation: f32,
        anim_type: MoveAnimationType,
        anim_state: MoveAnimationState,
        jump_state: JumpState,
    ) {
        let mut anim_type = anim_type;
        let mut anim_speed = MoveAnimationSpeed::Running; // TODO: sprint is 78, jog is 72, but falling and normal running are always 60
        //let mut falling = false;

        // We're purely walking or strafing while walking. No jumping or falling.
        if anim_type & MoveAnimationType::WALKING_OR_LANDING
            == MoveAnimationType::WALKING_OR_LANDING
            && anim_state == MoveAnimationState::None
            && jump_state == JumpState::NoneOrFalling
        {
            anim_speed = MoveAnimationSpeed::Walking;
        }

        if anim_state == MoveAnimationState::LeavingCollision {
            anim_type |= MoveAnimationType::FALLING;
        }

        if jump_state == JumpState::Ascending {
            anim_type |= MoveAnimationType::FALLING;
            if anim_state == MoveAnimationState::LeavingCollision
                || anim_state == MoveAnimationState::StartFalling
            {
                anim_type |= MoveAnimationType::JUMPING;
            }
        }

        if anim_state == MoveAnimationState::EnteringCollision {
            anim_type = MoveAnimationType::WALKING_OR_LANDING;
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorMove(ActorMove {
            rotation,

            anim_type,
            anim_state,
            anim_speed,
            position,
        }));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn spawn_actor(&mut self, mut actor: Actor, mut spawn: NpcSpawn) {
        // There is no reason for us to spawn our own player again. It's probably a bug!'
        assert!(actor.id.0 != self.player_data.actor_id);

        actor.spawn_index = self.get_free_spawn_index() as u32;
        spawn.common.spawn_index = actor.spawn_index as u8;
        spawn.common.target_id = ObjectTypeId {
            object_id: actor.id,
            object_type: ObjectTypeKind::None,
        };

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::NpcSpawn(spawn));

        self.send_segment(PacketSegment {
            source_actor: actor.id.0,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;

        self.actors.push(actor);

        self.actor_control(
            actor.id.0,
            ActorControl {
                category: ActorControlCategory::ZoneIn {
                    warp_finish_anim: 1,
                    raise_anim: 0,
                    unk1: 0,
                },
            },
        )
        .await;
    }

    pub async fn remove_actor(&mut self, actor_id: u32) {
        if let Some(actor) = self.get_actor(ObjectId(actor_id)).cloned() {
            tracing::info!("Removing actor {actor_id} {}!", actor.spawn_index);

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Delete {
                spawn_index: actor.spawn_index as u8,
                actor_id,
            });

            self.send_segment(PacketSegment {
                source_actor: actor.id.0,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc(ipc),
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
        let ipc;
        {
            let game_data = self.gamedata.lock().unwrap();

            ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo {
                class_id: self.player_data.classjob_id,
                synced_level: self.current_level(&game_data) as u16,
                class_level: self.current_level(&game_data) as u16,
                current_level: self.current_level(&game_data) as u16,
                current_exp: self.current_exp(&game_data),
                ..Default::default()
            }));
        }
        self.send_ipc_self(ipc).await;
    }

    /// Request the global server state to change our zone.
    pub async fn change_zone(&mut self, new_zone_id: u16) {
        self.player_data.teleport_reason = TeleportReason::NotSpecified;
        self.handle
            .send(ToServer::ChangeZone(
                self.id,
                self.player_data.actor_id,
                new_zone_id,
            ))
            .await;
    }

    /// Handle the zone change information from the server state.
    pub async fn handle_zone_change(
        &mut self,
        new_zone_id: u16,
        weather_id: u16,
        exit_position: Position,
        exit_rotation: f32,
    ) {
        self.player_data.zone_id = new_zone_id;
        self.exit_position = Some(exit_position);
        self.exit_rotation = Some(exit_rotation);

        // fade in?
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PrepareZoning {
                log_message: 0,
                target_zone: self.player_data.zone_id,
                animation: 0,
                param4: 0,
                hide_character: 0,
                fade_out: 1,
                param_7: 1,
                fade_out_time: 1,
                unk1: 8,
                unk2: 0,
            });
            self.send_ipc_self(ipc).await;
        }

        // Player Class Info
        self.update_class_info().await;

        // Generate obsfucation-related keys if needed.
        if self.config.enable_packet_obsfucation {
            let seed1 = fastrand::u8(..);
            let seed2 = fastrand::u8(..);
            let seed3 = fastrand::u32(..);

            let generator = ScramblerKeyGenerator::new();

            self.obsfucation_data = ObsfucationData {
                seed1,
                seed2,
                seed3,
            };

            let ConnectionState::Zone { scrambler_keys, .. } = &mut self.state else {
                panic!("Unexpected connection type!");
            };
            *scrambler_keys = Some(generator.generate(seed1, seed2, seed3));

            tracing::info!(
                "You enabled packet obsfucation in your World config, if things break please report it!",
            );
        }

        // they send the initialize packet again for some reason
        {
            self.send_segment(PacketSegment {
                segment_type: SegmentType::Initialize,
                data: SegmentData::Initialize {
                    actor_id: self.player_data.actor_id,
                    timestamp: timestamp_secs(),
                },
                ..Default::default()
            })
            .await;
        }

        // Clear the server's copy of the buyback list.
        self.player_data.buyback_list = BuyBackList::default();

        // Init Zone
        {
            let config = get_config();

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InitZone(InitZone {
                territory_type: new_zone_id,
                weather_id,
                obsfucation_mode: if config.world.enable_packet_obsfucation {
                    OBFUSCATION_ENABLED_MODE
                } else {
                    0
                },
                seed1: !self.obsfucation_data.seed1,
                seed2: !self.obsfucation_data.seed2,
                seed3: !self.obsfucation_data.seed3,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
        }

        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::SetItemLevel {
                level: self.player_data.inventory.equipped.calculate_item_level() as u32,
            },
        })
        .await;

        // send some weird thing to make the zone load correctly
        {
            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::UnkZoneLoad1 {
                unk1: [0; 56],
            }))
            .await;

            self.send_ipc_self(ServerZoneIpcSegment::new(ServerZoneIpcData::UnkZoneLoad2 {
                unk1: [0; 8],
            }))
            .await;
        }
    }

    pub async fn warp(&mut self, warp_id: u32) {
        self.player_data.teleport_reason = TeleportReason::NotSpecified;
        self.handle
            .send(ToServer::Warp(self.id, self.player_data.actor_id, warp_id))
            .await;
    }

    pub async fn warp_aetheryte(&mut self, aetheryte_id: u32) {
        self.player_data.teleport_reason = TeleportReason::Aetheryte;
        self.handle
            .send(ToServer::WarpAetheryte(
                self.id,
                self.player_data.actor_id,
                aetheryte_id,
            ))
            .await;
    }

    pub async fn change_weather(&mut self, new_weather_id: u16) {
        self.weather_id = new_weather_id;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::WeatherId(WeatherChange {
            weather_id: new_weather_id,
            transistion_time: 1.0,
        }));
        self.send_ipc_self(ipc).await;
    }

    pub fn get_free_spawn_index(&mut self) -> u8 {
        self.spawn_index += 1;
        self.spawn_index
    }

    /// Inform other clients (including yourself) that you changed your equipped model ids.
    pub async fn inform_equip(&mut self) {
        let main_weapon_id;
        let sub_weapon_id;
        let model_ids;
        {
            let mut game_data = self.gamedata.lock().unwrap();
            let inventory = &self.player_data.inventory;

            main_weapon_id = inventory.get_main_weapon_id(&mut game_data);
            sub_weapon_id = inventory.get_sub_weapon_id(&mut game_data);
            model_ids = inventory.get_model_ids(&mut game_data);
        }

        self.handle
            .send(ToServer::Equip(
                self.id,
                self.player_data.actor_id,
                main_weapon_id,
                sub_weapon_id,
                model_ids,
            ))
            .await;
    }

    pub async fn send_inventory(&mut self, first_update: bool) {
        let mut last_sequence = 0;
        for (sequence, (container_type, container)) in (&self.player_data.inventory.clone())
            .into_iter()
            .enumerate()
        {
            // currencies
            if container_type == ContainerType::Currency {
                let mut send_currency = async |item: &Item| {
                    // skip telling the client what they don't have
                    if item.quantity == 0 && first_update {
                        return;
                    }

                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CurrencyCrystalInfo(
                        CurrencyInfo {
                            sequence: sequence as u32,
                            container: container_type,
                            quantity: item.quantity,
                            catalog_id: item.id,
                            unk1: 1,
                            ..Default::default()
                        },
                    ));
                    self.send_ipc_self(ipc).await;
                };

                for i in 0..container.max_slots() {
                    send_currency(container.get_slot(i as u16)).await;
                }
            } else {
                // items

                let mut send_slot = async |slot_index: u16, item: &Item| {
                    // skip telling the client what they don't have
                    if item.quantity == 0 && first_update {
                        return;
                    }

                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateItem(ItemInfo {
                        sequence: sequence as u32,
                        container: container_type,
                        slot: slot_index,
                        quantity: item.quantity,
                        catalog_id: item.id,
                        condition: item.condition,
                        glamour_catalog_id: item.glamour_catalog_id,
                        ..Default::default()
                    }));
                    self.send_ipc_self(ipc).await;
                };

                for i in 0..container.max_slots() {
                    send_slot(i as u16, container.get_slot(i as u16)).await;
                }
            }

            // inform the client of container state
            {
                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
                        container: container_type,
                        num_items: container.num_items(),
                        sequence: sequence as u32,
                        ..Default::default()
                    }));
                self.send_ipc_self(ipc).await;
            }

            last_sequence = sequence;
        }

        let mut sequence = last_sequence + 1;

        // dummy container states that are not implemented
        // inform the client of container state
        for container_type in [
            ContainerType::Crystals,
            ContainerType::Mail,
            ContainerType::Unk2,
            ContainerType::ArmoryWaist,
        ] {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContainerInfo(ContainerInfo {
                container: container_type,
                num_items: 0,
                sequence: sequence as u32,
                ..Default::default()
            }));
            self.send_ipc_self(ipc).await;
            sequence += 1;
        }
    }

    pub async fn update_equip(
        &mut self,
        actor_id: u32,
        main_weapon_id: u64,
        sub_weapon_id: u64,
        model_ids: [u32; 10],
    ) {
        let chara_details = self.database.find_chara_make(self.player_data.content_id);
        self.send_stats(&chara_details).await;
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Equip(Equip {
            main_weapon_id,
            sub_weapon_id,
            model_ids,
            ..Default::default()
        }));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;

        // TODO: get a capture of another player equipping stuff to see if we get this as well, but it seems unlikely.
        if self.player_data.actor_id == actor_id {
            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::SetItemLevel {
                    level: self.player_data.inventory.equipped.calculate_item_level() as u32,
                },
            })
            .await;
            // Uknown what this is, it's seen when (un)equipping stuff.
            self.actor_control_self(ActorControlSelf {
                category: ActorControlCategory::Unknown {
                    category: 57,
                    param1: 0,
                    param2: 0,
                    param3: 0,
                    param4: 0,
                },
            })
            .await;
        }

        self.process_effects_list().await;
        self.update_class_info().await;
    }

    pub async fn send_message(&mut self, message: &str) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerChatMessage {
            message: message.to_string(),
            param: 0,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn toggle_invisibility(&mut self, invisible: bool) {
        self.player_data.gm_invisible = invisible;
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::ToggleInvisibility { invisible },
        })
        .await;
    }

    pub async fn process_lua_player(&mut self, player: &mut LuaPlayer) {
        // First, send player-related segments
        for segment in &player.queued_segments {
            self.send_segment(segment.clone()).await;
        }
        player.queued_segments.clear();

        // Second, send zone-related segments
        for segment in &player.zone_data.queued_segments {
            let mut edited_segment = segment.clone();
            edited_segment.target_actor = player.player_data.actor_id;
            self.send_segment(edited_segment).await;
        }
        player.zone_data.queued_segments.clear();

        let tasks = player.queued_tasks.clone();
        for task in &tasks {
            match task {
                Task::ChangeTerritory { zone_id } => self.change_zone(*zone_id).await,
                Task::SetRemakeMode(remake_mode) => self
                    .database
                    .set_remake_mode(player.player_data.content_id, *remake_mode),
                Task::Warp { warp_id } => {
                    self.warp(*warp_id).await;
                }
                Task::BeginLogOut => self.begin_log_out().await,
                Task::FinishEvent {
                    handler_id,
                    arg,
                    finish_type,
                } => self.event_finish(*handler_id, *arg, *finish_type).await,
                Task::SetClassJob { classjob_id } => {
                    self.player_data.classjob_id = *classjob_id;
                    self.update_class_info().await;
                }
                Task::WarpAetheryte { aetheryte_id } => {
                    self.warp_aetheryte(*aetheryte_id).await;
                }
                Task::ReloadScripts => {
                    self.reload_scripts();
                }
                Task::ToggleInvisibility { invisible } => {
                    self.toggle_invisibility(*invisible).await;
                }
                Task::Unlock { id } => {
                    let (value, index) = value_to_flag_byte_index_value(*id);
                    self.player_data.unlocks.unlocks[index as usize] |= value;

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::ToggleUnlock {
                            id: *id,
                            unlocked: true,
                        },
                    })
                    .await;
                }
                Task::UnlockAetheryte { id, on } => {
                    let unlock_all = *id == 0;
                    if unlock_all {
                        for i in 1..239 {
                            let (value, index) = value_to_flag_byte_index_value(i);
                            if *on {
                                self.player_data.unlocks.aetherytes[index as usize] |= value;
                            } else {
                                self.player_data.unlocks.aetherytes[index as usize] ^= value;
                            }

                            self.actor_control_self(ActorControlSelf {
                                category: ActorControlCategory::LearnTeleport {
                                    id: i,
                                    unlocked: *on,
                                },
                            })
                            .await;
                        }
                    } else {
                        let (value, index) = value_to_flag_byte_index_value(*id);
                        if *on {
                            self.player_data.unlocks.aetherytes[index as usize] |= value;
                        } else {
                            self.player_data.unlocks.aetherytes[index as usize] ^= value;
                        }

                        self.actor_control_self(ActorControlSelf {
                            category: ActorControlCategory::LearnTeleport {
                                id: *id,
                                unlocked: *on,
                            },
                        })
                        .await;
                    }
                }
                Task::SetLevel { level } => {
                    self.set_current_level(*level);
                    self.update_class_info().await;
                }
                Task::ChangeWeather { id } => {
                    self.change_weather(*id).await;
                }
                Task::AddGil { amount } => {
                    self.player_data.inventory.currency.get_slot_mut(0).quantity += *amount;
                    self.send_inventory(false).await;
                }
                Task::RemoveGil {
                    amount,
                    send_client_update,
                } => {
                    self.player_data.inventory.currency.get_slot_mut(0).quantity -= *amount;
                    if *send_client_update {
                        self.send_inventory(false).await;
                    }
                }
                Task::UnlockOrchestrion { id, on } => {
                    // id == 0 means "all"
                    if *id == 0 {
                        /* Currently 792 songs ingame.
                         * Commented out because this learns literally zero songs
                         * for some unknown reason. */
                        /*for i in 1..793 {
                            let idd = i as u16;
                            connection.send_message("test!").await;
                            connection.actor_control_self(ActorControlSelf {
                                category: ActorControlCategory::ToggleOrchestrionUnlock { song_id: id, unlocked: on } }).await;
                        }*/
                    } else {
                        self.actor_control_self(ActorControlSelf {
                            category: ActorControlCategory::ToggleOrchestrionUnlock {
                                song_id: *id,
                                unlocked: *on,
                            },
                        })
                        .await;
                    }
                }
                Task::AddItem {
                    id,
                    quantity,
                    send_client_update,
                } => {
                    let item_info;
                    {
                        let mut game_data = self.gamedata.lock().unwrap();
                        item_info = game_data.get_item_info(ItemInfoQuery::ById(*id));
                    }
                    if item_info.is_some() {
                        if self
                            .player_data
                            .inventory
                            .add_in_next_free_slot(Item::new(item_info.unwrap(), *quantity))
                            .is_some()
                        {
                            if *send_client_update {
                                self.send_inventory(false).await;
                            }
                        } else {
                            tracing::error!(ERR_INVENTORY_ADD_FAILED);
                            self.send_message(ERR_INVENTORY_ADD_FAILED).await;
                        }
                    } else {
                        tracing::error!(ERR_INVENTORY_ADD_FAILED);
                        self.send_message(ERR_INVENTORY_ADD_FAILED).await;
                    }
                }
                Task::CompleteAllQuests {} => {
                    self.player_data.unlocks.completed_quests =
                        vec![0xFF; COMPLETED_QUEST_BITMASK_SIZE];
                    self.send_quest_information().await;
                }
                Task::UnlockContent { id } => {
                    {
                        let mut game_data = self.gamedata.lock().unwrap();
                        if let Some(instance_content_type) = game_data.find_type_for_content(*id) {
                            // Each id has to be subtracted by it's offset in the InstanceContent Excel sheet. For example, all guildheists start at ID 10000.
                            match instance_content_type {
                                InstanceContentType::Dungeon => {
                                    let (value, index) =
                                        value_to_flag_byte_index_value(*id as u32 - 1);
                                    self.player_data.unlocks.unlocked_dungeons[index as usize] |=
                                        value;
                                }
                                InstanceContentType::Raid => {
                                    let (value, index) =
                                        value_to_flag_byte_index_value(*id as u32 - 30001);
                                    self.player_data.unlocks.unlocked_raids[index as usize] |=
                                        value;
                                }
                                InstanceContentType::Guildhests => {
                                    let (value, index) =
                                        value_to_flag_byte_index_value(*id as u32 - 10001);
                                    self.player_data.unlocks.unlocked_guildhests[index as usize] |=
                                        value;
                                }
                                InstanceContentType::Trial => {
                                    let (value, index) =
                                        value_to_flag_byte_index_value(*id as u32 - 20001);
                                    self.player_data.unlocks.unlocked_trials[index as usize] |=
                                        value;
                                }
                                _ => {
                                    tracing::warn!(
                                        "Not sure what to do about {instance_content_type:?} {id}!"
                                    );
                                }
                            };
                        } else {
                            tracing::warn!("Unknown content {id}!");
                        }
                    }

                    self.actor_control_self(ActorControlSelf {
                        category: ActorControlCategory::UnlockInstanceContent {
                            id: *id as u32,
                            unlocked: true,
                        },
                    })
                    .await;
                }
                Task::UpdateBuyBackList { list } => {
                    self.player_data.buyback_list = list.clone();
                }
                Task::AddExp { amount } => {
                    let current_exp;
                    {
                        let game_data = self.gamedata.lock().unwrap();
                        current_exp = self.current_exp(&game_data);
                    }
                    self.set_current_exp(current_exp + amount);
                    self.update_class_info().await;
                }
                Task::StartEvent {
                    actor_id,
                    event_id,
                    event_type,
                    event_arg,
                } => {
                    self.start_event(*actor_id, *event_id, *event_type, *event_arg)
                        .await;
                }
                Task::SetInnWakeup { watched } => {
                    self.player_data.saw_inn_wakeup = *watched;
                }
            }
        }
        player.queued_tasks.clear();
    }

    /// Reloads Global.lua
    pub fn reload_scripts(&mut self) {
        let mut lua = self.lua.lock().unwrap();
        if let Err(err) = load_init_script(&mut lua) {
            tracing::warn!("Failed to load Init.lua: {:?}", err);
        }
    }

    pub async fn event_scene(
        &mut self,
        target: &ObjectTypeId,
        event_id: u32,
        scene: u16,
        scene_flags: u32,
        params: Vec<u32>,
    ) {
        let scene = EventScene {
            actor_id: *target,
            event_id,
            scene,
            scene_flags,
            params_count: params.len() as u8,
            params,
            ..Default::default()
        };
        if let Some(ipc) = scene.package_scene() {
            self.send_ipc_self(ipc).await;
        } else {
            tracing::error!(
                "Unable to play event {event_id}, scene {:?}, scene_flags {scene_flags}!",
                scene
            );
            self.event_finish(event_id, 0, EventFinishType::Normal)
                .await;
        }
    }

    pub async fn event_finish(&mut self, handler_id: u32, arg: u32, finish_type: EventFinishType) {
        self.player_data.target_actorid = ObjectTypeId::default();
        // sent event finish
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventFinish {
                handler_id,
                event: self.event_type,
                result: 1,
                arg,
            });
            self.send_ipc_self(ipc).await;
        }

        // give back control to the player, or mark them as busy for some events
        match finish_type {
            EventFinishType::Normal => {
                // TODO: clear the cutscene flag instead of going nuclear
                self.conditions = Conditions::default();
            }
            EventFinishType::Jumping => {
                // We want it set here, because when the client finishes the animation, they will send us a client trigger to tell us.
                self.conditions.set_condition(Condition::WalkInEvent);
            }
        };

        self.send_conditions().await;
    }

    pub async fn send_conditions(&mut self) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Condition(self.conditions));
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_inventory_ack(&mut self, sequence: u32, action_type: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryActionAck {
            sequence,
            action_type,
        });
        self.send_ipc_self(ipc).await;
        self.player_data.item_sequence += 1;
    }

    // TODO: When we add support for ItemObtainedLogMessage, rename this and update this
    pub async fn send_gilshop_ack(
        &mut self,
        event_id: u32,
        item_id: u32,
        item_quantity: u32,
        price_per_item: u32,
        message_type: LogMessageType,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ShopLogMessage {
            event_id,
            message_type: message_type as u32,
            params_count: 3,
            item_id,
            item_quantity,
            total_sale_cost: item_quantity * price_per_item,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_gilshop_item_update(
        &mut self,
        dst_storage_id: u16,
        dst_container_index: u16,
        dst_stack: u32,
        dst_catalog_id: u32,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateInventorySlot {
            sequence: self.player_data.shop_sequence,
            dst_storage_id,
            dst_container_index,
            dst_stack,
            dst_catalog_id,
            unk1: 0x7530_0000,
        });
        self.send_ipc_self(ipc).await;
        self.player_data.shop_sequence += 1;
    }

    pub async fn send_inventory_transaction_finish(&mut self, unk1: u32, unk2: u32) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InventoryTransactionFinish {
            sequence: self.player_data.item_sequence,
            sequence_repeat: self.player_data.item_sequence,
            unk1,
            unk2,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn begin_log_out(&mut self) {
        // Write the player back to the database
        self.database.commit_player_data(&self.player_data);

        // Don't bother sending these if the client forcefully D/C'd.
        if self.gracefully_logged_out {
            // Set the client's conditions for logout preparation
            self.conditions.set_condition(Condition::LoggingOut);
            self.send_conditions().await;

            // Tell the client we're ready to disconnect at any moment
            {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LogOutComplete {
                    unk: [1, 0, 0, 0, 0, 0, 0, 0],
                });
                self.send_ipc_self(ipc).await;
            }
        }
    }

    pub async fn process_effects_list(&mut self) {
        // Only update the client if absolutely necessary (e.g. an effect is added, removed or changed duration)
        if self.status_effects.dirty {
            let mut list = [StatusEffect::default(); 30];
            list[..self.status_effects.status_effects.len()]
                .copy_from_slice(&self.status_effects.status_effects);

            let ipc;
            {
                let game_data = self.gamedata.lock().unwrap();

                ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StatusEffectList(
                    StatusEffectList {
                        statues: list,
                        classjob_id: self.player_data.classjob_id,
                        level: self.current_level(&game_data) as u8,
                        curr_hp: self.player_data.curr_hp,
                        max_hp: self.player_data.max_hp,
                        curr_mp: self.player_data.curr_mp,
                        max_mp: self.player_data.max_mp,
                        ..Default::default()
                    },
                ));
            }
            self.send_ipc_self(ipc).await;

            self.status_effects.dirty = false;
        }
    }

    pub async fn update_hp_mp(&mut self, actor_id: ObjectId, hp: u32, mp: u16) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::UpdateHpMpTp { hp, mp, unk: 0 });

        self.send_segment(PacketSegment {
            source_actor: actor_id.0,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
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
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlSelf(actor_control));
        self.send_ipc_self(ipc).await;
    }

    pub async fn actor_control(&mut self, actor_id: u32, actor_control: ActorControl) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControl(actor_control));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn actor_control_target(&mut self, actor_id: u32, actor_control: ActorControlTarget) {
        tracing::info!(
            "we are sending actor control target to {actor_id}: {actor_control:#?} and WE ARE {:#?}",
            self.player_data.actor_id
        );

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ActorControlTarget(actor_control));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        })
        .await;
    }

    pub async fn update_config(&mut self, actor_id: u32, config: Config) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Config(config));

        self.send_segment(PacketSegment {
            source_actor: actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
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

        let mut look = chara_details.chara_make.customize;

        // There seems to be no display flag for this, so clear the bit out
        if self
            .player_data
            .display_flags
            .intersects(EquipDisplayFlag::HIDE_LEGACY_MARK)
        {
            look.facial_features &= !(1 << 7);
        }

        CommonSpawn {
            class_job: self.player_data.classjob_id,
            name: chara_details.name,
            hp_curr: self.player_data.curr_hp,
            hp_max: self.player_data.max_hp,
            mp_curr: self.player_data.curr_mp,
            mp_max: self.player_data.max_mp,
            level: self.current_level(&game_data) as u8,
            object_kind: ObjectKind::Player(PlayerSubKind::Player),
            look,
            display_flags: DisplayFlag::INVISIBLE | self.player_data.display_flags.into(),
            main_weapon_model: inventory.get_main_weapon_id(&mut game_data),
            sec_weapon_model: inventory.get_sub_weapon_id(&mut game_data),
            models: inventory.get_model_ids(&mut game_data),
            pos: exit_position.unwrap_or_default(),
            rotation: exit_rotation.unwrap_or(0.0),
            voice: chara_details.chara_make.voice_id as u8,
            active_minion: self.player_data.active_minion as u16,
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

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PlayerStats(PlayerStats {
            strength: attributes.strength,
            dexterity: attributes.dexterity,
            vitality: attributes.vitality,
            intelligence: attributes.intelligence,
            mind: attributes.mind,
            hp: self.player_data.max_hp,
            mp: self.player_data.max_mp as u32,
            ..Default::default()
        }));
        self.send_ipc_self(ipc).await;
    }

    pub async fn execute_action(&mut self, request: ActionRequest, lua_player: &mut LuaPlayer) {
        let mut effects_builder = None;

        // run action script
        {
            let lua = self.lua.lock().unwrap();
            let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

            let key = request.action_key;
            if let Some(action_script) = state.action_scripts.get(&key) {
                lua.scope(|scope| {
                    let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

                    let config = get_config();

                    let file_name = format!("{}/{}", &config.world.scripts_location, action_script);
                    lua.load(
                        std::fs::read(&file_name).expect("Failed to locate scripts directory!"),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                    let func: Function = lua.globals().get("doAction").unwrap();

                    effects_builder = Some(func.call::<EffectsBuilder>(connection_data).unwrap());

                    Ok(())
                })
                .unwrap();
            } else {
                tracing::warn!("Action {key} isn't scripted yet! Ignoring...");
            }
        }

        // tell them the action results
        if let Some(effects_builder) = effects_builder {
            if let Some(actor) = self.get_actor_mut(request.target.object_id) {
                for effect in &effects_builder.effects {
                    match effect.kind {
                        EffectKind::Damage { amount, .. } => {
                            actor.hp = actor.hp.saturating_sub(amount as u32);
                        }
                        _ => todo!(),
                    }
                }

                let actor = *actor;
                self.update_hp_mp(actor.id, actor.hp, 10000).await;
            }

            // TODO: send Cooldown ActorControlSelf

            // ActionResult
            {
                let mut effects = [ActionEffect::default(); 8];
                effects[..effects_builder.effects.len()].copy_from_slice(&effects_builder.effects);

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::ActionResult(ActionResult {
                        main_target: request.target,
                        target_id_again: request.target,
                        action_id: request.action_key,
                        animation_lock_time: 0.6,
                        rotation: self.player_data.rotation,
                        action_animation_id: request.action_key as u16, // assuming action id == animation id
                        flag: 1,
                        effect_count: effects_builder.effects.len() as u8,
                        effects,
                        unk1: 2662353,
                        unk2: 3758096384,
                        hidden_animation: 1,
                        ..Default::default()
                    }));
                self.send_ipc_self(ipc).await;
            }

            // EffectResult
            // TODO: is this always sent? needs investigation
            {
                let mut num_entries = 0u8;
                let mut entries = [EffectEntry::default(); 4];

                for effect in &effects_builder.effects {
                    if let EffectKind::Unk1 {
                        effect_id,
                        duration,
                        param,
                        source_actor_id,
                        ..
                    } = effect.kind
                    {
                        entries[num_entries as usize] = EffectEntry {
                            index: num_entries,
                            unk1: 0,
                            id: effect_id,
                            param,
                            unk2: 0,
                            duration,
                            source_actor_id: INVALID_OBJECT_ID,
                        };
                        num_entries += 1;

                        // also inform the server of our new status effect
                        self.handle
                            .send(ToServer::GainEffect(
                                self.id,
                                self.player_data.actor_id,
                                effect_id,
                                duration,
                                param,
                                source_actor_id,
                            ))
                            .await;
                    }
                }

                let ipc =
                    ServerZoneIpcSegment::new(ServerZoneIpcData::EffectResult(EffectResult {
                        unk1: 1,
                        unk2: 776386,
                        target_id: request.target.object_id,
                        current_hp: self.player_data.curr_hp,
                        max_hp: self.player_data.max_hp,
                        current_mp: self.player_data.curr_mp,
                        unk3: 0,
                        class_id: self.player_data.classjob_id,
                        shield: 0,
                        entry_count: num_entries,
                        unk4: 0,
                        statuses: entries,
                    }));
                self.send_ipc_self(ipc).await;
            }

            if let Some(actor) = self.get_actor(request.target.object_id) {
                if actor.hp == 0 {
                    tracing::info!("Despawning {} because they died!", actor.id.0);
                    // if the actor died, despawn them
                    /*connection.handle
                     *                                       .send(ToServer::ActorDespawned(connection.id, actor.id.0))
                     *                                       .await;*/
                }
            }
        }
    }

    pub async fn cancel_action(&mut self) {
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::CancelCast {},
        })
        .await;
    }

    pub fn current_level(&self, game_data: &GameData) -> i32 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_levels[index as usize]
    }

    pub fn set_current_level(&mut self, level: i32) {
        let game_data = self.gamedata.lock().unwrap();

        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_levels[index as usize] = level;
    }

    pub fn current_exp(&self, game_data: &GameData) -> u32 {
        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_exp[index as usize]
    }

    pub fn set_current_exp(&mut self, exp: u32) {
        let game_data = self.gamedata.lock().unwrap();

        let index = game_data
            .get_exp_array_index(self.player_data.classjob_id as u16)
            .unwrap();
        self.player_data.classjob_exp[index as usize] = exp;
    }

    pub async fn send_quest_information(&mut self) {
        // quest active list
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::QuestActiveList(
                QuestActiveList::default(),
            ));
            self.send_ipc_self(ipc).await;
        }

        // quest complete list
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::QuestCompleteList {
                completed_quests: self.player_data.unlocks.completed_quests.clone(),
                unk2: vec![0xFF; 69],
            });
            self.send_ipc_self(ipc).await;
        }

        // levequest complete list
        // NOTE: all levequests are unlocked by default
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::LevequestCompleteList {
                completed_levequests: vec![0xFF; COMPLETED_LEVEQUEST_BITMASK_SIZE],
                unk2: Vec::default(),
            });
            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn replay_packets(&mut self, path: &str) {
        tracing::info!("Beginning replay from {path}...");
        self.handle
            .send(ToServer::BeginReplay(self.id, path.to_string()))
            .await;
    }

    pub async fn lose_effect(
        &mut self,
        effect_id: u16,
        effect_param: u16,
        effect_source_actor_id: ObjectId,
        lua_player: &mut LuaPlayer,
    ) {
        // first, inform the effect script
        {
            let lua = self.lua.lock().unwrap();
            let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

            let key = effect_id as u32;
            if let Some(effect_script) = state.effect_scripts.get(&key) {
                lua.scope(|scope| {
                    let connection_data = scope.create_userdata_ref_mut(lua_player).unwrap();

                    let config = get_config();

                    let file_name = format!("{}/{}", &config.world.scripts_location, effect_script);
                    lua.load(
                        std::fs::read(&file_name).expect("Failed to locate scripts directory!"),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                    let func: Function = lua.globals().get("onLose").unwrap();

                    func.call::<()>(connection_data).unwrap();

                    Ok(())
                })
                .unwrap();
            } else {
                tracing::warn!("Effect {effect_id} isn't scripted yet! Ignoring...");
            }
        }

        // then send the actor control to lose the effect
        self.actor_control_self(ActorControlSelf {
            category: ActorControlCategory::LoseEffect {
                effect_id: effect_id as u32,
                unk2: effect_param as u32,
                source_actor_id: effect_source_actor_id,
            },
        })
        .await;
    }

    pub async fn spawn_eobjs(&mut self, lua_player: &mut LuaPlayer) {
        let lua = self.lua.lock().unwrap();
        let state = lua.app_data_ref::<ExtraLuaState>().unwrap();

        let key = self.player_data.zone_id as u32;
        if let Some(zone_eobj_script) = state.zone_eobj_scripts.get(&key) {
            lua.scope(|scope| {
                let connection_data = scope
                    .create_userdata_ref_mut(&mut lua_player.zone_data)
                    .unwrap();

                let config = get_config();

                let file_name = format!("{}/{}", &config.world.scripts_location, zone_eobj_script);
                lua.load(std::fs::read(&file_name).expect("Failed to locate scripts directory!"))
                    .set_name("@".to_string() + &file_name)
                    .exec()
                    .unwrap();

                let func: Function = lua.globals().get("onRequestEObjSpawn").unwrap();

                func.call::<()>(connection_data).unwrap();

                Ok(())
            })
            .unwrap();
        } else {
            tracing::info!(
                "Zone {} doesn't have an eobj script.",
                self.player_data.zone_id
            );
        }
    }

    pub async fn start_event(
        &mut self,
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: u8,
        event_arg: u32,
    ) {
        self.player_data.target_actorid = actor_id;
        self.event_type = event_type;

        // tell the client the event has started
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::EventStart(EventStart {
                target_id: actor_id,
                event_id,
                event_type,
                event_arg,
                ..Default::default()
            }));

            self.send_segment(PacketSegment {
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc(ipc),
            })
            .await;
        }

        // load event script if needed
        let mut should_cancel = false;
        {
            let lua = self.lua.lock().unwrap();
            let state = lua.app_data_ref::<ExtraLuaState>().unwrap();
            if let Some(event_script) = state.event_scripts.get(&event_id) {
                self.event = Some(Event::new(event_id, event_script));
            } else {
                tracing::warn!("Event {event_id} isn't scripted yet! Ignoring...");

                should_cancel = true;
            }
        }

        if should_cancel {
            // give control back to the player so they aren't stuck
            self.event_finish(event_id, 0, EventFinishType::Normal)
                .await;
            self.send_message(&format!(
                "Event {event_id} tried to start, but it doesn't have a script associated with it!"
            ))
            .await;
        }
    }

    pub async fn send_arbitrary_packet(&mut self, op_code: u16, data: Vec<u8>) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::Unknown(op_code),
            data: ServerZoneIpcData::Unknown { unk: data },
            ..Default::default()
        };
        self.send_ipc_self(ipc).await;
    }

    pub async fn run_gm_command(
        &mut self,
        command: u32,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        lua_player: &mut LuaPlayer,
    ) {
        let lua = self.lua.lock().unwrap();
        let state = lua.app_data_ref::<ExtraLuaState>().unwrap();
        let config = get_config();

        if let Some(command_script) = state.gm_command_scripts.get(&command) {
            let file_name = format!("{}/{}", &config.world.scripts_location, command_script);

            let mut run_script = || -> mlua::Result<()> {
                lua.scope(|scope| {
                    let connection_data = scope
                    .create_userdata_ref_mut(lua_player)?;
                    /* TODO: Instead of panicking we ought to send a message to the player
                        * and the console log, and abandon execution. */
                    lua.load(
                        std::fs::read(&file_name).unwrap_or_else(|_| panic!("Failed to load script file {}!", &file_name)),
                    )
                    .set_name("@".to_string() + &file_name)
                    .exec()?;

                    let required_rank = lua.globals().get("required_rank");
                    if let Err(error) = required_rank {
                        tracing::info!("Script is missing required_rank! Unable to run command, sending error to user. Additional information: {}", error);
                        let func: Function =
                            lua.globals().get("onCommandRequiredRankMissingError")?;
                        func.call::<()>((error.to_string(), connection_data))?;
                        return Ok(());
                    }

                    /* Reset state for future commands. Without this it'll stay set to the last value
                    * and allow other commands that omit required_rank to run, which is undesirable. */
                    lua.globals().set("required_rank", mlua::Value::Nil)?;

                    if self.player_data.gm_rank as u8 >= required_rank? {
                        let func: Function =
                            lua.globals().get("onCommand")?;
                        func.call::<()>(([arg0, arg1, arg2, arg3], connection_data))?;

                        /* `command_sender` is an optional variable scripts can define to identify themselves in print messages.
                            * It's okay if this global isn't set. We also don't care what its value is, just that it exists.
                            * This is reset -after- running the command intentionally. Resetting beforehand will never display the command's identifier.
                            */
                        let command_sender: Result<mlua::prelude::LuaValue, mlua::prelude::LuaError> = lua.globals().get("command_sender");
                        if command_sender.is_ok() {
                            lua.globals().set("command_sender", mlua::Value::Nil)?;
                        }
                        Ok(())
                    } else {
                        tracing::info!("User with account_id {} tried to invoke GM command {} with insufficient privileges!",
                        self.player_data.account_id, command);
                        let func: Function =
                            lua.globals().get("onCommandRequiredRankInsufficientError")?;
                        func.call::<()>(connection_data)?;
                        Ok(())
                    }
                })
            };

            if let Err(err) = run_script() {
                tracing::warn!("Lua error in {file_name}: {:?}", err);
            }
        } else {
            tracing::warn!(
                "Received unknown GM command {command} with args: arg0: {arg0} arg1: {arg1} arg2: {arg2} arg3: {arg3}!"
            );
        }
    }
}
