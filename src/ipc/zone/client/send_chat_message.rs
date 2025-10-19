use binrw::binrw;

use crate::common::{MESSAGE_MAX_LENGTH, Position, read_string, write_string};
use crate::ipc::chat::ChatChannelType;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendChatMessage {
    #[brw(pad_before = 4)] // empty
    pub actor_id: u32,

    pub pos: Position,
    pub rotation: f32,

    pub channel: ChatChannelType,

    #[brw(pad_after = 6)] // seems to be junk?
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
