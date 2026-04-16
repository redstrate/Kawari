use binrw::binrw;
use bstr::BString;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, ObjectId, read_sestring, read_string, write_sestring,
    write_string,
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

    /// Name of the sender.
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,

    /// The contents of the chat message.
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
