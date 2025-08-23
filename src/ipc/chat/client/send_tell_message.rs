use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendTellMessage {
    #[brw(pad_before = 8)]
    pub origin_world_id: u16,

    #[brw(pad_before = 6)]
    pub recipient_world_id: u16,

    #[br(count = 32)]
    #[bw(pad_size_to = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_before = 1)]
    pub recipient_name: String,

    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 5)]
    pub message: String,
}
