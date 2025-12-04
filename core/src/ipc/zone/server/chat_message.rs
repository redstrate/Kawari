use binrw::binrw;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, ObjectId, read_string, write_string,
};

use crate::ipc::chat::ChatChannelType;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct ChatMessage {
    pub sender_account_id: u64,
    pub sender_content_id: u64,

    pub sender_actor_id: ObjectId,

    pub sender_world_id: u16,
    pub channel: ChatChannelType,

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
