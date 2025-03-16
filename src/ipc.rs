use binrw::binrw;

use crate::{
    CHAR_NAME_MAX_LENGTH,
    common::{read_string, write_string},
    world::{
        ActorControlSelf, ChatMessage, InitZone, PlayerSetup, PlayerSpawn, PlayerStats, Position,
        SocialList, SocialListRequest, UpdateClassInfo,
    },
};

// NOTE: See https://github.com/karashiiro/FFXIVOpcodes/blob/master/FFXIVOpcodes/Ipcs.cs for opcodes

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum IPCOpCode {
    /// Sent by the server to Initialize something chat-related?
    InitializeChat = 0x2,
    /// Sent by the client when it requests the character list in the lobby.
    RequestCharacterList = 0x3,
    /// Sent by the client when it requests to enter a world.
    RequestEnterWorld = 0x4,
    /// Sent by the client after exchanging encryption information with the lobby server.
    ClientVersionInfo = 0x5,
    /// Sent by the client when they request something about the character (e.g. deletion.)
    LobbyCharacterAction = 0xB,
    /// Sent by the server to inform the client of their service accounts.
    LobbyServiceAccountList = 0xC,
    /// Sent by the server to inform the client of their characters.
    LobbyCharacterList = 0xD,
    /// Sent by the server to tell the client how to connect to the world server.
    LobbyEnterWorld = 0xF,
    /// Sent by the server to inform the client of their servers.
    LobbyServerList = 0x15,
    /// Sent by the server to inform the client of their retainers.
    LobbyRetainerList = 0x17,

    /// Sent by the client when they successfully initialize with the server, and they need several bits of information (e.g. what zone to load)
    InitRequest = 0x2ED,
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
    // Sent by the client when they're done loading and they need to be spawned in
    FinishLoading = 0x397, // TODO: assumed
    // Sent by the server to spawn the player in
    PlayerSpawn = 0x1AB,

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
    /// ALSO Sent by the server as response to ZoneInitRequest.
    Unk5 = 0x2D0,
    // Sent by the client when it requests the friends list and other related info
    SocialListRequest = 0x1A1,
    // FIXME: 32 bytes of something from the client, not sure what yet
    Unk7 = 0x2B5,
    UpdatePositionHandler = 0x249, // TODO: assumed
    // Sent by the client when the user requests to log out
    LogOut = 0x217,
    // Sent by the server to indicate the log out is complete
    LogOutComplete = 0x369,
    // Sent by the client when it's actually disconnecting
    Disconnected = 0x360,
    // Sent by the client when they send a chat message
    ChatMessage = 0xCA,
    // Sent by the client when they send a GM command. This can only be sent by the client if they are sent a GM rank.
    GameMasterCommand = 0x3B3,
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
    // Assumed what this is, but probably incorrect
    CharacterCreated = 0xE,
    // Unknown, client sends this for ???
    Unk12 = 0x0E9,
    // Sent by the client when the character walks into a zone transistion
    EnterZoneLine = 0x205,
    // Sent by the client after we sent a InitZone in TravelToZone??
    // TODO: Actually, I don't think is real...
    Unk13 = 0x2EE,
    // Sent by the server when it wants the client to... prepare to zone?
    PrepareZoning = 0x308,
    // Sent by the client for unknown reasons
    Unk14 = 0x87,
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
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ServiceAccount {
    pub id: u32,
    pub unk1: u32,
    pub index: u32,
    #[bw(pad_size_to = 0x44)]
    #[br(count = 0x44)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Server {
    pub id: u16,
    pub index: u16,
    pub flags: u32,
    #[brw(pad_before = 4)]
    #[brw(pad_after = 4)]
    pub icon: u32,
    #[bw(pad_size_to = 64)]
    #[br(count = 64)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorSetPos {
    pub unk: u32,
    pub layer_id: u32,
    pub position: Position,
    pub unk3: u32,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CharacterDetails {
    #[brw(pad_after = 4)]
    pub id: u32,
    pub content_id: u64,
    #[brw(pad_after = 4)]
    pub index: u32,
    pub origin_server_id: u16,
    pub current_server_id: u16,
    pub unk1: [u8; 16],
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub origin_server_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub current_server_name: String,
    #[bw(pad_size_to = 1024)]
    #[br(count = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_detail_json: String,
    pub unk2: [u8; 20],
}

#[binrw]
#[derive(Clone, PartialEq, Debug)]
pub enum LobbyCharacterAction {
    #[brw(magic = 0x1u8)]
    ReserveName,
    #[brw(magic = 0x2u8)]
    Create,
    #[brw(magic = 0x3u8)]
    Rename,
    #[brw(magic = 0x4u8)]
    Delete,
    #[brw(magic = 0x5u8)]
    Move,
    #[brw(magic = 0x6u8)]
    RemakeRetainer,
    #[brw(magic = 0x7u8)]
    RemakeChara,
    #[brw(magic = 0x8u8)]
    SettingsUploadBegin,
    #[brw(magic = 0xCu8)]
    SettingsUpload,
    #[brw(magic = 0xEu8)]
    WorldVisit,
    #[brw(magic = 0xFu8)]
    DataCenterToken,
    #[brw(magic = 0x15u8)]
    Request,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug)]
pub enum GameMasterCommandType {
    ChangeTerritory = 0x58,
}

#[binrw]
#[br(import(magic: &IPCOpCode))]
#[derive(Debug, Clone)]
pub enum IPCStructData {
    // Client->Server IPC
    #[br(pre_assert(*magic == IPCOpCode::ClientVersionInfo))]
    ClientVersionInfo {
        sequence: u64,

        #[brw(pad_before = 14)] // full of nonsense i don't understand yet
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        session_id: String,

        #[brw(pad_before = 8)] // empty
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(ignore)]
        version_info: String,
        // unknown stuff at the end, it's not completely empty'
    },
    #[br(pre_assert(*magic == IPCOpCode::RequestCharacterList))]
    RequestCharacterList {
        #[brw(pad_before = 16)]
        sequence: u64,
        // TODO: what is in here?
    },
    #[br(pre_assert(*magic == IPCOpCode::LobbyCharacterAction))]
    LobbyCharacterAction {
        request_number: u32,
        unk1: u32,
        character_id: u64,
        #[br(pad_before = 8)]
        character_index: u8,
        action: LobbyCharacterAction,
        world_id: u16,
        #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
        #[bw(pad_size_to = 436)]
        #[br(count = 436)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        json: String,
    },
    #[br(pre_assert(*magic == IPCOpCode::RequestEnterWorld))]
    RequestEnterWorld {
        #[brw(pad_before = 16)]
        sequence: u64,
        lookup_id: u64,
        // TODO: what else is in here?
    },
    #[br(pre_assert(*magic == IPCOpCode::InitRequest))]
    InitRequest {
        // TODO: full of possibly interesting information
        #[br(dbg)]
        unk: [u8; 105],
    },
    #[br(pre_assert(*magic == IPCOpCode::FinishLoading))]
    FinishLoading {
        // TODO: full of possibly interesting information
        unk: [u8; 72],
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk1))]
    Unk1 {
        // TODO: full of possibly interesting information
        unk: [u8; 32],
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk2))]
    Unk2 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk3))]
    Unk3 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk4))]
    Unk4 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::SetSearchInfoHandler))]
    SetSearchInfoHandler {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk5))]
    Unk5 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::SocialListRequest))]
    SocialListRequest(SocialListRequest),
    #[br(pre_assert(*magic == IPCOpCode::Unk7))]
    Unk7 {
        // TODO: full of possibly interesting information
        timestamp: u32,
        #[brw(pad_before = 8)] // empty bytes
        #[brw(pad_after = 4)] // empty bytes
        unk1: [u8; 16], // something
    },
    #[br(pre_assert(*magic == IPCOpCode::UpdatePositionHandler))]
    UpdatePositionHandler {
        // TODO: full of possibly interesting information
        unk: [u8; 24],
    },
    #[br(pre_assert(*magic == IPCOpCode::LogOut))]
    LogOut {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::Disconnected))]
    Disconnected {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == IPCOpCode::ChatMessage))]
    ChatMessage(ChatMessage),
    #[br(pre_assert(*magic == IPCOpCode::GameMasterCommand))]
    GameMasterCommand {
        // TODO: incomplete
        command: GameMasterCommandType,
        #[br(pad_before = 3)] // idk, not empty though
        arg: u32,
        #[br(dbg)]
        unk: [u8; 24],
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk12))]
    Unk12 {
        unk: [u8; 8], // TODO: unknown
    },
    #[br(pre_assert(*magic == IPCOpCode::EnterZoneLine))]
    EnterZoneLine {
        exit_box_id: u32,
        position: Position,
        #[brw(pad_after = 4)] // empty
        landset_index: i32,
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk13))]
    Unk13 {
        #[br(dbg)]
        unk: [u8; 16], // TODO: unknown
    },
    #[br(pre_assert(*magic == IPCOpCode::Unk14))]
    Unk14 {
        #[br(dbg)]
        unk: [u8; 8], // TODO: unknown
    },

    // Server->Client IPC
    #[br(pre_assert(false))]
    LobbyServiceAccountList {
        #[br(dbg)]
        sequence: u64,
        #[brw(pad_before = 1)]
        num_service_accounts: u8,
        unk1: u8,
        #[brw(pad_after = 4)]
        unk2: u8,
        #[br(count = 8)]
        service_accounts: Vec<ServiceAccount>,
    },
    #[br(pre_assert(false))]
    LobbyServerList {
        sequence: u64,
        unk1: u16,
        offset: u16,
        #[brw(pad_after = 8)]
        num_servers: u32,
        #[br(count = 6)]
        #[brw(pad_size_to = 504)]
        servers: Vec<Server>,
    },
    #[br(pre_assert(false))]
    LobbyRetainerList {
        // TODO: what is in here?
        #[brw(pad_before = 7)]
        #[brw(pad_after = 202)]
        unk1: u8,
    },
    #[br(pre_assert(false))]
    LobbyCharacterList {
        sequence: u64,
        counter: u8,
        #[brw(pad_after = 2)]
        num_in_packet: u8,
        unk1: u8,
        unk2: u8,
        unk3: u8,
        /// Set to 128 if legacy character
        unk4: u8,
        unk5: [u32; 7],
        unk6: u8,
        veteran_rank: u8,
        #[brw(pad_after = 1)]
        unk7: u8,
        days_subscribed: u32,
        remaining_days: u32,
        days_to_next_rank: u32,
        max_characters_on_world: u16,
        unk8: u16,
        #[brw(pad_after = 12)]
        entitled_expansion: u32,
        #[br(count = 2)]
        #[brw(pad_size_to = 2368)]
        characters: Vec<CharacterDetails>,
    },
    #[br(pre_assert(false))]
    LobbyEnterWorld {
        sequence: u64,
        character_id: u32,
        #[brw(pad_before = 4)]
        content_id: u64,
        #[brw(pad_before = 4)]
        #[bw(pad_size_to = 66)]
        #[br(count = 66)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        session_id: String,
        port: u16,
        #[brw(pad_after = 16)]
        #[br(count = 48)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        host: String,
    },
    #[br(pre_assert(false))]
    InitializeChat { unk: [u8; 8] },
    #[br(pre_assert(false))]
    InitResponse {
        unk1: u64,
        character_id: u32,
        unk2: u32,
    },
    #[br(pre_assert(false))]
    InitZone(InitZone),
    #[br(pre_assert(false))]
    ActorControlSelf(ActorControlSelf),
    #[br(pre_assert(false))]
    PlayerStats(PlayerStats),
    #[br(pre_assert(false))]
    PlayerSetup(PlayerSetup),
    #[br(pre_assert(false))]
    UpdateClassInfo(UpdateClassInfo),
    #[br(pre_assert(false))]
    PlayerSpawn(PlayerSpawn),
    #[br(pre_assert(false))]
    LogOutComplete {
        // TODO: guessed
        unk: [u8; 8],
    },
    #[br(pre_assert(false))]
    ActorSetPos(ActorSetPos),
    #[br(pre_assert(false))]
    ServerChatMessage {
        unk: u8, // channel?
        #[brw(pad_after = 775)]
        #[br(count = 775)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        message: String,
    },
    #[br(pre_assert(false))]
    Unk8 { unk: [u8; 808] },
    #[br(pre_assert(false))]
    LinkShellInformation { unk: [u8; 456] },
    #[br(pre_assert(false))]
    Unk9 { unk: [u8; 24] },
    #[br(pre_assert(false))]
    Unk10 { unk: u64 },
    #[br(pre_assert(false))]
    Unk11 {
        timestamp: u32,
        #[brw(pad_after = 24)] // empty bytes
        unk: u32,
    },
    #[br(pre_assert(false))]
    NameRejection {
        // FIXME: This is opcode 0x2, which is InitializeChat. We need to separate the lobby/zone IPC codes.
        unk1: u8,
        #[brw(pad_before = 7)] // empty
        unk2: u16,
        #[brw(pad_before = 6)] // empty
        #[brw(pad_after = 516)] // mostly empty
        unk3: u32,
    },
    #[br(pre_assert(false))]
    CharacterCreated {
        #[brw(pad_after = 4)] // empty
        unk1: u32,
        #[brw(pad_after = 4)] // empty
        unk2: u32,
        #[brw(pad_before = 32)] // empty
        #[brw(pad_after = 1136)] // empty
        details: CharacterDetails,
    },
    #[br(pre_assert(false))]
    PrepareZoning { unk: [u32; 4] },
    #[br(pre_assert(false))]
    Unk15 { unk: u32, player_id: u32 },
    #[br(pre_assert(false))]
    Unk16 { unk: [u8; 136] },
    #[br(pre_assert(false))]
    ActorControl {
        #[brw(pad_after = 20)] // empty
        unk: u32,
    },
    #[br(pre_assert(false))]
    ActorMove {
        #[brw(pad_after = 4)] // empty
        pos: Position,
    },
    #[br(pre_assert(false))]
    Unk17 { unk: [u8; 104] },
    #[br(pre_assert(false))]
    SocialList(SocialList),
    #[br(pre_assert(false))]
    LobbyError {
        sequence: u64,
        error: u32,
        value: u32,
        exd_error_id: u16,
        unk1: u16,
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct IPCSegment {
    pub unk1: u8,
    pub unk2: u8,
    #[br(dbg)]
    pub op_code: IPCOpCode,
    #[brw(pad_before = 2)] // empty
    #[br(dbg)]
    pub server_id: u16,
    #[br(dbg)]
    pub timestamp: u32,
    #[brw(pad_before = 4)]
    #[br(args(&op_code))]
    pub data: IPCStructData,
}

impl Default for IPCSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: IPCOpCode::InitializeChat,
            server_id: 0,
            timestamp: 0,
            data: IPCStructData::ClientVersionInfo {
                session_id: String::new(),
                version_info: String::new(),
                sequence: 0,
            },
        }
    }
}

