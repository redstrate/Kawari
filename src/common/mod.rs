use bitflags::bitflags;
use serde::Deserialize;
use std::{
    ffi::{CStr, CString},
    time::{SystemTime, UNIX_EPOCH},
};
use strum_macros::{Display, EnumIter, FromRepr};

mod customize_data;
use binrw::binrw;
pub use customize_data::CustomizeData;

mod position;
pub use position::Position;

mod gamedata;
pub use gamedata::GameData;
pub use gamedata::{InstanceContentType, ItemInfo, ItemInfoQuery, TerritoryNameKind};

pub mod workdefinitions;

mod bitmask;
pub use bitmask::Bitmask;

#[binrw]
#[brw(little)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct ObjectId(pub u32);

impl ObjectId {
    /// Returns true if it points to a *valid-looking* object id.
    pub fn is_valid(&self) -> bool {
        *self != INVALID_OBJECT_ID
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        INVALID_OBJECT_ID
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "INVALID_ACTOR")
        }
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjectId ({self})")
    }
}

// This is unrelated to the ObjectKind struct as named by ClientStructs; it's used for ACT::SetTarget, ACT::Emote, and probably more.
// Instead it correlates to the Type field in the GameObjectId client struct.
// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Object/GameObject.cs#L230
#[binrw]
#[brw(repr = u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectTypeKind {
    /// Everything that has a proper entity/actor ID.
    #[default]
    None = 0,
    /// Orchestrions, static NPCs in towns, etc., and possibly more.
    EObjOrNpc = 1,
    /// Unclear when this is used, more research is needed.
    /// ClientStructs describes it as "if (BaseId == 0 || (ObjectIndex >= 200 && ObjectIndex < 244)) ObjectId = ObjectIndex, Type = 2"
    Unknown = 2,
    /// Player-summoned minions (not to be confused with chocobos or other bnpc pets), and possibly more.
    Minion = 4,
}

// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Object/GameObject.cs#L238
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTypeId {
    pub object_id: ObjectId,
    pub object_type: ObjectTypeKind,
}

impl Default for ObjectTypeId {
    fn default() -> Self {
        Self {
            object_id: INVALID_OBJECT_ID,
            object_type: ObjectTypeKind::None,
        }
    }
}

/// An invalid actor/object id.
pub const INVALID_OBJECT_ID: ObjectId = ObjectId(0xE0000000);

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

/// Maximum length of most (all?) chat messages.
pub const MESSAGE_MAX_LENGTH: usize = 1024;

pub(crate) fn read_bool_from<T: std::convert::From<u8> + std::cmp::PartialEq>(x: T) -> bool {
    x == T::from(1u8)
}

pub(crate) fn write_bool_as<T: std::convert::From<u8>>(x: &bool) -> T {
    if *x { T::from(1u8) } else { T::from(0u8) }
}

pub(crate) fn read_string(byte_stream: Vec<u8>) -> String {
    // TODO: This can surely be made better, but it seems to satisfy some strange edge cases. If there are even more found, then we should probably rewrite this function altogether.
    let Ok(result) = CStr::from_bytes_until_nul(&byte_stream) else {
        if let Ok(str) = String::from_utf8(byte_stream.clone()) {
            return str.trim_matches(char::from(0)).to_string(); // trim \0 from the end of strings
        } else {
            tracing::error!(
                "Found an edge-case where both CStr::from_bytes_until_nul and String::from_utf8 failed: {:#?}",
                byte_stream.clone()
            );
            return String::default();
        }
    };

    let Ok(result) = result.to_str().to_owned() else {
        tracing::error!("Unable to make this CStr an owned string, what happened?");
        return String::default();
    };

    result.to_string()
}

pub(crate) fn write_string(str: &String) -> Vec<u8> {
    let c_string = CString::new(&**str).unwrap();
    c_string.as_bytes_with_nul().to_vec()
}

/// Converts a quantized rotation to degrees in f32
pub(crate) fn read_quantized_rotation(quantized: u16) -> f32 {
    let max = u16::MAX as f32;
    let pi = std::f32::consts::PI;

    quantized as f32 / max * (2.0 * pi) - pi
}

/// Converts a rotation (in degrees) to
pub(crate) fn write_quantized_rotation(quantized: &f32) -> u16 {
    let max = u16::MAX as f32;
    let pi = std::f32::consts::PI;

    (((quantized + pi) / (2.0 * pi)) * max) as u16
}

pub(crate) fn read_packed_float(packed: u16) -> f32 {
    ((packed as f32 / 0.327675) / 100.0) - 1000.0
}

pub(crate) fn write_packed_float(float: &f32) -> u16 {
    (((float + 1000.0) * 100.0) * 0.327675) as u16
}

