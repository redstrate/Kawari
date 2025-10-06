use binrw::binrw;

use crate::common::{ChatChannel, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct ChatMessage {
    pub sender_account_id: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,

    pub sender_actor_id: u32,

    pub sender_world_id: u16,
    pub channel: ChatChannel,

    #[br(count = 32)]
    #[bw(pad_size_to = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,

    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