impl IPCSegment {
    pub fn calc_size(&self) -> u32 {
        let header = 16;
        header
            + match self.data {
                IPCStructData::ClientVersionInfo { .. } => todo!(),
                IPCStructData::LobbyServiceAccountList { .. } => 24 + (8 * 80),
                IPCStructData::RequestCharacterList { .. } => todo!(),
                IPCStructData::LobbyServerList { .. } => 24 + (6 * 84),
                IPCStructData::LobbyRetainerList { .. } => 210,
                IPCStructData::LobbyCharacterList { .. } => 80 + (2 * 1184),
                IPCStructData::LobbyCharacterAction { .. } => todo!(),
                IPCStructData::LobbyEnterWorld { .. } => 160,
                IPCStructData::RequestEnterWorld { .. } => todo!(),
                IPCStructData::InitializeChat { .. } => 8,
                IPCStructData::InitRequest { .. } => 16,
                IPCStructData::InitResponse { .. } => 16,
                IPCStructData::InitZone { .. } => 103,
                IPCStructData::ActorControlSelf { .. } => 32,
                IPCStructData::PlayerStats { .. } => 224,
                IPCStructData::PlayerSetup { .. } => 2784,
                IPCStructData::UpdateClassInfo { .. } => 48,
                IPCStructData::FinishLoading { .. } => todo!(),
                IPCStructData::PlayerSpawn { .. } => 656,
                IPCStructData::Unk1 { .. } => todo!(),
                IPCStructData::Unk2 { .. } => todo!(),
                IPCStructData::Unk3 { .. } => todo!(),
                IPCStructData::Unk4 { .. } => todo!(),
                IPCStructData::SetSearchInfoHandler { .. } => todo!(),
                IPCStructData::Unk5 { .. } => todo!(),
                IPCStructData::SocialListRequest { .. } => todo!(),
                IPCStructData::Unk7 { .. } => todo!(),
                IPCStructData::UpdatePositionHandler { .. } => todo!(),
                IPCStructData::LogOut { .. } => todo!(),
                IPCStructData::LogOutComplete { .. } => 8,
                IPCStructData::Disconnected { .. } => todo!(),
                IPCStructData::ChatMessage { .. } => 1056,
                IPCStructData::GameMasterCommand { .. } => todo!(),
                IPCStructData::ActorSetPos { .. } => 24,
                IPCStructData::ServerChatMessage { .. } => 776,
                IPCStructData::Unk8 { .. } => 808,
                IPCStructData::LinkShellInformation { .. } => 456,
                IPCStructData::Unk9 { .. } => 24,
                IPCStructData::Unk10 { .. } => 8,
                IPCStructData::Unk11 { .. } => 32,
                IPCStructData::NameRejection { .. } => 536,
                IPCStructData::CharacterCreated { .. } => 2568,
                IPCStructData::Unk12 { .. } => todo!(),
                IPCStructData::EnterZoneLine { .. } => todo!(),
                IPCStructData::Unk13 { .. } => todo!(),
                IPCStructData::PrepareZoning { .. } => 16,
                IPCStructData::Unk14 { .. } => todo!(),
                IPCStructData::Unk15 { .. } => 8,
                IPCStructData::Unk16 { .. } => 136,
                IPCStructData::ActorControl { .. } => 24,
                IPCStructData::ActorMove { .. } => 16,
                IPCStructData::Unk17 { .. } => 104,
                IPCStructData::SocialList { .. } => 1136,
                IPCStructData::LobbyError { .. } => 536,
            }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn test_ipc_sizes() {
        let ipc_types = [
            IPCStructData::LobbyServerList {
                sequence: 0,
                unk1: 0,
                offset: 0,
                num_servers: 0,
                servers: Vec::new(),
            },
            IPCStructData::LobbyCharacterList {
                sequence: 0,
                counter: 0,
                num_in_packet: 0,
                unk1: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,
                unk5: [0; 7],
                unk6: 0,
                veteran_rank: 0,
                unk7: 0,
                days_subscribed: 0,
                remaining_days: 0,
                days_to_next_rank: 0,
                max_characters_on_world: 0,
                unk8: 0,
                entitled_expansion: 0,
                characters: Vec::new(),
            },
            IPCStructData::ActorControlSelf(ActorControlSelf::default()),
            IPCStructData::InitializeChat { unk: [0; 8] },
            IPCStructData::PlayerStats(PlayerStats::default()),
            IPCStructData::PlayerSetup(PlayerSetup::default()),
            IPCStructData::UpdateClassInfo(UpdateClassInfo::default()),
            IPCStructData::PlayerSpawn(PlayerSpawn::default()),
            IPCStructData::ActorSetPos(ActorSetPos::default()),
        ];

        for ipc in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = IPCSegment {
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::InitializeChat, // doesn't matter for this test
                server_id: 0,
                timestamp: 0,
                data: ipc.clone(),
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{:?} did not match size!",
                ipc
            );
        }
    }
}