pub(crate) fn read_packed_position(packed: [u16; 3]) -> Position {
    Position {
        x: read_packed_float(packed[0]),
        y: read_packed_float(packed[1]),
        z: read_packed_float(packed[2]),
    }
}

pub(crate) fn write_packed_position(pos: &Position) -> [u16; 3] {
    [
        write_packed_float(&pos.x),
        write_packed_float(&pos.y),
        write_packed_float(&pos.z),
    ]
}

/// Get the number of seconds since UNIX epoch.
pub fn timestamp_secs() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_secs()
        .try_into()
        .unwrap()
}

/// Get the number of milliseconds since UNIX epoch.
pub fn timestamp_msecs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_millis()
        .try_into()
        .unwrap()
}

/// Gets the initial zone for a given city-state id
pub fn determine_initial_starting_zone(citystate_id: u8) -> u16 {
    match citystate_id {
        // Limsa
        1 => 181,
        // Gridania
        2 => 183,
        // Ul'dah
        3 => 182,
        _ => panic!("This is not a valid city-state id!"),
    }
}

pub fn value_to_flag_byte_index_value(in_value: u32) -> (u8, u16) {
    let bit_index = in_value % 8;
    (1 << bit_index, (in_value / 8) as u16)
}

pub fn value_to_flag_byte_index_value_quests(in_value: u32) -> (u8, u16) {
    let bit_index = in_value % 8;
    (0x80 >> bit_index, (in_value / 8) as u16)
}

pub struct Attributes {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
}

#[binrw]
#[brw(repr(u32))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DistanceRange {
    Normal = 0x0,
    Extended = 0x1,
    Maximum = 0x2,
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EquipDisplayFlag(pub u16);

impl std::fmt::Debug for EquipDisplayFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

bitflags! {
    impl EquipDisplayFlag : u16 {
        const NONE = 0x00;
        const HIDE_LEGACY_MARK = 0x04;
        const HIDE_HEAD = 0x01;
        const HIDE_WEAPON = 0x02;
        const UNK1 = 0x04;
        const UNK2 = 0x08;
        const UNK3 = 0x10;
        const UNK4 = 0x20;
        const CLOSE_VISOR = 0x40;
        const HIDE_EARS = 0x80;
    }
}

impl Default for EquipDisplayFlag {
    fn default() -> Self {
        Self::NONE
    }
}

/// The client sends this to inform the server (and other clients) about the animation its player is performing while moving.
/// Multiple can be set at once, e.g. Strafing and walking at the same time.
// TODO: Why does RUNNING display as a comma in PacketAnalyzer?
#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MoveAnimationType(u8);

bitflags! {
    impl MoveAnimationType : u8 {
        /// The player is running.
        const RUNNING = 0x00;
        /// Unknown: seems to be the start of the regular run animation and loops the first few frames endlessly.
        const UNKNOWN = 0x01;
        /// The player is walking or landing from a jump/fall (MoveAnimationState::ENTER_COLLISION is set).
        const WALKING_OR_LANDING = 0x02;
        /// The player is strafing.
        const STRAFING = 0x04;
        /// The player is being knocked back by an attack or some other force.
        const KNOCKBACK = 0x08;
        /// The player is jumping.
        const JUMPING = 0x10;
        /// The player has begun falling after jumping.
        const FALLING = 0x20;
    }
}

impl Default for MoveAnimationType {
    fn default() -> Self {
        Self::RUNNING
    }
}

impl std::fmt::Debug for MoveAnimationType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// The client sends this to inform the server about its player's current state when moving around.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveAnimationState {
    #[default]
    /// No special state is in play.
    None = 0,
    /// The player fell off something, or they began jumping.
    LeavingCollision = 1,
    /// The player landed back on the ground.
    EnteringCollision = 2,
    /// The player reached the apex of their jump, and began to fall.
    StartFalling = 4,
}

/// The client sends this to inform the server about its player's current state when jumping.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JumpState {
    /// The player is descending back to the ground, or isn't jumping at all.
    #[default]
    NoneOrFalling = 0,
    /// The player is ascending to the apex of the jump.
    Ascending = 16,
}

/// The server responds with these values to set the correct speed when informing other clients about how quickly to animate the movements.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveAnimationSpeed {
    Walking = 20,
    #[default]
    Running = 60,
    Jogging = 72,
    Sprinting = 78,
}

