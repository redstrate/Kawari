use binrw::binrw;
use strum_macros::{Display, EnumIter, FromRepr};

use crate::common::{
    CHAR_NAME_MAX_LENGTH, CharacterMode, CrestData, CustomizeData, EquipDisplayFlag, HandlerId,
    LegacyEquipmentModelId, ObjectId, ObjectTypeId, Position, WeaponModelId,
    read_quantized_rotation, read_string, write_quantized_rotation, write_string,
};
use bitflags::bitflags;

use super::StatusEffect;

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum PlayerSubKind {
    /// Used for players.
    #[default]
    Player = 4,
}

// See https://github.com/Caraxi/Dalamud/blob/e6017f96c09b8cde20e02371914ec25cfa989ef7/Dalamud/Game/ClientState/Objects/Enums/BattleNpcSubKind.cs#L6
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum BattleNpcSubKind {
    #[default]
    None = 0,
    Part = 1,
    Pet = 2,
    Chocobo = 3,
    /// Regular enemies.
    Enemy = 5,
    NpcPartyMember = 9,
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum ObjectKind {
    #[default]
    #[brw(magic = 0u8)]
    None,
    /// Regular players. They usually have blue nameplates.
    #[brw(magic = 1u8)]
    Player(PlayerSubKind),
    /// NPCs that you fight with. They usually have yellow/red nameplates.
    #[brw(magic = 2u8)]
    BattleNpc(BattleNpcSubKind),
    /// NPCs that are friendly/non-interactable. They usually have green nameplates.
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
    /// Supposedly used for VFX/AoE effects.
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
    Unknown(u8),
}

// From https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/common/Common.h#L212
// TODO: Where did they get this list from??
#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Display, EnumIter, FromRepr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Integer))]
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

#[cfg(feature = "server")]
impl mlua::IntoLua for GameMasterRank {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

#[cfg(feature = "server")]
impl diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite>
    for GameMasterRank
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(*self as i32);
        Ok(diesel::serialize::IsNull::No)
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite>
    for GameMasterRank
{
    fn from_sql(
        mut integer: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        Ok(GameMasterRank::from_repr(integer.read_integer() as u8).unwrap())
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
    /// Initial target for this character.
    pub target_id: ObjectTypeId,
    /// Free Company Crest data.
    pub crest_data: CrestData,
    /// Model ID for their main weapon.
    pub main_weapon_model: WeaponModelId,
    /// Model ID for their secondary weapon.
    pub sec_weapon_model: WeaponModelId,
    /// Model ID for their craft weapon.
    pub craft_tool_model: WeaponModelId,
    /// Unknown purpose, but seen filled with enemy data that a Player has aggro'd. Also seen for Quests that spawn an enemy the player must fight (filled on the enemy's CommonSpawn.)
    pub combat_tagger_id: ObjectTypeId,
    /// See BNpcBase/ENpcBase Excel sheet.
    pub base_id: u32,
    /// See BNpcName/ENpcResident Excel sheet.
    pub name_id: u32,
    /// Refers to the original game object ID associated with this character.
    pub layout_id: u32,
    /// Index into the Pet Excel sheet. Seems only relevant for Carbuncles and other "pets".
    pub pet_id: u32,
    /// Which director spawned and is managing this actor, if any.
    pub handler_id: HandlerId,
    /// Seen set to the player owners of Carbuncles and Chocobos.
    pub owner_id: ObjectId,
    /// Unknown purpose.
    pub tether_target_id: ObjectId,
    /// Their maximum HP.
    pub max_health_points: u32,
    /// Their current HP.
    pub health_points: u32,
    /// Initial display flags for this character.
    pub display_flags: DisplayFlag,
    /// Index into the FATE sheet.
    pub fate_id: u16,
    /// Their current MP/CP etc.
    pub resource_points: u16,
    /// Their maximum MP/CP etc.
    pub max_resource_points: u16,
    /// Unknown purpose.
    pub unk: u16,
    /// See ModelChara Excel sheet.
    pub model_chara: u16,
    /// Their initial rotation.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// Index into the Mount Excel sheet.
    pub current_mount: u16,
    /// Index into the Companion Excel sheet.
    pub active_minion: u16,
    /// Unknown purpose.
    pub follow_mount_id: u16,
    /// Unknown purpose.
    pub ornament_id: u16,
    /// Unknown purpose.
    pub tether_id: u16,
    /// Unique for each actor, and is used to eventually free them from the allocator.
    pub spawn_index: u8,
    /// What mode this actor should initially be in.
    pub mode: CharacterMode,
    /// The argument for `mode`.
    pub mode_arg: u8,
    #[brw(pad_size_to = 2)] // for kinds that don't have a param
    pub object_kind: ObjectKind,
    /// The character's voice.
    pub voice: u8,
    /// For the Free Company crest.
    pub crest_bitfield: u8,
    /// See Battalion Excel sheet.
    pub battalion: u8,
    /// The level of this character.
    pub level: u8,
    /// See ClassJob Excel sheet.
    pub class_job: u8,
    /// Unknown purpose.
    pub event_state: u8,
    /// Unknown purpose.
    pub unk79: u8,
    /// Unknown purpose.
    pub combat_tag_type: u8,
    /// Unknown purpose.
    pub mount_head: u8,
    /// Unknown purpose.
    pub mount_body: u8,
    /// Unknown purpose.
    pub mount_feet: u8,
    /// Unknown purpose.
    pub mount_color: u8,
    /// Unknown purpose.
    pub status_loop_vfx_id: u8,
    /// Unknown purpose, seems to be used in at least Bozja to signify the rank of enemies.
    pub foray_rank: u8,
    /// Unknown purpose.
    pub foray_element: u8,
    /// Unknown purpose.
    pub model_scale_id: u8,
    /// Unknown purpose.
    pub model_state: u8,
    /// Unknown purpose.
    pub model_attribute_flags: u8,
    /// Unknown purpose.
    #[brw(pad_after = 2)] // probably empty, not read by the client
    pub animation_state: u8,
    /// Their status effects.
    pub status_effects: [StatusEffect; 30],
    /// Their initial position.
    pub position: Position,
    /// Equipment model IDs if humanoid.
    pub models: [LegacyEquipmentModelId; 10],
    /// Second dye stains for the given `models`.
    pub second_model_stain_ids: [u8; 10],
    /// Unknown purpose.
    pub glasses_ids: [u16; 2],
    /// Their name, for non-player characters this is the usually the original Japanese name.
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    /// Customization data for humanoid characters.
    pub look: CustomizeData,
    /// Their short Free Company tag.
    #[br(count = 6)]
    #[bw(pad_size_to = 6)]
    #[brw(pad_after = 6)] // i think is empty?
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,
}
