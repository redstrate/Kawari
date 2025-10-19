use binrw::binrw;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, ChatChannel, MESSAGE_MAX_LENGTH, read_string, write_string,
};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct ChatMessage {
    pub sender_account_id: u64,
    pub unk1: u32,
    pub unk2: u32,

    pub sender_actor_id: u32,

    pub sender_world_id: u16,
    pub channel: ChatChannel,

    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
