use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum IPCOpCode {
    /// Sent by the client when it requests the character list in the lobby.
    RequestCharacterList = 0x3,
    /// Sent by the client after exchanging encryption information with the lobby server.
    ClientVersionInfo = 0x5,
    /// Sent by the server to inform the client of their service accounts.
    LobbyServiceAccountList = 0xC,
    /// Sent by the server to inform the client of their characters.
    LobbyCharacterList = 0xD,
    /// Sent by the server to inform the client of their servers.
    LobbyServerList = 0x15,
    /// Sent by the server to inform the client of their retainers.
    LobbyRetainerList = 0x17,
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
    #[bw(pad_size_to = 0x40)]
    #[br(count = 0x40)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CharacterDetails {
    #[brw(pad_after = 4)]
    pub id: u32,
    pub content_id: u64,
    #[brw(pad_after = 4)]
    pub index: u32,
    // TODO: lol? why is there server_id1?
    pub server_id: u16,
    pub server_id1: u16,
    pub unk1: [u8; 16],
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_server_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_server_name1: String,
    #[bw(pad_size_to = 1024)]
    #[br(count = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_detail_json: String,
    pub unk2: [u8; 20],
}

/*impl Default for CharacterDetails {
    fn default() -> Self {
        Self {
            id: 0,
            content_id: 0,
            index: 1,
            server_id: 0,
            server_id1: 0,
            unk1: [0; 16],
            character_name: String::new(),
            character_server_name: String::new(),
            character_server_name1: String::new(),
            character_detail_json: String::new(),
            unk2: [0; 20],
        }
    }
}*/

#[binrw]
#[br(import(magic: &IPCOpCode))]
#[derive(Debug, Clone)]
pub enum IPCStructData {
    // Client->Server IPC
    #[br(pre_assert(*magic == IPCOpCode::ClientVersionInfo))]
    ClientVersionInfo {
        #[brw(pad_before = 18)] // full of nonsense i don't understand yet
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

    // Server->Client IPC
    LobbyServiceAccountList {
        sequence: u64,
        #[brw(pad_before = 1)]
        num_service_accounts: u8,
        unk1: u8,
        #[brw(pad_after = 4)]
        unk2: u8,
        #[br(count = 8)]
        service_accounts: Vec<ServiceAccount>,
    },
    LobbyServerList {
        sequence: u64,
        unk1: u16,
        offset: u16,
        #[brw(pad_after = 8)]
        num_servers: u32,
        #[br(count = 6)]
        servers: Vec<Server>,
    },
    LobbyRetainerList {
        // TODO: what is in here?
        #[brw(pad_before = 7)]
        #[brw(pad_after = 202)]
        unk1: u8,
    },
    LobbyCharacterList {
        sequence: u64,
        counter: u8,
        #[brw(pad_after = 2)]
        num_in_packet: u8,
        unk1: u8,
        unk2: u8,
        unk3: u8,
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
        characters: Vec<CharacterDetails>,
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct IPCSegment {
    pub unk1: u8,
    pub unk2: u8,
    pub op_code: IPCOpCode,
    #[brw(pad_before = 2)] // empty
    pub server_id: u16,
    pub timestamp: u32,
    #[brw(pad_before = 4)]
    #[br(args(&op_code))]
    pub data: IPCStructData,
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
            }
    }
}
