mod chat_message;
use binrw::binrw;
pub use chat_message::ChatMessage;

mod social_list;
pub use social_list::PlayerEntry;
pub use social_list::SocialList;
pub use social_list::SocialListRequest;
pub use social_list::SocialListRequestType;

mod player_spawn;
pub use player_spawn::PlayerSpawn;

mod status_effect;
pub use status_effect::StatusEffect;

mod update_class_info;
pub use update_class_info::UpdateClassInfo;

mod player_setup;
pub use player_setup::PlayerSetup;

mod player_stats;
pub use player_stats::PlayerStats;

mod actor_control;
pub use actor_control::{ActorControl, ActorControlCategory, ActorControlSelf};

mod init_zone;
pub use init_zone::InitZone;

mod npc_spawn;
pub use npc_spawn::NpcSpawn;

mod common_spawn;
pub use common_spawn::{
    BattleNpcSubKind, CharacterMode, CommonSpawn, DisplayFlag, GameMasterRank, ObjectKind,
    OnlineStatus, PlayerSubKind,
};

mod status_effect_list;
pub use status_effect_list::StatusEffectList;

mod weather_change;
pub use weather_change::WeatherChange;

mod action_request;
pub use action_request::ActionRequest;

mod container_info;
pub use container_info::{ContainerInfo, ContainerType};

mod item_info;
pub use item_info::ItemInfo;

mod event_play;
pub use event_play::EventPlay;

mod event_start;
pub use event_start::EventStart;

mod action_result;
pub use action_result::{ActionEffect, ActionResult, EffectKind};

mod actor_move;
pub use actor_move::ActorMove;

mod actor_set_pos;
pub use actor_set_pos::ActorSetPos;

use crate::common::Position;
use crate::common::read_string;
use crate::common::write_string;
use crate::opcodes::ClientZoneIpcType;
use crate::opcodes::ServerZoneIpcType;
use crate::packet::IpcSegment;
use crate::packet::ReadWriteIpcSegment;

pub type ClientZoneIpcSegment = IpcSegment<ClientZoneIpcType, ClientZoneIpcData>;

impl ReadWriteIpcSegment for ClientZoneIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + self.op_code.calc_size()
    }
}

// TODO: make generic
impl Default for ClientZoneIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientZoneIpcType::InitRequest,
            server_id: 0,
            timestamp: 0,
            data: ClientZoneIpcData::InitRequest { unk: [0; 120] },
        }
    }
}

pub type ServerZoneIpcSegment = IpcSegment<ServerZoneIpcType, ServerZoneIpcData>;

impl ReadWriteIpcSegment for ServerZoneIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + self.op_code.calc_size()
    }
}

// TODO: make generic
impl Default for ServerZoneIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ServerZoneIpcType::InitializeChat,
            server_id: 0,
            timestamp: 0,
            data: ServerZoneIpcData::InitializeChat { unk: [0; 8] },
        }
    }
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug)]
pub enum GameMasterCommandType {
    ChangeWeather = 0x6,
    ToggleInvisibility = 0xD,
    ToggleWireframe = 0x26,
    ChangeTerritory = 0x58,
}

