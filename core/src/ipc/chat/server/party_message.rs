use binrw::binrw;
use bstr::BString;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, ObjectId, read_sestring, read_string, write_sestring,
    write_string,
};
use crate::ipc::chat::ChatChannel;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct PartyMessage {
    pub party_chatchannel: ChatChannel,
    pub sender_account_id: u64,
    pub sender_content_id: u64,

    pub sender_actor_id: ObjectId,
    pub sender_world_id: u16,

    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_before = 1)]
    pub sender_name: String,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    #[brw(pad_after = 1)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
