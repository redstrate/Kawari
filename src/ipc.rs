use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, PartialEq, Debug)]
pub enum IPCOpCode {
    /// Sent by the client after exchanging encryption information with the lobby server.
    ClientVersionInfo = 0x5,
    /// Sent by the server to inform the client of service accounts.
    LobbyServiceAccountList = 0xC,
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
            }
    }
}
