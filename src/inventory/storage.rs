use binrw::binrw;

use super::Item;

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
}

/// Represents a generic item storage.
pub trait Storage: Sync {
    fn max_slots(&self) -> u32;
    fn num_items(&self) -> u32;
    fn get_slot_mut(&mut self, index: u16) -> &mut Item;
    fn get_slot(&self, index: u16) -> &Item;
}
