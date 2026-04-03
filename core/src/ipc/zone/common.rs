use binrw::binrw;

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
