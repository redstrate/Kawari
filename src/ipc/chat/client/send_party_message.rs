use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendPartyMessage {
    pub party_id: u64,

    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
