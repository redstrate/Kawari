use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter, FromRepr};

use super::{Item, Storage};

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumIter, FromRepr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum CrystalKind {
    FireShard = 2,
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
}

impl mlua::IntoLua for CrystalKind {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl mlua::FromLua for CrystalKind {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Integer(integer) => Ok(Self::from_repr(integer as u32).unwrap()),
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize, Debug)]
pub struct CrystalsStorage {
    pub fire_shard: Item,
    pub ice_shard: Item,
    pub wind_shard: Item,
    pub earth_shard: Item,
    pub lightning_shard: Item,
    pub water_shard: Item,

    pub fire_crystal: Item,
    pub ice_crystal: Item,
    pub wind_crystal: Item,
    pub earth_crystal: Item,
    pub lightning_crystal: Item,
    pub water_crystal: Item,

    pub fire_cluster: Item,
    pub ice_cluster: Item,
    pub wind_cluster: Item,
    pub earth_cluster: Item,
    pub lightning_cluster: Item,
    pub water_cluster: Item,
}

impl CrystalsStorage {
    pub fn get_slot_for_id(id: CrystalKind) -> u16 {
        id as u16 - 2
    }

    pub fn get_item_for_id(&mut self, id: CrystalKind) -> &mut Item {
        self.get_slot_mut(Self::get_slot_for_id(id))
    }
}

impl Default for CrystalsStorage {
    fn default() -> Self {
        Self {
            fire_shard: Item {
                item_id: CrystalKind::FireShard as u32,
                ..Default::default()
            },
            ice_shard: Item {
                item_id: CrystalKind::IceShard as u32,
                ..Default::default()
            },
            wind_shard: Item {
                item_id: CrystalKind::WindShard as u32,
                ..Default::default()
            },
            earth_shard: Item {
                item_id: CrystalKind::EarthShard as u32,
                ..Default::default()
            },
            lightning_shard: Item {
                item_id: CrystalKind::LightningShard as u32,
                ..Default::default()
            },
            water_shard: Item {
                item_id: CrystalKind::WaterShard as u32,
                ..Default::default()
            },

            fire_crystal: Item {
                item_id: CrystalKind::FireCrystal as u32,
                ..Default::default()
            },
            ice_crystal: Item {
                item_id: CrystalKind::IceCrystal as u32,
                ..Default::default()
            },
            wind_crystal: Item {
                item_id: CrystalKind::WindCrystal as u32,
                ..Default::default()
            },
            earth_crystal: Item {
                item_id: CrystalKind::EarthCrystal as u32,
                ..Default::default()
            },
            lightning_crystal: Item {
                item_id: CrystalKind::LightningCrystal as u32,
                ..Default::default()
            },
            water_crystal: Item {
                item_id: CrystalKind::WaterCrystal as u32,
                ..Default::default()
            },

            fire_cluster: Item {
                item_id: CrystalKind::FireCluster as u32,
                ..Default::default()
            },
            ice_cluster: Item {
                item_id: CrystalKind::IceCluster as u32,
                ..Default::default()
            },
            wind_cluster: Item {
                item_id: CrystalKind::WindCluster as u32,
                ..Default::default()
            },
            earth_cluster: Item {
                item_id: CrystalKind::EarthCluster as u32,
                ..Default::default()
            },
            lightning_cluster: Item {
                item_id: CrystalKind::LightningCluster as u32,
                ..Default::default()
            },
            water_cluster: Item {
                item_id: CrystalKind::WaterCluster as u32,
                ..Default::default()
            },
        }
    }
}

impl Storage for CrystalsStorage {
    fn max_slots(&self) -> u32 {
        18
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        match index {
            0 => &mut self.fire_shard,
            1 => &mut self.ice_shard,
            2 => &mut self.wind_shard,
            3 => &mut self.earth_shard,
            4 => &mut self.lightning_shard,
            5 => &mut self.water_shard,

            6 => &mut self.fire_crystal,
            7 => &mut self.ice_crystal,
            8 => &mut self.wind_crystal,
            9 => &mut self.earth_crystal,
            10 => &mut self.lightning_crystal,
            11 => &mut self.water_crystal,

            12 => &mut self.fire_cluster,
            13 => &mut self.ice_cluster,
            14 => &mut self.wind_cluster,
            15 => &mut self.earth_cluster,
            16 => &mut self.lightning_cluster,
            17 => &mut self.water_cluster,

            _ => &mut self.fire_shard,
        }
    }

    fn get_slot(&self, index: u16) -> &Item {
        match index {
            0 => &self.fire_shard,
            1 => &self.ice_shard,
            2 => &self.wind_shard,
            3 => &self.earth_shard,
            4 => &self.lightning_shard,
            5 => &self.water_shard,

            6 => &self.fire_crystal,
            7 => &self.ice_crystal,
            8 => &self.wind_crystal,
            9 => &self.earth_crystal,
            10 => &self.lightning_crystal,
            11 => &self.water_crystal,

            12 => &self.fire_cluster,
            13 => &self.ice_cluster,
            14 => &self.wind_cluster,
            15 => &self.earth_cluster,
            16 => &self.lightning_cluster,
            17 => &self.water_cluster,

            _ => &self.fire_shard,
        }
    }
}
