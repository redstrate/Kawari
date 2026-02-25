#![allow(unused_assignments)] // false positive caused by binrw

use binrw::binrw;

use crate::common::HandlerId;

#[derive(Debug, Clone, Default)]
#[binrw]
#[brw(little)]
#[brw(import{max_params: usize})]
pub struct MapMarkers {
    /// How many markers to update.
    marker_count: u32,

    /// Icons to set.
    #[br(count = max_params)]
    #[bw(pad_size_to = 4 * max_params)]
    icon_ids: Vec<u32>,

    /// The instance ID in the level.
    #[br(count = max_params)]
    #[bw(pad_size_to = 4 * max_params)]
    layout_ids: Vec<u32>,

    /// The event ID to update for, usually a quest ID.
    #[br(count = max_params)]
    #[bw(pad_size_to = 4 * max_params)]
    handler_ids: Vec<HandlerId>,

    /// Unknown (elevation?)
    #[brw(pad_after = 4)] // empty
    #[br(count = max_params)]
    #[bw(pad_size_to = max_params)]
    unk: Vec<u8>,
}
