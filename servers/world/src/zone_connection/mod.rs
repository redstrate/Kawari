use std::{
    sync::Arc,
    time::{Instant, SystemTime},
};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;

use crate::{
    Content, GameData, Unlock,
    database::{AetherCurrent, Aetheryte, Character, ClassJob, Companion, Quest, Volatile},
    lua::LuaTask,
};
use kawari::{
    common::{ClientLanguage, HandlerId, ObjectId, ObjectTypeId, Position, timestamp_secs},
    config::WorldConfig,
    ipc::zone::{
        client::ClientZoneIpcSegment,
        server::{Condition, Conditions, ServerZoneIpcData, ServerZoneIpcSegment},
    },
    opcodes::ServerZoneIpcType,
    packet::{
        CompressionType, ConnectionState, ConnectionType, IpcSegmentHeader, PacketSegment,
        SegmentData, SegmentType, ServerIpcSegmentHeader, parse_packet, send_keep_alive,
        send_packet,
    },
};

use super::{
    Event, WorldDatabase,
    common::{ClientId, ServerHandle},
    inventory::{BuyBackList, Inventory},
};

mod actors;
mod chat;
mod effect;
mod event;
mod item;
mod lua;
mod quest;
mod shop;
mod social;
pub mod spawn_allocator;
mod stats;
mod unlock;
mod zone;

#[derive(Debug, Default, Clone)]
pub struct TeleportQuery {
    pub aetheryte_id: u16,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub enum TeleportReason {
    #[default]
    NotSpecified,
    /// Teleporting/Returning to an Aetheryte or shared
    Aetheryte,
}

/// Quest information stored in the database.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PersistentQuest {
    /// ID of the quest.
    pub id: u16,
    /// Sequence in the quest.
    pub sequence: u8,
}

/// Persistent player data.
/// Please try to keep fields in here to a minimum, specifically stuff that may be persistent and/or accessed from Lua.
#[derive(Debug, Default, Clone)]
pub struct PlayerData {
    pub character: Character,
    pub classjob: ClassJob,
    pub subrace: u8,
    pub volatile: Volatile,
    pub inventory: Inventory,
    pub city_state: u8,

    pub teleport_query: TeleportQuery,
    pub gm_invisible: bool,

    pub item_sequence: u32,
    pub shop_sequence: u32,
    /// The server-side copy of NPC shop buyback lists.
    pub buyback_list: BuyBackList,
    pub unlock: Unlock,
    pub content: Content,
    pub aetheryte: Aetheryte,
    pub aether_current: AetherCurrent,
    pub companion: Companion,
    pub quest: Quest,
    pub saw_inn_wakeup: bool,
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

    pub events: Vec<Event>,

    pub id: ClientId,
    pub handle: ServerHandle,

    pub database: Arc<Mutex<WorldDatabase>>,
    pub lua: Arc<Mutex<mlua::Lua>>,
    pub gamedata: Arc<Mutex<GameData>>,

    pub exit_position: Option<Position>,
    pub exit_rotation: Option<f32>,
    pub teleport_reason: TeleportReason,
    pub active_minion: u32,
    /// The player's party id number, used for networking party-related events
    pub party_id: u64,
    /// The player's status when connecting/reconnecting. If true, they need to rejoin their party.
    pub rejoining_party: bool,
    /// The player's currently active quests.
    pub login_time: Option<SystemTime>,
    /// Store the target actor id for the purpose of chaining cutscenes.
    pub target_actorid: ObjectTypeId,
    pub transaction_sequence: u32,

    pub last_keep_alive: Instant,

    /// Whether the player was gracefully logged out
    pub gracefully_logged_out: bool,

    pub obsfucation_data: ObsfucationData,

    // TODO: support more than one content in the queue
    pub queued_content: Option<u16>,

    pub conditions: Conditions,
    pub client_language: ClientLanguage,

    /// List of queued tasks from the server.
    pub queued_tasks: Vec<LuaTask>,

    /// Information from before we entered the content.
    pub old_zone_id: u16,
    pub old_position: Position,
    pub old_rotation: f32,

    /// Information about the current content.
    pub content_handler_id: HandlerId,
}

