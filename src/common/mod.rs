use std::{
    ffi::CString,
    time::{SystemTime, UNIX_EPOCH},
};

mod customize_data;
pub use customize_data::CustomizeData;
use physis::{
    common::{Language, Platform},
    gamedata::GameData,
};

use crate::config::get_config;

pub mod custom_ipc;

mod position;
pub use position::Position;

/// An invalid actor/object id.
pub const INVALID_OBJECT_ID: u32 = 0xE0000000;

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
