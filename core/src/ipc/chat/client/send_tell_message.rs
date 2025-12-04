use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, MESSAGE_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendTellMessage {
    #[brw(pad_before = 8)]
    pub origin_world_id: u16,

    #[brw(pad_before = 6)]
    pub recipient_world_id: u16,

    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_before = 1)]
    pub recipient_name: String,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 5)]
    pub message: String,
}
