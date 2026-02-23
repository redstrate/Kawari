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

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StrategyBoard {
    /// Apparently compressed data. Completely unknown what's in here, but it seems the server doesn't care about it, unless we want to document it someday!
    #[br(count = 1176)]
    #[bw(pad_size_to = 1176)]
    data: Vec<u8>,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StrategyBoardUpdate {
    /// Unknown data for now. The server seems not to care about it, unless we want to document it someday!
    #[br(count = 64)]
    #[bw(pad_size_to = 64)]
    data: Vec<u8>,
}

#[binrw]
#[repr(u8)]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum WaymarkPlacementMode {
    #[default]
    Removed = 0,
    Placed = 1,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default)]
pub struct WaymarkPreset {
    #[br(count = 104)]
    #[bw(pad_size_to = 104)]
    unk: Vec<u8>,
}
