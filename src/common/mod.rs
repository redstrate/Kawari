use std::{
    ffi::CString,
    time::{SystemTime, UNIX_EPOCH},
};

mod customize_data;
use binrw::binrw;
pub use customize_data::CustomizeData;
use physis::{
    common::{Language, Platform},
    gamedata::GameData,
};

use crate::config::get_config;

pub mod custom_ipc;

mod position;
pub use position::Position;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    let max = std::u16::MAX as f32;
    let pi = std::f32::consts::PI;

    quantized as f32 / max * (2.0 * pi) - pi
}

/// Converts a rotation (in degrees) to
pub(crate) fn write_quantized_rotation(quantized: &f32) -> u16 {
    let max = std::u16::MAX as f32;
    let pi = std::f32::consts::PI;

    ((quantized + pi / (2.0 * pi)) * max) as u16
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

/// Gets the world name from an id into the World Excel sheet.
pub fn get_world_name(world_id: u16) -> String {
    let config = get_config();

    let mut game_data = GameData::from_existing(Platform::Win32, &config.game_location).unwrap();

    let exh = game_data.read_excel_sheet_header("World").unwrap();
    let exd = game_data
        .read_excel_sheet("World", &exh, Language::None, 0)
        .unwrap();

    let world_row = &exd.read_row(&exh, world_id as u32).unwrap()[0];

    let physis::exd::ColumnData::String(name) = &world_row.data[1] else {
        panic!("Unexpected type!");
    };

    name.clone()
}

/// Gets the starting city-state from a given class/job id.
pub fn get_citystate(classjob_id: u16) -> u8 {
    let config = get_config();

    let mut game_data = GameData::from_existing(Platform::Win32, &config.game_location).unwrap();

    let exh = game_data.read_excel_sheet_header("ClassJob").unwrap();
    let exd = game_data
        .read_excel_sheet("ClassJob", &exh, Language::English, 0)
        .unwrap();

    let world_row = &exd.read_row(&exh, classjob_id as u32).unwrap()[0];

    let physis::exd::ColumnData::UInt8(town_id) = &world_row.data[33] else {
        panic!("Unexpected type!");
    };

    *town_id
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

/// Gets the primary model ID for a given item ID
pub fn get_primary_model_id(item_id: u32) -> u16 {
    let config = get_config();

    let mut game_data = GameData::from_existing(Platform::Win32, &config.game_location).unwrap();

    let exh = game_data.read_excel_sheet_header("Item").unwrap();
    for (i, _) in exh.pages.iter().enumerate() {
        let exd = game_data
            .read_excel_sheet("Item", &exh, Language::English, i)
            .unwrap();

        if let Some(row) = exd.read_row(&exh, item_id) {
            let item_row = &row[0];

            let physis::exd::ColumnData::UInt64(id) = &item_row.data[47] else {
                panic!("Unexpected type!");
            };

            return *id as u16;
        }
    }

    // TODO: just turn this into an Option<>
    tracing::warn!("Failed to get model id for {item_id}, this is most likely a bug!");

    0
}

pub struct Attributes {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
}

pub fn get_racial_base_attributes(tribe_id: u8) -> Attributes {
    // The Tribe Excel sheet only has deltas (e.g. 2 or -2) which are applied to a base 20 number... from somewhere
    let base_stat = 20;

    let config = get_config();

    let mut game_data = GameData::from_existing(Platform::Win32, &config.game_location).unwrap();

    let exh = game_data.read_excel_sheet_header("Tribe").unwrap();
    let exd = game_data
        .read_excel_sheet("Tribe", &exh, Language::English, 0)
        .unwrap();

    let tribe_row = &exd.read_row(&exh, tribe_id as u32).unwrap()[0];

    let get_column = |column_index: usize| {
        let physis::exd::ColumnData::Int8(delta) = &tribe_row.data[column_index] else {
            panic!("Unexpected type!");
        };

        *delta
    };

    Attributes {
        strength: (base_stat + get_column(4)) as u32,
        dexterity: (base_stat + get_column(6)) as u32,
        vitality: (base_stat + get_column(5)) as u32,
        intelligence: (base_stat + get_column(7)) as u32,
        mind: (base_stat + get_column(8)) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
