use serde::{Deserialize, Serialize};

use super::{Item, Storage};

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
    AlliedSeal,
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
}

impl Default for CurrencyStorage {
    fn default() -> Self {
        Self {
            gil: Item::new(0, CurrencyKind::Gil as u32),
        }
    }
}

impl Storage for CurrencyStorage {
    fn max_slots(&self) -> u32 {
        1
    }

    fn num_items(&self) -> u32 {
        1
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        match index {
            0 => &mut self.gil,
            _ => panic!("{index} is not a valid src_container_index?!?"),
        }
    }

    fn get_slot(&self, index: u16) -> &Item {
        match index {
            0 => &self.gil,
            _ => panic!("{index} is not a valid src_container_index?!?"),
        }
    }
}
