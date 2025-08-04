use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ChatMessage {
    // TODO: incomplete
    #[brw(pad_before = 4)] // empty
    pub actor_id: u32,

    #[brw(pad_before = 4)] // empty
    pub timestamp: u32,

    #[brw(pad_before = 8)] // NOT empty
    pub channel: u16,

    #[brw(pad_after = 6)] // seems to be junk?
    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
