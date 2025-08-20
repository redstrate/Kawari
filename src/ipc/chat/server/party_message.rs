use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone)]
pub struct PartyMessage {
    pub party_id: u64,
    pub sender_account_id: u32,
    pub unk1: u32,
    pub unk2: u32,
    pub unk3: u16,
    pub unk4: u16,

    pub sender_actor_id: u32,
    pub sender_world_id: u16,

    #[br(count = 32)]
    #[bw(pad_size_to = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_before = 1)]
    pub sender_name: String,

    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 1)]
    pub message: String,
}
