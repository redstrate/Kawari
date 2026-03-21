use binrw::binrw;

use crate::common::{MESSAGE_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendAllianceMessage {
    #[brw(pad_before = 4)] // Seems to be empty/zeroes
    unk: u8, // Unknown, observed as 1

    #[brw(pad_after = 3)] // Seems to be empty/zeroes
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
