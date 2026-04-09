use binrw::binrw;
use bstr::BString;

use crate::common::{MESSAGE_MAX_LENGTH, read_sestring, write_sestring};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendAllianceMessage {
    #[brw(pad_before = 4)] // Seems to be empty/zeroes
    unk: u8, // Unknown, observed as 1

    #[brw(pad_after = 3)] // Seems to be empty/zeroes
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
