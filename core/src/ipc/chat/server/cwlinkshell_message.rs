use binrw::binrw;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, ObjectId, read_string, write_string,
};
use crate::ipc::chat::ChatChannel;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CWLinkshellMessage {
    pub cwls_chatchannel: ChatChannel,
    pub sender_account_id: u64,
    pub sender_content_id: u64,

    pub sender_actor_id: ObjectId,
    pub sender_home_world_id: u16, // TODO: This world id or the other might need to be swapped with each other, need a capture of being on a different world
    pub sender_current_world_id: u16,

    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_before = 1)] // Seems to be empty/zeroes
    pub sender_name: String,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 7)] // Seems to be empty/zeroes
    pub message: String,
}
