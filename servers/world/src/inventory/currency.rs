use kawari::common::ItemInfo;

use serde::{Deserialize, Serialize};

use super::{Item, Storage};

// TODO: look at TomestonesItem Excel sheet for inventory slots

// TODO: Add society currencies, this is just a good baseline
#[repr(u32)]
pub enum CurrencyKind {
    Gil = 1,
    FireShard,
    IceShard,
    WindShard,
    EarthShard,
    LightningShard,
    WaterShard,
    FireCrystal,
    IceCrystal,
    WindCrystal,
    EarthCrystal,
    LightningCrystal,
    WaterCrystal,
    FireCluster,
    IceCluster,
    WindCluster,
    EarthCluster,
    LightningCluster,
    WaterCluster,
    StormSeal,
    SerpentSeal,
    FlameSeal,
    WolfMark = 25,
    AlliedSeal = 27,
    TomestonePoetics,
    MGP = 29,
    TomestoneHelio = 47,
    TomestoneMaths,
    CenturioSeal = 10307,
    Venture = 21072,
    SackOfNuts = 26533,
    TrophyCrystal = 36656,
}

#[derive(Clone, Copy, Deserialize, Serialize, Debug)]
pub struct CurrencyStorage {
    pub gil: Item,
    pub flame_seal: Item, // TODO: rename to something company agnostic
    pub wolf_mark: Item,
    pub tomestone_poetics: Item,
    pub tomestone_mathematics: Item, // TODO: rename season agnostic
    pub allied_seal: Item,
    pub mgp: Item,
    pub tomestone_heliometry: Item, // TODO: rename season agnostic
    pub dummy: Item,
}

impl CurrencyStorage {
    pub fn get_slot_for_id(id: u32) -> u16 {
        // TODO: duplicated between Rust and Lua
        match id {
            1 => 0,
            29 => 9,
            _ => unimplemented!(),
        }
    }

    pub fn get_item_for_id(&mut self, id: u32) -> &mut Item {
        self.get_slot_mut(Self::get_slot_for_id(id))
    }
}

impl Default for CurrencyStorage {
    fn default() -> Self {
        Self {
            gil: Item::new(
                ItemInfo {
                    id: CurrencyKind::Gil as u32,
                    ..Default::default()
                },
                0,
            ),
            flame_seal: Item::new(
                ItemInfo {
                    id: CurrencyKind::FlameSeal as u32,
                    ..Default::default()
                },
                0,
            ),
            wolf_mark: Item::new(
                ItemInfo {
                    id: CurrencyKind::WolfMark as u32,
                    ..Default::default()
                },
                0,
            ),
            tomestone_poetics: Item::new(
                ItemInfo {
                    id: CurrencyKind::TomestonePoetics as u32,
                    ..Default::default()
                },
                0,
            ),
            tomestone_mathematics: Item::new(
                ItemInfo {
                    id: CurrencyKind::TomestoneMaths as u32,
                    ..Default::default()
                },
                0,
            ),
            allied_seal: Item::new(
                ItemInfo {
                    id: CurrencyKind::AlliedSeal as u32,
                    ..Default::default()
                },
                0,
            ),
            mgp: Item::new(
                ItemInfo {
                    id: CurrencyKind::MGP as u32,
                    ..Default::default()
                },
                0,
            ),
            tomestone_heliometry: Item::new(
                ItemInfo {
                    id: CurrencyKind::TomestoneHelio as u32,
                    ..Default::default()
                },
                0,
            ),
            dummy: Item::default(),
        }
    }
}

impl Storage for CurrencyStorage {
    fn max_slots(&self) -> u32 {
        11
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        match index {
            0 => &mut self.gil,
            3 => &mut self.flame_seal,
            4 => &mut self.wolf_mark,
            6 => &mut self.tomestone_poetics,
            7 => &mut self.tomestone_mathematics,
            8 => &mut self.allied_seal,
            9 => &mut self.mgp,
            10 => &mut self.tomestone_heliometry,
            _ => &mut self.dummy,
        }
    }

    fn get_slot(&self, index: u16) -> &Item {
        match index {
            0 => &self.gil,
            3 => &self.flame_seal,
            4 => &self.wolf_mark,
            6 => &self.tomestone_poetics,
            7 => &self.tomestone_mathematics,
            8 => &self.allied_seal,
            9 => &self.mgp,
            10 => &self.tomestone_heliometry,
            _ => &self.dummy,
        }
    }
}
