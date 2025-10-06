use bitflags::bitflags;
use std::{
    ffi::CString,
    time::{SystemTime, UNIX_EPOCH},
};

mod customize_data;
use binrw::binrw;
pub use customize_data::CustomizeData;

mod position;
pub use position::Position;

mod gamedata;
pub use gamedata::GameData;
pub use gamedata::{InstanceContentType, ItemInfo, ItemInfoQuery, TerritoryNameKind};

pub mod workdefinitions;

#[binrw]
#[brw(little)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(pub u32);

impl ObjectId {
    /// Returns true if it points to a *valid-looking* object id.
    fn is_valid(&self) -> bool {
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

pub(crate) fn read_bool_from<T: std::convert::From<u8> + std::cmp::PartialEq>(x: T) -> bool {
    x == T::from(1u8)
}

pub(crate) fn write_bool_as<T: std::convert::From<u8>>(x: &bool) -> T {
    if *x { T::from(1u8) } else { T::from(0u8) }
}

pub(crate) fn read_string(byte_stream: Vec<u8>) -> String {
    // TODO: better error handling here
    if let Ok(str) = String::from_utf8(byte_stream) {
        str.trim_matches(char::from(0)).to_string() // trim \0 from the end of strings
    } else {
        String::default()
    }
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
        1 => 128,
        // Gridania
        2 => 132,
        // Ul'dah
        3 => 130,
        _ => panic!("This is not a valid city-state id!"),
    }
}

pub fn value_to_flag_byte_index_value(in_value: u32) -> (u8, u16) {
    let bit_index = in_value % 8;
    (1 << bit_index, (in_value / 8) as u16)
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

// TODO: Possibly relocate this to src/world/common.rs as it's unclear if we'll need this in more places, so it was placed here for now.
#[binrw]
#[brw(repr(u16))]
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ChatChannel {
    #[default]
    Say = 10,
    Shout = 11,
    CustomEmote = 28,
    Yell = 30,
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

    #[test]
    fn read_string() {
        // The nul terminator is supposed to be removed
        assert_eq!(
            crate::common::read_string(STRING_DATA.to_vec()),
            "FOO".to_string()
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