#[binrw]
#[br(import(_magic: &ServerZoneIpcType))]
#[derive(Debug, Clone)]
pub enum ServerZoneIpcData {
    /// Sent by the server to Initialize something chat-related?
    InitializeChat { unk: [u8; 8] },
    /// Sent by the server as response to ZoneInitRequest.
    InitResponse {
        unk1: u64,
        character_id: u32,
        unk2: u32,
    },
    /// Sent by the server that tells the client which zone to load
    InitZone(InitZone),
    /// Sent by the server for... something
    ActorControlSelf(ActorControlSelf),
    /// Sent by the server containing character stats
    PlayerStats(PlayerStats),
    /// Sent by the server to setup the player on the client
    PlayerSetup(PlayerSetup),
    /// Sent by the server to setup class info
    UpdateClassInfo(UpdateClassInfo),
    /// Sent by the server to spawn the player in
    PlayerSpawn(PlayerSpawn),
    /// Sent by the server to indicate the log out is complete
    LogOutComplete {
        // TODO: guessed
        unk: [u8; 8],
    },
    /// Sent by the server to modify the client's position
    ActorSetPos(ActorSetPos),
    /// Sent by the server when they send a chat message
    ServerChatMessage {
        unk: u8, // channel?
        #[brw(pad_after = 775)]
        #[br(count = 775)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        message: String,
    },
    /// Unknown, server sends to the client before player spawn
    Unk8 { unk: [u8; 808] },
    /// Unknown, but seems to contain information on cross-world linkshells
    LinkShellInformation { unk: [u8; 456] },
    /// Unknown, server sends to the client before player spawn
    Unk9 { unk: [u8; 24] },
    /// Unknown, server sends this in response to Unk7
    Unk11 {
        timestamp: u32,
        #[brw(pad_after = 24)] // empty bytes
        unk: u32,
    },
    /// Sent by the server when it wants the client to... prepare to zone?
    PrepareZoning { unk: [u32; 4] },
    /// Sent by the server???
    Unk15 { unk: u32, player_id: u32 },
    /// Sent by the server before init zone???
    Unk16 { unk: [u8; 136] },
    /// Sent by the server
    ActorControl(ActorControl),
    /// Sent by the server
    ActorMove(ActorMove),
    /// Sent by the server
    Unk17 { unk: [u8; 104] },
    /// Sent by the server in response to SocialListRequest
    SocialList(SocialList),
    /// Sent by the server to spawn an NPC
    NpcSpawn(NpcSpawn),
    /// Sent by the server to update an actor's status effect list
    StatusEffectList(StatusEffectList),
    /// Sent by the server when it's time to change the weather
    WeatherChange(WeatherChange),
    /// Sent to inform the client of an inventory item
    ItemInfo(ItemInfo),
    /// Sent to inform the client of container status
    ContainerInfo(ContainerInfo),
    /// Sent to tell the client to play a scene
    EventPlay(EventPlay),
    /// Sent to tell the client to load a scene, but not play it
    EventStart(EventStart),
    /// Sent to update an actor's hp & mp values
    UpdateHpMpTp {
        hp: u32,
        mp: u16,
        unk: u16, // it's filled with... something
    },
    /// Sent to inform the client the consequences of their actions
    ActionResult(ActionResult),
}

