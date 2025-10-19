use binrw::binrw;

use crate::common::{MESSAGE_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendPartyMessage {
    pub party_id: u64,

    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
