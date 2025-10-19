use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct PartyMessage {
    pub party_id: u64,
    pub sender_account_id: u64,
    pub unk1: u32,
    pub unk2: u16,
    pub unk3: u16,

    pub sender_actor_id: u32,
    pub sender_world_id: u16,

    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_before = 1)]
    pub sender_name: String,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 1)]
    pub message: String,
}