// Just based off of Sapphire's version
pub fn euler_to_direction(euler: [f32; 3]) -> f32 {
    let sin_z = f32::sin(euler[2]);
    let cos_z = f32::cos(euler[2]);
    let sin_y = f32::sin(euler[1]);
    let cos_y = f32::cos(euler[1]);
    let sin_x = f32::sin(euler[0]);
    let cos_x = f32::cos(euler[0]);

    let m00 = cos_z * cos_y;
    let m02 = sin_z * sin_x + (-cos_z * sin_y) * cos_x;

    let m10 = -sin_z * cos_y;
    let m12 = cos_z * sin_x + sin_z * sin_y * cos_x;

    let m20 = sin_y;
    let m22 = cos_y * cos_x;

    let vector = [0.0, 0.0, 1.0];
    let dst_x = vector[2] * m20 + vector[0] * m00 + vector[1] * m10;
    let dst_z = vector[2] * m22 + vector[0] * m02 + vector[1] * m12;

    let squared = dst_z * dst_z + dst_x * dst_x;
    let v1;
    let v2;

    if squared > 0.00000011920929 {
        let mut ret = f32::sqrt(squared);
        ret = -((squared * ret) * ret - 1.0) * (0.5 * ret) + ret;
        ret = -((squared * ret) * ret - 1.0) * (0.5 * ret) + ret;
        v1 = dst_z * (-(((squared * ret) * ret) - 1.0) * (0.5 * ret) + ret);
        v2 = dst_x * (-(((squared * ret) * ret) - 1.0) * (0.5 * ret) + ret);
    } else {
        v1 = 0.0;
        v2 = 0.0;
    }

    f32::atan2(v2, v1)
}

/// This allows us (and probably the client as well) to determine which event belongs to each sheet, or type of NPC.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumIter, FromRepr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum EventHandlerType {
    /// See Quests Excel sheet.
    Quests = 1,
    /// See Warp Excel sheet.
    Warp = 2,
    /// See GilShop Excel sheet.
    GilShop = 4,
    /// See Aetheryte Excel sheet.
    Aetheryte = 5,
    /// See GuildleveAssignment Excel sheet.
    GuildLeveAssignment = 6,
    /// See DefaultTalk Excel sheet.
    DefaultTalk = 9,
    /// See CustomTalk Excel sheet.
    CustomTalk = 11,
    /// See CraftLeve Excel sheet.
    CraftLevel = 14,
    /// See ChocoboTaxiStand Excel sheet.
    ChocoboTaxiStand = 18,
    /// See Opening Excel sheet.
    Opening = 19,
    /// Used for housing.
    ExitRange = 20,
    /// See GCShop Excel sheet.
    GcShop = 22,
    /// See GuildOrderGuide Excel sheet.
    GuildOrderGuide = 23,
    /// See GuildOrderOfficer Excel sheet.
    GuildOrderOfficer = 24,
    /// See ContentNpc Excel sheet.
    ContentNpc = 25,
    /// See Story Excel sheet.
    Story = 26,
    /// See SpecialShop Excel sheet.
    SpecialShop = 27,
    // See SwitchTalk Excel sheet.
    SwitchTalk = 31,
    /// See TripleTriad Excel sheet.
    TripleTriad = 35,
    /// See GoldSaucerArcadeMachine Excel sheet.
    GoldSaucerArcadeMachine = 36,
    /// See FccShop Excel sheet.
    FccShop = 42,
    /// See DpsChallengeOfficer Excel sheet.
    DpsChallengeOfficer = 47,
    /// See TopicSelect Excel sheet.
    TopicSelect = 50,
    /// See LotteryExchangeShop Excel sheet.
    LotteryExchangeShop = 52,
    /// See DisposalShop Excel sheet.
    DisposalShop = 53,
    /// See PreHandler Excel sheet.
    PreHandler = 54,
    /// See InclusionShop Excel sheet.
    InclusionShop = 58,
    /// See CollectablesShop Excel sheet.
    CollectablesShop = 59,
    /// See EventPathMove Excel sheet.
    EventPathMove = 61,
    /// These are used for the Solution Nine teleporter pads, for example. See EventGimmickPathMove Excel sheet.
    EventGimmickPathMove = 64,
}

#[cfg(all(not(target_family = "wasm"), feature = "server"))]
impl mlua::IntoLua for EventHandlerType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl TryFrom<u32> for EventHandlerType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_repr(value).ok_or(())
    }
}

/// Calculates the maximum achievable level for a given expansion.
/// For example, 0 (ARR) would be 50.
pub fn calculate_max_level(expansion: u8) -> u8 {
    50 + (expansion * 10)
}

/// Which language the client indicates as its primary language.
/// Not to be confused with physis::common::Language.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum ClientLanguage {
    #[default]
    Japanese = 0,
    English = 1,
    German = 2,
    French = 3,
}

#[macro_export]
macro_rules! web_templates_dir {
    ($rel_path:literal) => {
        concat!("resources/web/templates/", $rel_path)
    };
}

