mod chat_message;
use binrw::binrw;
pub use chat_message::ChatMessage;

mod social_list;
pub use social_list::PlayerEntry;
pub use social_list::SocialList;
pub use social_list::SocialListRequest;
pub use social_list::SocialListRequestType;

mod player_spawn;
pub use player_spawn::CharacterMode;
pub use player_spawn::PlayerSpawn;

mod position;
pub use position::Position;

mod status_effect;
pub use status_effect::StatusEffect;

mod update_class_info;
pub use update_class_info::UpdateClassInfo;

mod player_setup;
pub use player_setup::PlayerSetup;

mod player_stats;
pub use player_stats::PlayerStats;

mod actor_control_self;
pub use actor_control_self::ActorControlSelf;
pub use actor_control_self::ActorControlType;

mod init_zone;
pub use init_zone::InitZone;

mod npc_spawn;
pub use npc_spawn::NpcSpawn;

mod common_spawn;
pub use common_spawn::{CommonSpawn, ObjectKind};

use crate::common::read_string;
use crate::common::write_string;
use crate::packet::IpcSegment;
use crate::packet::ReadWriteIpcSegment;

pub type ClientZoneIpcSegment = IpcSegment<ClientZoneIpcType, ClientZoneIpcData>;

impl ReadWriteIpcSegment for ClientZoneIpcSegment {}

// TODO: make generic
impl Default for ClientZoneIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientZoneIpcType::InitRequest,
            server_id: 0,
            timestamp: 0,
            data: ClientZoneIpcData::InitRequest { unk: [0; 105] },
        }
    }
}

pub type ServerZoneIpcSegment = IpcSegment<ServerZoneIpcType, ServerZoneIpcData>;

impl ReadWriteIpcSegment for ServerZoneIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + match self.op_code {
            ServerZoneIpcType::InitializeChat => 8,
            ServerZoneIpcType::InitZone => 103,
            ServerZoneIpcType::ActorControlSelf => 32,
            ServerZoneIpcType::PlayerStats => 224,
            ServerZoneIpcType::PlayerSetup => 2784,
            ServerZoneIpcType::UpdateClassInfo => 48,
            ServerZoneIpcType::PlayerSpawn => 656,
            ServerZoneIpcType::InitResponse => 16,
            ServerZoneIpcType::LogOutComplete => 8,
            ServerZoneIpcType::ActorSetPos => 24,
            ServerZoneIpcType::ServerChatMessage => 776,
            ServerZoneIpcType::Unk8 => 808,
            ServerZoneIpcType::LinkShellInformation => 456,
            ServerZoneIpcType::Unk9 => 24,
            ServerZoneIpcType::Unk10 => 8,
            ServerZoneIpcType::Unk11 => 32,
            ServerZoneIpcType::Unk15 => 8,
            ServerZoneIpcType::Unk16 => 136,
            ServerZoneIpcType::ActorControl => 24,
            ServerZoneIpcType::ActorMove => 16,
            ServerZoneIpcType::Unk17 => 104,
            ServerZoneIpcType::SocialList => 1136,
            ServerZoneIpcType::PrepareZoning => 16,
            ServerZoneIpcType::NpcSpawn => 648,
        }
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

// TODO: move to their own files
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorSetPos {
    pub unk: u32,
    pub layer_id: u32,
    pub position: Position,
    pub unk3: u32,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug)]
pub enum GameMasterCommandType {
    ChangeTerritory = 0x58,
}

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum ServerZoneIpcType {
    /// Sent by the server to Initialize something chat-related?
    InitializeChat = 0x2,
    /// Sent by the server that tells the client which zone to load
    InitZone = 0x0311,
    /// Sent by the server for... something
    ActorControlSelf = 0x018C,
    /// Sent by the server containing character stats
    PlayerStats = 0x01FA,
    /// Sent by the server to setup the player on the client
    PlayerSetup = 0x006B,
    // Sent by the server to setup class info
    UpdateClassInfo = 0x006A,
    // Sent by the server to spawn the player in
    PlayerSpawn = 0x1AB,
    /// Sent by the server as response to ZoneInitRequest.
    InitResponse = 0x2D0,
    // Sent by the server to indicate the log out is complete
    LogOutComplete = 0x369,
    // Sent by the server to modify the client's position
    ActorSetPos = 0x223,
    // Sent by the server when they send a chat message
    ServerChatMessage = 0x196,
    // Unknown, server sends to the client before player spawn
    Unk8 = 0x134,
    // Unknown, but seems to contain information on cross-world linkshells
    LinkShellInformation = 0x234,
    // Unknown, server sends to the client before player spawn
    Unk9 = 0x189,
    // Unknown, server sends to the client before player spawn.
    // Seems to the same across two different characters?
    Unk10 = 0x110,
    // Unknown, server sends this in response to Unk7
    Unk11 = 0x156,
    // Sent by the server when it wants the client to... prepare to zone?
    PrepareZoning = 0x308,
    // Sent by the server???
    Unk15 = 0x28C,
    // Sent by the server before init zone???
    Unk16 = 0x3AB,
    // Sent by the server
    ActorControl = 0x1B9,
    // Sent by the server
    ActorMove = 0x3D8,
    // Sent by the server
    Unk17 = 0x2A1,
    // Sent by the server in response to SocialListRequest
    SocialList = 0x36C,
    // Sent by the server to spawn an NPC
    NpcSpawn = 0x100,
}

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum ClientZoneIpcType {
    /// Sent by the client when they successfully initialize with the server, and they need several bits of information (e.g. what zone to load)
    InitRequest = 0x2ED,
    // Sent by the client when they're done loading and they need to be spawned in
    FinishLoading = 0x397, // TODO: assumed
    // FIXME: 32 bytes of something from the client, not sure what yet
    Unk1 = 0x37C,
    // FIXME: 16 bytes of something from the client, not sure what yet
    Unk2 = 0x2E5,
    // FIXME: 8 bytes of something from the client, not sure what yet
    Unk3 = 0x326,
    // FIXME: 8 bytes of something from the client, not sure what yet
    Unk4 = 0x143,
    SetSearchInfoHandler = 0x3B2, // TODO: assumed,
    // FIXME: 8 bytes of something from the client, not sure what yet
    Unk5 = 0x2D0,
    // Sent by the client when it requests the friends list and other related info
    SocialListRequest = 0x1A1,
    // FIXME: 32 bytes of something from the client, not sure what yet
    Unk7 = 0x2B5,
    UpdatePositionHandler = 0x249, // TODO: assumed
    // Sent by the client when the user requests to log out
    LogOut = 0x217,
    // Sent by the client when it's actually disconnecting
    Disconnected = 0x360,
    // Sent by the client when they send a chat message
    ChatMessage = 0xCA,
    // Sent by the client when they send a GM command. This can only be sent by the client if they are sent a GM rank.
    GameMasterCommand = 0x3B3,
    // Unknown, client sends this for ???
    Unk12 = 0x0E9,
    // Sent by the client when the character walks into a zone transistion
    EnterZoneLine = 0x205,
    // Sent by the client after we sent a InitZone in TravelToZone??
    // TODO: Actually, I don't think is real...
    Unk13 = 0x2EE,
    // Sent by the client for unknown reasons
    Unk14 = 0x87,
}

