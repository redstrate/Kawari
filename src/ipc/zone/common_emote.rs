use crate::common::{ObjectTypeId, read_bool_from, write_bool_as};
use binrw::binrw;

/// The common structure used by both ActorControlTarget and ClientTrigger.
#[binrw]
#[brw(little)]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct CommonEmoteInfo {
    /// The id of the emote.
    emote: u32,
    /// 0/false = text shown, 1/true = text hidden
    #[brw(pad_before = 4)] // blank
    #[br(map = read_bool_from::<u32>)]
    #[bw(map = write_bool_as::<u32>)]
    hide_text: bool,
    /// The actor id of the target.
    #[brw(pad_before = 8)] // blank
    target: ObjectTypeId,
}
