//! Content director related types.

use binrw::binrw;
use strum_macros::{Display, EnumIter, FromRepr};

use crate::common::TerritoryIntendedUse;

/// Also refer to the DirectorType Excel sheet.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumIter, FromRepr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DirectorType {
    BattleLeve = 0x8001,
    GatheringLeve = 0x8002,
    /// Used for dungeons, including Deep Dungeons.
    InstanceContent = 0x8003,
    PublicContent = 0x8004,
    QuestBattle = 0x8006,
    CompanyLeve = 0x8007,
    TreasureHunt = 0x8009,
    GoldSaucer = 0x800A,
    CompanyCraft = 0x800B,
    SkyIsland = 0x800C,
    DpsChallenge = 0x800D,
    MassivePcContent = 0x800E,
    /// Used in open world zones that have FATEs.
    Fate = 0x801A,
}

impl DirectorType {
    /// Determine the correct director for use in this zone.
    pub fn from_intended_use(intended_use: TerritoryIntendedUse) -> Option<Self> {
        match intended_use {
            TerritoryIntendedUse::OpenWorld => Some(Self::Fate),
            TerritoryIntendedUse::Dungeon => Some(Self::InstanceContent),
            TerritoryIntendedUse::DeepDungeon => Some(Self::InstanceContent),
            _ => None,
        }
    }
}

#[cfg(feature = "server")]
impl mlua::IntoLua for DirectorType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl TryFrom<u32> for DirectorType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_repr(value).ok_or(())
    }
}

#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorEvent {
    /// Shows "Duty Commenced", and starts the clock ticking down. `arg` is the number of seconds the duty should last.
    #[brw(magic = 0x40000001u32)]
    DutyCommence,
    /// Seems to be in response to the Sync trigger. Arg seems to always be 1.
    #[brw(magic = 0x80000000u32)]
    SyncResponse,
    Unknown(u32),
}

#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorTrigger {
    /// When the player requests to summon a striking dummy. `arg` always seems to be 1.
    #[brw(magic = 0x40000006u32)]
    SummonStrikingDummy,
    /// Unknown purpose.
    #[brw(magic = 0x80000000u32)]
    Sync,
    Unknown(u32),
}