#[macro_export]
macro_rules! web_static_dir {
    ($rel_path:literal) => {
        concat!("resources/web/static/", $rel_path)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    const DATA: [u8; 2] = [0u8, 1u8];

    #[test]
    fn read_bool_u8() {
        assert!(!read_bool_from::<u8>(DATA[0]));
        assert!(read_bool_from::<u8>(DATA[1]));
    }

    #[test]
    fn write_bool_u8() {
        assert_eq!(write_bool_as::<u8>(&false), DATA[0]);
        assert_eq!(write_bool_as::<u8>(&true), DATA[1]);
    }

    // "FOO\0"
    const STRING_DATA: [u8; 4] = [0x46u8, 0x4Fu8, 0x4Fu8, 0x0u8];

    // "Helper Name" followed by numerous zeroes and garbage data at the end, intended to trip up our old string parser
    const MALFORMED_STRING_DATA: [u8; 32] = [
        0x48, 0x65, 0x6C, 0x70, 0x65, 0x72, 0x20, 0x4E, 0x61, 0x6D, 0x65, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0xD8, 0x33,
    ];

    // "Edda Miller" with garbage at the very end of the field
    const MALFORMED_SECOND_EXAMPLE: [u8; 32] = [
        69, 100, 100, 97, 32, 77, 105, 108, 108, 101, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 40, 86,
    ];

    #[test]
    fn read_string() {
        // The nul terminator is supposed to be removed
        assert_eq!(
            crate::common::read_string(STRING_DATA.to_vec()),
            "FOO".to_string()
        );

        // Will this malformed name string trip up our parser? It makes our previous one blow up.
        assert_eq!(
            crate::common::read_string(MALFORMED_STRING_DATA.to_vec()),
            "Helper Name".to_string()
        );

        assert_eq!(
            crate::common::read_string(MALFORMED_SECOND_EXAMPLE.to_vec()),
            "Edda Miller".to_string()
        );
    }

    #[test]
    fn write_string() {
        // Supposed to include the nul terminator
        assert_eq!(
            crate::common::write_string(&"FOO".to_string()),
            STRING_DATA.to_vec()
        );
    }

    #[test]
    fn quantized_rotations() {
        assert_eq!(read_quantized_rotation(0), -std::f32::consts::PI);
        assert_eq!(read_quantized_rotation(65535), std::f32::consts::PI);

        assert_eq!(write_quantized_rotation(&-std::f32::consts::PI), 0);
        assert_eq!(write_quantized_rotation(&std::f32::consts::PI), 65535);
    }

    #[test]
    fn packed_floats() {
        assert_eq!(read_packed_float(32931), 4.989685);
        assert_eq!(write_packed_float(&5.0), 32931);
    }

    #[test]
    fn value_flag() {
        assert_eq!(value_to_flag_byte_index_value(0), (1, 0));
        assert_eq!(value_to_flag_byte_index_value(32), (1, 4));
        assert_eq!(value_to_flag_byte_index_value(64), (1, 8));
    }

    #[test]
    fn test_max_ex_levels() {
        assert_eq!(calculate_max_level(0), 50); // A Realm Reborn
        assert_eq!(calculate_max_level(1), 60); // Heavensward
        assert_eq!(calculate_max_level(2), 70); // Stormblood
        assert_eq!(calculate_max_level(3), 80); // Shadowbringers
        assert_eq!(calculate_max_level(4), 90); // Endwalker
        assert_eq!(calculate_max_level(5), 100); // Dawntrail
    }

    #[test]
    fn test_euler_to_direction() {
        assert_eq!(euler_to_direction([0.0, 0.0, 0.0]), 0.0);
        assert_eq!(euler_to_direction([90.0, 0.0, 0.0]), 3.1415927);
        assert_eq!(euler_to_direction([0.0, 90.0, 0.0]), 2.0354056);
        assert_eq!(euler_to_direction([0.0, 0.0, 90.0]), 0.0);
        assert_eq!(euler_to_direction([-90.0, 0.0, 0.0]), 3.1415927);
        assert_eq!(euler_to_direction([0.0, -90.0, 0.0]), -2.0354056);
        assert_eq!(euler_to_direction([0.0, 0.0, -90.0]), 0.0);
    }

    // Helper macros so we don't repeat ourselves in tests a bunch of times
    #[macro_export]
    macro_rules! client_zone_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/zone/client/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! server_zone_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/zone/server/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! client_chat_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/chat/client/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! server_chat_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/chat/server/", $rel_path)
        };
    }

    // Helper macros so we don't repeat ourselves a bunch of times
    #[macro_export]
    macro_rules! client_lobby_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/lobby/client/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! server_lobby_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/lobby/server/", $rel_path)
        };
    }

    #[macro_export]
    macro_rules! patch_tests_dir {
        ($rel_path:literal) => {
            concat!("resources/data/tests/patch/", $rel_path)
        };
    }
}
