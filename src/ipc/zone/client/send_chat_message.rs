use binrw::binrw;

use crate::common::{ChatChannel, Position, read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SendChatMessage {
    #[brw(pad_before = 4)] // empty
    pub actor_id: u32,

    pub pos: Position,
    pub rotation: f32,

    pub channel: ChatChannel,

    #[brw(pad_after = 6)] // seems to be junk?
    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
