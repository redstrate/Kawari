use binrw::binrw;
use serde::{Deserialize, Serialize};

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ServiceAccount {
    pub id: u64,
    pub index: u32,
    #[bw(pad_size_to = 0x44)]
    #[br(count = 0x44)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct LoginReply {
    pub sequence: u64,
    #[brw(pad_before = 1)]
    pub num_service_accounts: u8,
    pub unk1: u8,
    #[brw(pad_after = 4)]
    pub unk2: u8,
    #[br(count = 8)]
    #[brw(pad_size_to = (8 * 80))]
    pub service_accounts: Vec<ServiceAccount>,
}