#[binrw]
#[br(import(magic: &ClientZoneIpcType))]
#[derive(Debug, Clone)]
pub enum ClientZoneIpcData {
    /// Sent by the client when they successfully initialize with the server, and they need several bits of information (e.g. what zone to load)
    #[br(pre_assert(*magic == ClientZoneIpcType::InitRequest))]
    InitRequest {
        // TODO: full of possibly interesting information
        unk: [u8; 120],
    },
    /// Sent by the client when they're done loading and they need to be spawned in
    #[br(pre_assert(*magic == ClientZoneIpcType::FinishLoading))]
    FinishLoading {
        // TODO: full of possibly interesting information
        unk: [u8; 72],
    },
    /// FIXME: 32 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk1))]
    Unk1 {
        // 3 = target
        category: u32,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
        param5: u32,
        param6: u32,
        param7: u32,
    },
    /// FIXME: 16 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk2))]
    Unk2 {
        // TODO: full of possibly interesting information
        unk: [u8; 16],
    },
    /// FIXME: 8 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk3))]
    Unk3 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// FIXME: 8 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk4))]
    Unk4 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::SetSearchInfoHandler))]
    SetSearchInfoHandler {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// FIXME: 8 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk5))]
    Unk5 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// Sent by the client when it requests the friends list and other related info
    #[br(pre_assert(*magic == ClientZoneIpcType::SocialListRequest))]
    SocialListRequest(SocialListRequest),
    /// FIXME: 32 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk7))]
    Unk7 {
        // TODO: full of possibly interesting information
        timestamp: u32,
        #[brw(pad_before = 8)] // empty bytes
        #[brw(pad_after = 4)] // empty bytes
        unk1: [u8; 16], // something
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::UpdatePositionHandler))]
    UpdatePositionHandler {
        /// In radians.
        #[brw(pad_after = 4)] // empty
        rotation: f32,
        #[brw(pad_after = 4)] // empty
        position: Position,
    },
    /// Sent by the client when the user requests to log out
    #[br(pre_assert(*magic == ClientZoneIpcType::LogOut))]
    LogOut {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// Sent by the client when it's actually disconnecting
    #[br(pre_assert(*magic == ClientZoneIpcType::Disconnected))]
    Disconnected {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// Sent by the client when they send a chat message
    #[br(pre_assert(*magic == ClientZoneIpcType::ChatMessage))]
    ChatMessage(ChatMessage),
    /// Sent by the client when they send a GM command. This can only be sent by the client if they are sent a GM rank.
    #[br(pre_assert(*magic == ClientZoneIpcType::GameMasterCommand))]
    GameMasterCommand {
        // TODO: incomplete
        command: GameMasterCommandType,
        #[br(pad_before = 3)] // idk, not empty though
        arg: u32,
        unk: [u8; 24],
    },
    /// Sent by the client when the character walks into a zone transistion
    #[br(pre_assert(*magic == ClientZoneIpcType::EnterZoneLine))]
    EnterZoneLine {
        exit_box_id: u32,
        position: Position,
        #[brw(pad_after = 4)] // empty
        landset_index: i32,
    },
    /// Sent by the client when a character performs an action
    #[br(pre_assert(*magic == ClientZoneIpcType::ActionRequest))]
    ActionRequest(ActionRequest),
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk16))]
    Unk16 {
        unk: [u8; 8], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk17))]
    Unk17 {
        unk: [u8; 32], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk18))]
    Unk18 {
        unk: [u8; 8], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::EventRelatedUnk))]
    EventRelatedUnk {
        unk1: u32,
        unk2: u16,
        #[brw(pad_before = 2)]
        unk3: u32,
        unk4: u32,
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk19))]
    Unk19 {
        unk: [u8; 16], // TODO: unknown
    },
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use crate::opcodes::ServerZoneIpcType;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn world_ipc_sizes() {
        let ipc_types = [
            (
                ServerZoneIpcType::ActorControlSelf,
                ServerZoneIpcData::ActorControlSelf(ActorControlSelf::default()),
            ),
            (
                ServerZoneIpcType::InitializeChat,
                ServerZoneIpcData::InitializeChat { unk: [0; 8] },
            ),
            (
                ServerZoneIpcType::PlayerStats,
                ServerZoneIpcData::PlayerStats(PlayerStats::default()),
            ),
            (
                ServerZoneIpcType::PlayerSetup,
                ServerZoneIpcData::PlayerSetup(PlayerSetup::default()),
            ),
            (
                ServerZoneIpcType::UpdateClassInfo,
                ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo::default()),
            ),
            (
                ServerZoneIpcType::PlayerSpawn,
                ServerZoneIpcData::PlayerSpawn(PlayerSpawn::default()),
            ),
            (
                ServerZoneIpcType::ActorSetPos,
                ServerZoneIpcData::ActorSetPos(ActorSetPos::default()),
            ),
            (
                ServerZoneIpcType::NpcSpawn,
                ServerZoneIpcData::NpcSpawn(NpcSpawn::default()),
            ),
            (
                ServerZoneIpcType::ActorControl,
                ServerZoneIpcData::ActorControl(ActorControl::default()),
            ),
            (
                ServerZoneIpcType::StatusEffectList,
                ServerZoneIpcData::StatusEffectList(StatusEffectList::default()),
            ),
            (
                ServerZoneIpcType::WeatherChange,
                ServerZoneIpcData::WeatherChange(WeatherChange::default()),
            ),
            (
                ServerZoneIpcType::ActorControl,
                ServerZoneIpcData::ActorControl(ActorControl::default()),
            ),
            (
                ServerZoneIpcType::ItemInfo,
                ServerZoneIpcData::ItemInfo(ItemInfo::default()),
            ),
            (
                ServerZoneIpcType::ContainerInfo,
                ServerZoneIpcData::ContainerInfo(ContainerInfo::default()),
            ),
            (
                ServerZoneIpcType::EventPlay,
                ServerZoneIpcData::EventPlay(EventPlay::default()),
            ),
            (
                ServerZoneIpcType::EventStart,
                ServerZoneIpcData::EventStart(EventStart::default()),
            ),
            (
                ServerZoneIpcType::ActionResult,
                ServerZoneIpcData::ActionResult(ActionResult::default()),
            ),
            (
                ServerZoneIpcType::ActorMove,
                ServerZoneIpcData::ActorMove(ActorMove::default()),
            ),
        ];

        for (opcode, data) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerZoneIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: opcode.clone(), // doesn't matter for this test
                server_id: 0,
                timestamp: 0,
                data: data.clone(),
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{:#?} did not match size!",
                opcode
            );
        }
    }
}
