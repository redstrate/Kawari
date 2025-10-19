use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct PartyMessage {
    pub party_chatchannel: u64, // TODO: This will be changed in a future PR to use the proper ChatChannel type (not to be confused with our currently named one)!
    pub sender_account_id: u64,
    pub sender_content_id: u64,

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