#[binrw]
#[br(import(_magic: &ServerZoneIpcType))]
#[derive(Debug, Clone)]
pub enum ServerZoneIpcData {
    InitializeChat {
        unk: [u8; 8],
    },
    InitResponse {
        unk1: u64,
        character_id: u32,
        unk2: u32,
    },
    InitZone(InitZone),
    ActorControlSelf(ActorControlSelf),
    PlayerStats(PlayerStats),
    PlayerSetup(PlayerSetup),
    UpdateClassInfo(UpdateClassInfo),
    PlayerSpawn(PlayerSpawn),
    LogOutComplete {
        // TODO: guessed
        unk: [u8; 8],
    },
    ActorSetPos(ActorSetPos),
    ServerChatMessage {
        unk: u8, // channel?
        #[brw(pad_after = 775)]
        #[br(count = 775)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        message: String,
    },
    Unk8 {
        unk: [u8; 808],
    },
    LinkShellInformation {
        unk: [u8; 456],
    },
    Unk9 {
        unk: [u8; 24],
    },
    Unk10 {
        unk: u64,
    },
    Unk11 {
        timestamp: u32,
        #[brw(pad_after = 24)] // empty bytes
        unk: u32,
    },
    PrepareZoning {
        unk: [u32; 4],
    },
    Unk15 {
        unk: u32,
        player_id: u32,
    },
    Unk16 {
        unk: [u8; 136],
    },
    ActorControl {
        #[brw(pad_after = 20)] // empty
        unk: u32,
    },
    ActorMove {
        #[brw(pad_after = 4)] // empty
        pos: Position,
    },
    Unk17 {
        unk: [u8; 104],
    },
    SocialList(SocialList),
    NpcSpawn(NpcSpawn),
}

#[binrw]
#[br(import(magic: &ClientZoneIpcType))]
#[derive(Debug, Clone)]
pub enum ClientZoneIpcData {
    #[br(pre_assert(*magic == ClientZoneIpcType::InitRequest))]
    InitRequest {
        // TODO: full of possibly interesting information
        unk: [u8; 105],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::FinishLoading))]
    FinishLoading {
        // TODO: full of possibly interesting information
        unk: [u8; 72],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk1))]
    Unk1 {
        // TODO: full of possibly interesting information
        unk: [u8; 32],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk2))]
    Unk2 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk3))]
    Unk3 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
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
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk5))]
    Unk5 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::SocialListRequest))]
    SocialListRequest(SocialListRequest),
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
        // TODO: full of possibly interesting information
        unk: [u8; 24],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::LogOut))]
    LogOut {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Disconnected))]
    Disconnected {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::ChatMessage))]
    ChatMessage(ChatMessage),
    #[br(pre_assert(*magic == ClientZoneIpcType::GameMasterCommand))]
    GameMasterCommand {
        // TODO: incomplete
        command: GameMasterCommandType,
        #[br(pad_before = 3)] // idk, not empty though
        arg: u32,
        unk: [u8; 24],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk12))]
    Unk12 {
        unk: [u8; 8], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::EnterZoneLine))]
    EnterZoneLine {
        exit_box_id: u32,
        position: Position,
        #[brw(pad_after = 4)] // empty
        landset_index: i32,
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk13))]
    Unk13 {
        unk: [u8; 16], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk14))]
    Unk14 {
        unk: [u8; 8], // TODO: unknown
    },
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

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
