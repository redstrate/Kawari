use std::{net::SocketAddr, sync::Arc, time::Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

use crate::{
    common::{
        Bitmask, ClientLanguage, EquipDisplayFlag, GameData, ObjectTypeId, Position, timestamp_secs,
    },
    config::WorldConfig,
    constants::{
        ACTIVE_HELP_BITMASK_SIZE, ADVENTURE_BITMASK_SIZE, AETHER_CURRENT_BITMASK_SIZE,
        AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, AETHERYTE_UNLOCK_BITMASK_SIZE,
        BUDDY_EQUIP_BITMASK_SIZE, CAUGHT_FISH_BITMASK_SIZE, CAUGHT_SPEARFISH_BITMASK_SIZE,
        CHOCOBO_TAXI_STANDS_BITMASK_SIZE, COMPLETED_QUEST_BITMASK_SIZE,
        CRYSTALLINE_CONFLICT_ARRAY_SIZE, CUTSCENE_SEEN_BITMASK_SIZE, DUNGEON_ARRAY_SIZE,
        FRONTLINE_ARRAY_SIZE, GLASSES_STYLES_BITMASK_SIZE, GUILDHEST_ARRAY_SIZE,
        MINION_BITMASK_SIZE, MOUNT_BITMASK_SIZE, ORCHESTRION_ROLL_BITMASK_SIZE,
        ORNAMENT_BITMASK_SIZE, RAID_ARRAY_SIZE, TRIAL_ARRAY_SIZE, TRIPLE_TRIAD_CARDS_BITMASK_SIZE,
        UNLOCK_BITMASK_SIZE,
    },
    inventory::{BuyBackList, Inventory},
    ipc::zone::{
        client::ClientZoneIpcSegment,
        server::{Condition, Conditions, GameMasterRank, ServerZoneIpcData, ServerZoneIpcSegment},
    },
    opcodes::ServerZoneIpcType,
    packet::{
        CompressionType, ConnectionState, ConnectionType, IpcSegmentHeader, PacketSegment,
        SegmentData, SegmentType, ServerIpcSegmentHeader, parse_packet, send_keep_alive,
        send_packet,
    },
};

use super::{
    Actor, Event, StatusEffects, WorldDatabase,
    common::{ClientId, ServerHandle},
};

mod action;
mod actors;
mod chat;
mod effect;
mod event;
mod item;
mod lua;
mod quest;
mod social;
mod stats;
mod unlock;
mod zone;

#[derive(Debug, Default, Clone)]
pub struct TeleportQuery {
    pub aetheryte_id: u16,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnlockData {
    #[serde(default = "Bitmask::default")]
    pub unlocks: Bitmask<UNLOCK_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub aetherytes: Bitmask<AETHERYTE_UNLOCK_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub completed_quests: Bitmask<COMPLETED_QUEST_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub unlocked_raids: Bitmask<RAID_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub unlocked_dungeons: Bitmask<DUNGEON_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub unlocked_guildhests: Bitmask<GUILDHEST_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub unlocked_trials: Bitmask<TRIAL_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub unlocked_crystalline_conflict: Bitmask<CRYSTALLINE_CONFLICT_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub unlocked_frontline: Bitmask<FRONTLINE_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cleared_raids: Bitmask<RAID_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cleared_dungeons: Bitmask<DUNGEON_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cleared_guildhests: Bitmask<GUILDHEST_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cleared_trials: Bitmask<TRIAL_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cleared_crystalline_conflict: Bitmask<CRYSTALLINE_CONFLICT_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cleared_frontline: Bitmask<FRONTLINE_ARRAY_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub seen_active_help: Bitmask<ACTIVE_HELP_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub minions: Bitmask<MINION_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub mounts: Bitmask<MOUNT_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub aether_current_comp_flg_set: Bitmask<AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub aether_currents: Bitmask<AETHER_CURRENT_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub orchestrion_rolls: Bitmask<ORCHESTRION_ROLL_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub buddy_equip: Bitmask<BUDDY_EQUIP_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub cutscene_seen: Bitmask<CUTSCENE_SEEN_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub ornaments: Bitmask<ORNAMENT_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub caught_fish: Bitmask<CAUGHT_FISH_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub caught_spearfish: Bitmask<CAUGHT_SPEARFISH_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub adventures: Bitmask<ADVENTURE_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub triple_triad_cards: Bitmask<TRIPLE_TRIAD_CARDS_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub glasses_styles: Bitmask<GLASSES_STYLES_BITMASK_SIZE>,
    #[serde(default = "Bitmask::default")]
    pub chocobo_taxi_stands: Bitmask<CHOCOBO_TAXI_STANDS_BITMASK_SIZE>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum TeleportReason {
    #[default]
    NotSpecified,
    /// Teleporting/Returning to an Aetheryte or shared
    Aetheryte,
}

/// Quest information stored in the database.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PersistentQuest {
    /// ID of the quest.
    pub id: u16,
    /// Sequence in the quest.
    pub sequence: u8,
}

#[derive(Debug, Default, Clone)]
pub struct PlayerData {
    // Static data
    pub actor_id: u32,
    pub content_id: u64,
    pub account_id: u64,

    pub classjob_id: u8,
    pub classjob_levels: Vec<u16>,
    pub classjob_exp: Vec<i32>,
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
    pub city_state: u8,

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
    /// The player's party id number, used for networking party-related events
    pub party_id: u64,
    /// The player's status when connecting/reconnecting. If true, they need to rejoin their party.
    pub rejoining_party: bool,
    /// The player's currently active quests.
    pub active_quests: Vec<PersistentQuest>,
}

/// Various obsfucation-related bits like the seeds and keys for this connection.
#[derive(Debug, Default, Clone)]
pub struct ObsfucationData {
    pub seed1: u8,
    pub seed2: u8,
    pub seed3: u32,
}

/// Represents a single connection between an instance of the client and the zone portion of the world server.
pub struct ZoneConnection {
    pub config: WorldConfig,
    pub socket: TcpStream,

    pub state: ConnectionState,
    pub player_data: PlayerData,

    pub spawn_index: u8,

    pub status_effects: StatusEffects,

    pub events: Vec<Event>,
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
    pub client_language: ClientLanguage,

    pub should_run_finish_zoning: bool,
}

impl ZoneConnection {
    pub fn parse_packet(&mut self, data: &[u8]) -> Vec<PacketSegment<ClientZoneIpcSegment>> {
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

        // This is meant to protect against stack-smashing in nested futures
        Box::pin(send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Zone,
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        ))
        .await;
    }

    // TODO: Get rid of this? Lua.rs doesn't really need it but we'll continue using it for now.
    pub async fn send_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        // Ditto as above
        Box::pin(send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Zone,
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        ))
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

    pub async fn send_arbitrary_packet(&mut self, op_code: u16, data: Vec<u8>) {
        let ipc = ServerZoneIpcSegment {
            header: ServerIpcSegmentHeader::from_opcode(ServerZoneIpcType::Unknown(op_code)),
            data: ServerZoneIpcData::Unknown { unk: data },
        };
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_keep_alive(&mut self, id: u32, timestamp: u32) {
        send_keep_alive::<ServerZoneIpcSegment>(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Zone,
            id,
            timestamp,
        )
        .await;
    }
}
