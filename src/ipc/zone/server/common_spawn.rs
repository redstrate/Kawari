use binrw::binrw;
use strum_macros::{Display, EnumIter};

use crate::common::{
    CHAR_NAME_MAX_LENGTH, CustomizeData, EquipDisplayFlag, ObjectId, ObjectTypeId, Position,
    read_quantized_rotation, read_string, write_quantized_rotation, write_string,
};
use bitflags::bitflags;

use super::StatusEffect;

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug, Default)]
pub enum PlayerSubKind {
    #[default]
    Player = 4,
}

// See https://github.com/Caraxi/Dalamud/blob/e6017f96c09b8cde20e02371914ec25cfa989ef7/Dalamud/Game/ClientState/Objects/Enums/BattleNpcSubKind.cs#L6
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, PartialEq, Debug, Default)]
pub enum BattleNpcSubKind {
    #[default]
    None = 0,
    Part = 1,
    Pet = 2,
    Chocobo = 3,
    Enemy = 5,
    NpcPartyMember = 9,
}

#[binrw]
#[derive(Clone, PartialEq, Debug, Default)]
pub enum ObjectKind {
    #[default]
    #[brw(magic = 0u8)]
    None,
    #[brw(magic = 1u8)]
    Player(PlayerSubKind),
    #[brw(magic = 2u8)]
    BattleNpc(BattleNpcSubKind),
    #[brw(magic = 3u8)]
    EventNpc,
    #[brw(magic = 4u8)]
    Treasure,
    #[brw(magic = 5u8)]
    Aetheryte,
    #[brw(magic = 6u8)]
    GatheringPoint,
    #[brw(magic = 7u8)]
    EventObj,
    #[brw(magic = 8u8)]
    Mount,
    #[brw(magic = 9u8)]
    Companion,
    #[brw(magic = 10u8)]
    Retainer,
    #[brw(magic = 11u8)]
    AreaObject,
    #[brw(magic = 12u8)]
    HousingEventObject,
    #[brw(magic = 13u8)]
    Cutscene,
    #[brw(magic = 14u8)]
    MjiObject,
    #[brw(magic = 15u8)]
    Ornament,
    #[brw(magic = 16u8)]
    CardStand,
}

#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[derive(Debug, Clone, Default, PartialEq)]
pub enum CharacterMode {
    None = 0x0,
    #[default]
    Normal = 0x1,
    Dead = 0x2,
}

// From https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/common/Common.h#L212
// TODO: Where did they get this list from??
#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Display, EnumIter)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum GameMasterRank {
    #[default]
    NormalUser,
    GameMaster = 1,
    EventJunior = 3,
    EventSenior = 4,
    Support = 5,
    Senior = 7,
    Debug = 90,
}

#[cfg(all(not(target_family = "wasm"), feature = "server"))]
impl rusqlite::types::FromSql for GameMasterRank {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self::try_from(u8::column_result(value)?).unwrap())
    }
}

#[cfg(all(not(target_family = "wasm"), feature = "server"))]
impl mlua::IntoLua for GameMasterRank {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl TryFrom<u8> for GameMasterRank {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NormalUser),
            1 => Ok(Self::GameMaster),
            3 => Ok(Self::EventJunior),
            4 => Ok(Self::EventSenior),
            5 => Ok(Self::Support),
            7 => Ok(Self::Senior),
            90 => Ok(Self::Debug),
            _ => Err(()),
        }
    }
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct DisplayFlag(pub u32);

impl std::fmt::Debug for DisplayFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

impl From<EquipDisplayFlag> for DisplayFlag {
    fn from(value: EquipDisplayFlag) -> Self {
        let mut new_flag = Self::NONE;
        if value.intersects(EquipDisplayFlag::HIDE_HEAD) {
            new_flag.insert(DisplayFlag::HIDE_HEAD);
        }
        if value.intersects(EquipDisplayFlag::HIDE_WEAPON) {
            new_flag.insert(DisplayFlag::HIDE_WEAPON);
        }
        if value.intersects(EquipDisplayFlag::CLOSE_VISOR) {
            new_flag.insert(DisplayFlag::CLOSE_VISOR);
        }
        if value.intersects(EquipDisplayFlag::HIDE_EARS) {
            new_flag.insert(DisplayFlag::HIDE_EARS);
        }

        new_flag
    }
}

bitflags! {
    impl DisplayFlag : u32 {
        const NONE = 0x000;
        const ACTIVE_STANCE = 0x001;
        const INVISIBLE = 0x020;
        const HIDE_HEAD = 0x040;
        const HIDE_WEAPON = 0x80;
        const FADED = 0x100;
        const CLOSE_VISOR = 0x800;
        const UNK1 = 0x40000;
        const HIDE_EARS = 0x100000;
    }
}

impl Default for DisplayFlag {
    fn default() -> Self {
        Self::NONE
    }
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct CommonSpawn {
    pub target_id: ObjectTypeId,
    pub u6: u32,
    pub u7: u32,
    pub main_weapon_model: u64,
    pub sec_weapon_model: u64,
    pub craft_tool_model: u64,

    pub u14: u32,
    pub u15: u32,
    /// See BNpcBase Excel sheet
    pub bnpc_base: u32,
    /// See BNpcName Excel sheet
    pub bnpc_name: u32,
    pub unk3: [u8; 8],
    pub director_id: u32, // FIXME: i think the next three are in the wrong order
    pub spawner_id: ObjectId,
    pub parent_actor_id: ObjectId,
    pub hp_max: u32,
    pub hp_curr: u32,
    pub display_flags: DisplayFlag,
    pub fate_id: u16, // assumed
    pub mp_curr: u16,
    pub mp_max: u16,
    pub unk: u16,
    /// See ModelChara Excel sheet
    pub model_chara: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub current_mount: u16, // assumed
    pub active_minion: u16, // assumed
    pub u23: u8,            // assumed
    pub u24: u8,            // assumed
    pub u25: u8,            // assumed
    pub u26: u8,            // assumed
    pub u27: u8,            // assumed
    pub u28: u8,            // assumed
    /// Must be unique for each actor.
    pub spawn_index: u8,
    #[brw(pad_size_to = 2)] // for modes that don't have a param
    pub mode: CharacterMode,
    #[brw(pad_size_to = 2)] // for kinds that don't have a param
    pub object_kind: ObjectKind,
    pub voice: u8,
    pub unk27: u8,
    /// See Battalion Excel sheet. Used for determing whether it's friendy or an enemy.
    pub battalion: u8,
    pub level: u8,
    /// See ClassJob Excel sheet.
    pub class_job: u8,
    pub unk28: u8,
    pub unk29: u8,
    pub mount_head: u8,
    pub mount_body: u8,
    pub mount_feet: u8,
    pub mount_color: u8,
    pub scale: u8,
    pub element_data: [u8; 6],
    pub padding2: [u8; 3],
    pub effect: [StatusEffect; 30],
    pub pos: Position,
    pub models: [u32; 10],
    pub unknown6_58: [u8; 10],
    pub padding3: [u8; 4],
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    pub look: CustomizeData,
    #[br(count = 6)]
    #[bw(pad_size_to = 6)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,
}
