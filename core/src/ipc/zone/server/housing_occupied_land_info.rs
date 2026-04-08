use binrw::binrw;

use crate::{
    common::{LandId, read_string, write_string},
    ipc::zone::{HousingAppealTag, PlotSize},
};

/// Represents an occupied housing plot
#[binrw]
#[derive(Debug, Default, Clone)]
pub struct HousingOccupiedLandInfo {
    pub land_identifier: LandId,

    /// Can be either a character ID or an FC ID
    pub owner_id: u64,

    pub unk1: u8,

    /// This seems to represent the icon on the ward map
    /// (Yellow house) No visitors = 0, (Blue house) Visitors allowed = 1, (Mallet) Plot claimed, no estate built = 2
    pub house_icon: u8,

    pub house_size: PlotSize,

    /// This is very likely to be the `AvailabilityType`, but it is always `0` (Unavailable) for occupied plots
    pub unk2: u8,

    #[brw(pad_size_to = 23)]
    #[br(count = 23)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub estate_name: String,

    #[brw(pad_size_to = 193)]
    #[br(count = 193)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub estate_greeting: String,

    /// Contains the name of the owning character, or the full FC name if FC owned
    #[brw(pad_size_to = 31)]
    #[br(count = 31)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub owner_name: String,

    /// Contains the Tag of the owning FC, empty if the plot is player owned.
    #[brw(pad_size_to = 6)]
    #[br(count = 6)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,

    pub unk3: u8,

    pub tags: [HousingAppealTag; 3],
}
