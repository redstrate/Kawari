use binrw::binrw;
use bstr::BString;

use crate::common::{
    MESSAGE_MAX_LENGTH, ObjectId, read_sestring, read_string, write_sestring, write_string,
};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct AllianceMessage {
    pub sender_account_id: u64,
    pub sender_content_id: u64,

    pub sender_actor_id: ObjectId,
    pub sender_home_world_id: u16, // TODO: This world id or the other might need to be swapped with each other, need a capture of being on a different world
    pub sender_current_world_id: u16,
    pub unk1: u8, // Unknown, observed as 1

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 7)] // Seems to be empty/zeroes
    pub message: String,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct AllianceMessageEcho {
    pub unk1: u8, // Unknown, observed as 1

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    #[brw(pad_after = 7)] // Seems to be empty/zeroes
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
