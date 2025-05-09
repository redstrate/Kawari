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

pub mod workdefinitions;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(pub u32);

impl Default for ObjectId {
    fn default() -> Self {
        INVALID_OBJECT_ID
    }
}

// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Object/GameObject.cs#L158
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTypeId {
    pub object_id: ObjectId,
    #[brw(pad_after = 3)]
    pub object_type: u8,
}

impl Default for ObjectTypeId {
    fn default() -> Self {
        Self {
            object_id: INVALID_OBJECT_ID,
            object_type: 0, // TODO: not sure if correct?
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
    let str = String::from_utf8(byte_stream).unwrap();
    str.trim_matches(char::from(0)).to_string() // trim \0 from the end of strings
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

pub struct Attributes {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
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
}
