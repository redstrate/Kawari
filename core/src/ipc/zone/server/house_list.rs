use crate::{common::HousingFlag, ipc::zone::PlotSize};
use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Debug, Default)]
pub struct House {
    pub plot_size: PlotSize,
    pub status: HouseStatus,
    #[brw(pad_after = 1)]
    pub flags: HousingFlag,
    pub fc_id: u32,
    pub fc_crest_id: u32,
    pub fc_crest_id1: u32,
    pub exterior: HouseExterior,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum HouseStatus {
    None = 0,
    #[default]
    UpForAuction = 1,
    UnderConstruction = 2,
    HouseBuilt = 3,
}

/// Represents a House's "pattern ids", or in other words, what models make up the house's exterior. All values are an index into the HousingExterior Excel sheet.
#[binrw]
#[derive(Clone, Copy, Debug, Default)]
pub struct HouseExterior {
    /// The roof's style.
    pub roof_id: u16,
    /// The walls' style.
    pub walls_id: u16,
    /// The windows' style.
    pub windows_id: u16,
    // The front door's style.
    pub door_id: u16,
    /// The roof's fixture, like a chimney.
    pub roof_fixture_id: u16,
    /// Exterior wall fixture, like an awning.
    pub wall_fixture_id: u16,
    /// The flag/banner/crest that resides above the front door.
    pub above_door_banner_id: u16,
    /// The perimeter fence's style.
    pub fence_id: u16,
    /// All of the above's dyes/stains info.
    pub colors: HouseExteriorColors,
}

/// Represents a HouseExterior's dyes/stains.
#[binrw]
#[derive(Clone, Copy, Debug, Default)]
pub struct HouseExteriorColors {
    /// The roof.
    pub roof: u8,
    /// The walls.
    pub walls: u8,
    /// The windows.
    pub windows: u8,
    // The front door.
    pub door: u8,
    /// The roof's fixture.
    pub roof_fixture: u8,
    /// Exterior wall fixture.
    pub wall_fixture: u8,
    /// The flag/banner/crest that resides above the front door.
    pub above_door_banner: u8,
    /// The perimeter fence.
    pub fence: u8,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default)]
pub struct HouseList {
    pub land_id: u16,
    pub ward: u16,
    pub territory_type_id: u16,
    pub world_id: u16,

    #[brw(pad_after = 4)] // seems empty
    pub subdivision: u32,

    pub houses: [House; 30],
}
