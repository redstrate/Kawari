use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct TellMessage {
    pub sender_account_id: u64,
    pub unk1: u32,
    pub unk2: u32,
    pub sender_world_id: u16,
    /// Indicates if it's a GM tell or not.
    pub flags: u8,

    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 5)]
    pub message: String,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct TellNotFoundError {
    /// Assumed.
    pub recipient_account_id: u64,
    /// Assumed.
    pub sender_account_id: u64,
    /// Commonly seen as 0x68.
    pub unk: u32,
    /// The recipient's world id.
    pub recipient_world_id: u16,
    /// The recipient's name.
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 2)]
    pub recipient_name: String,
}
