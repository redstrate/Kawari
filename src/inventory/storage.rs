use binrw::binrw;

use super::Item;

/// When adding a new container type, make sure to add it to InventoryIterator
#[binrw]
#[brw(little)]
#[brw(repr = u16)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ContainerType {
    #[default]
    Inventory0 = 0,
    Inventory1 = 1,
    Inventory2 = 2,
    Inventory3 = 3,

    Equipped = 1000,

    Currency = 2000,
    Crystals = 2001,
    MailEdit = 2002,
    Mail = 2003,
    KeyItems = 2004,
    HandIn = 2005,
    Unk1 = 2006,
    BlockedItems = 2007,
    Unk2 = 2008,
    Examine = 2009,
    Reclaim = 2010,
    HousingExteriorAppearanceEdit = 2011,
    HousingInteriorAppearanceEdit = 2012,
    ReconstructionBuyback = 2013,

    ArmoryOffWeapon = 3200,
    ArmoryHead = 3201,
    ArmoryBody = 3202,
    ArmoryHand = 3203,
    ArmoryLeg = 3205,
    ArmoryFoot = 3206,
    ArmoryEarring = 3207,
    ArmoryNeck = 3208,
    ArmoryWrist = 3209,
    ArmoryRing = 3300,
    ArmorySoulCrystal = 3400,
    ArmoryWeapon = 3500,

    SaddleBag1 = 4000,
    SaddleBag2 = 4001,
    PremiumSaddleBag1 = 4100,
    PremiumSaddleBag2 = 4101,

    Cosmopouch1 = 5000,
    Cosmopouch2 = 5001,

    Invalid = 9999,

    RetainerPage1 = 10000,
    RetainerPage2 = 10001,
    RetainerPage3 = 10002,
    RetainerPage4 = 10003,
    RetainerPage5 = 10004,
    RetainerPage6 = 10005,
    RetainerPage7 = 10006,
    RetainerEquippedItems = 11000,
    RetainerGil = 12000,
    RetainerCrystals = 12001,
    RetainerMarket = 12002,

    FreeCompanyPage1 = 20000,
    FreeCompanyPage2 = 20001,
    FreeCompanyPage3 = 20002,
    FreeCompanyPage4 = 20003,
    FreeCompanyPage5 = 20004,
    FreeCompanyGil = 22000,
    FreeCompanyCrystals = 22001,

    HousingExteriorAppearance = 25000,
    HousingExteriorPlacedItems = 25001,
    HousingInteriorAppearance = 25002,
    HousingInteriorPlacedItems1 = 25003,
    HousingInteriorPlacedItems2 = 25004,
    HousingInteriorPlacedItems3 = 25005,
    HousingInteriorPlacedItems4 = 25006,
    HousingInteriorPlacedItems5 = 25007,
    HousingInteriorPlacedItems6 = 25008,
    HousingInteriorPlacedItems7 = 25009,
    HousingInteriorPlacedItems8 = 25010,

    HousingExteriorStoreroom = 27000,
    HousingInteriorStoreroom1 = 27001,
    HousingInteriorStoreroom2 = 27002,
    HousingInteriorStoreroom3 = 27003,
    HousingInteriorStoreroom4 = 27004,
    HousingInteriorStoreroom5 = 27005,
    HousingInteriorStoreroom6 = 27006,
    HousingInteriorStoreroom7 = 27007,
    HousingInteriorStoreroom8 = 27008,
}

/// Represents a generic item storage.
pub trait Storage: Sync {
    fn max_slots(&self) -> u32;
    fn num_items(&self) -> u32;
    fn get_slot_mut(&mut self, index: u16) -> &mut Item;
    fn get_slot(&self, index: u16) -> &Item;
}
