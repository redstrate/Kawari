use binrw::binrw;
use bitflags::bitflags;

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

/// Represents housing appeal tags that players can set for their owned housing.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Debug, Default)]
pub enum HousingAppealTag {
    #[default]
    None = 0,
    Emporium = 1,
    Boutique = 2,
    DesignerHome = 3,
    MessageBook = 4,
    Tavern = 5,
    Eatery = 6,
    ImmersiveExperience = 7,
    Cafe = 8,
    Aquarium = 9,
    Sanctum = 10,
    Venue = 11,
    Florist = 12,
    Unknown = 13, // blank in the sheet...
    Library = 14,
    PhotoStudio = 15,
    HauntedHouse = 16,
    Atelier = 17,
    Bathhouse = 18,
    Garden = 19,
    FarEastern = 20,
    VisitorsWelcome = 21,
    Bakery = 22,
    UnderRenovation = 23,
    ConcertHall = 24,
}

/// Represents the size of a housing plot.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlotSize {
    #[default]
    Small = 0,
    Medium = 1,
    Large = 2,
}

/// Represents the purchase system used for a plot.
/// FCFS hasn't been used since the addition of Empyreum in 6.0.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PurchaseType {
    #[default]
    Unavailable = 0,

    /// First come, First served
    FCFS = 1,

    Lottery = 2,
}

/// Represents the allowed type of tenant for a specific plot or entire ward.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TenantType {
    #[default]
    Any = 0,

    FreeCompany = 1,

    Personal = 2,
}

/// Represents the availability of a specific plot during the *current* lottery cycle/
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvailabilityType {
    /// Plot does not participate in current lottery cycle.
    #[default]
    Unavailable = 0,

    /// Plot is available for bidding.
    Available = 1,

    /// Plot participated in lottery cycle, no more bidding allowed.
    InResultsPeriod = 2,
}

#[binrw]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HousingFlags(u8);

impl Default for HousingFlags {
    fn default() -> Self {
        HousingFlags::NONE
    }
}

bitflags! {
    impl HousingFlags: u8 {
        const NONE = 0;
        const PLOT_OWNED = 1 << 0;
        const VISITORS_ALLOWED = 1 << 1;
        const HAS_SEARCH_COMMENT = 1 << 2;
        const HOUSE_BUILT = 1 << 3;
        const OWNED_BY_FC = 1 << 4;
    }
}