impl ZoneConnection {
    pub fn parse_packet(&mut self, data: &[u8]) -> Vec<PacketSegment<ClientZoneIpcSegment>> {
        parse_packet(data, &mut self.state)
    }

    /// Sends an IPC segment to the player, where the source actor is also the player.
    pub async fn send_ipc_self(&mut self, ipc: ServerZoneIpcSegment) {
        // This is meant to protect against stack-smashing in nested futures
        Box::pin(self.send_ipc_from(self.player_data.character.actor_id, ipc)).await;
    }

    /// Sends an IPC segment to the player, where the source actor can be specified.
    pub async fn send_ipc_from(&mut self, source_actor: ObjectId, ipc: ServerZoneIpcSegment) {
        let segment = PacketSegment {
            source_actor,
            target_actor: self.player_data.character.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        };

        // Ditt from above
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
                actor_id: self.player_data.character.actor_id,
                timestamp: timestamp_secs(),
            },
            ..Default::default()
        })
        .await;
    }

    pub async fn begin_log_out(&mut self) {
        // If we were last in an instance, tell the server we're outside of it so we don't get stuck/crash.
        if self.conditions.has_condition(Condition::BoundByDuty) {
            self.player_data.volatile.zone_id = self.old_zone_id as i32;
            self.player_data.volatile.position = self.old_position;
            self.player_data.volatile.rotation = self.old_rotation as f64;
        }

        // Update playtime
        {
            // By default, just write back the original playtime if something goes wrong.
            let mut database = self.database.lock();
            let mut time_played_minutes =
                database.find_playtime(self.player_data.character.content_id as u64);
            if let Some(login_time) = self.login_time {
                match SystemTime::now().duration_since(login_time) {
                    Ok(session_length) => {
                        time_played_minutes += (session_length.as_secs() / 60) as i64;
                    }
                    Err(e) => {
                        tracing::error!(
                            "Unable to update the session's playtime, due to the following error: {e}",
                        );
                    }
                }
            }
            self.player_data.character.time_played_minutes = time_played_minutes;
        }

        // Write the player back to the database
        {
            let mut database = self.database.lock();
            database.commit_player_data(&self.player_data);
        }

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

    pub async fn register_for_content(&mut self, content_ids: [u16; 5]) {
        self.queued_content = Some(content_ids[0]);

        // update
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContentFinderUpdate {
                state1: 1,
                classjob_id: self.player_data.classjob.current_class as u8, // TODO: store what they registered with, because it can change
                unk1: [0, 0, 0, 0, 0, 0, 96, 4, 2, 64, 1, 0, 0, 0, 0, 0, 1, 1],
                content_ids,
                unk2: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            });
            self.send_ipc_self(ipc).await;
        }

        // found
        {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ContentFinderFound {
                unk1: [
                    3, 0, 0, 0, 0, 0, 0, 0, 96, 4, 2, 64, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
                    0, 0,
                ],
                content_id: content_ids[0],
                unk2: [0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
            });
            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn send_playtime(&mut self) {
        if let Some(login_time) = self.login_time {
            let time_played_minutes;
            {
                let mut database = self.database.lock();
                time_played_minutes =
                    database.find_playtime(self.player_data.character.content_id as u64);
            }

            // In case something goes wrong with calculating the current session's playtime, we'll send the old total by default.
            let mut total_play_time = time_played_minutes;
            match SystemTime::now().duration_since(login_time) {
                Ok(session_length) => {
                    total_play_time = (session_length.as_secs() / 60) as i64 + time_played_minutes;

                    // Retail doesn't do this, but it's a nice QoL thing to have.
                    self.send_notice(
                        &format!(
                            "Total Play Time this Session: {} hours, {} minutes, {} seconds",
                            session_length.as_secs() / 3600,
                            (session_length.as_secs() / 60) % 60,
                            session_length.as_secs() % 60
                        )
                        .to_string(),
                    )
                    .await;
                }
                Err(e) => {
                    tracing::error!(
                        "Unable to determine the current session's playtime, due to an error: {e}",
                    );
                }
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Playtime {
                duration: total_play_time as u32,
            });

            self.send_ipc_self(ipc).await;
        }
    }
}
