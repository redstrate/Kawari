use binrw::binrw;
use bstr::BString;

use crate::common::{MESSAGE_MAX_LENGTH, ObjectId, Position, read_sestring, write_sestring};
use crate::ipc::chat::ChatChannelType;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendChatMessage {
    #[brw(pad_before = 4)] // empty
    pub actor_id: ObjectId,

    pub pos: Position,
    pub rotation: f32,

    pub channel: ChatChannelType,

    #[brw(pad_after = 6)] // seems to be junk?
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
