//! A server replacement for a certain MMO.

use common::CustomizeData;
use minijinja::Environment;

/// The blowfish implementation used for packet encryption.
pub mod blowfish;

/// Common functions, structures used between all servers.
pub mod common;

/// Config management.
pub mod config;

/// Bindings for Oodle network compression.
pub mod oodle;

/// Lobby server-specific code.
pub mod lobby;

/// World server-specific code.
pub mod world;

/// Everything packet parsing related.
pub mod packet;

// TODO: make this configurable
/// The world ID and name for the lobby.
/// See <https://ffxiv.consolegameswiki.com/wiki/Servers> for a list of possible IDs.
pub const WORLD_ID: u16 = 63;
pub const WORLD_NAME: &str = "KAWARI";

/// The zone ID you initially spawn in.
/// See the TerritoryType excel sheet for a list of possible IDs.
pub const ZONE_ID: u16 = 132;

pub const CONTENT_ID: u64 = 11111111111111111;

pub const CUSTOMIZE_DATA: CustomizeData = CustomizeData {
    race: 4,
    gender: 1,
    age: 1,
    height: 50,
    subrace: 7,
    face: 3,
    hair: 5,
    enable_highlights: 0,
    skin_tone: 10,
    right_eye_color: 75,
    hair_tone: 50,
    highlights: 0,
    facial_features: 1,
    facial_feature_color: 19,
    eyebrows: 1,
    left_eye_color: 75,
    eyes: 1,
    nose: 0,
    jaw: 1,
    mouth: 1,
    lips_tone_fur_pattern: 169,
    race_feature_size: 100,
    race_feature_type: 1,
    bust: 100,
    face_paint: 0,
    face_paint_color: 167,
};

pub const DEITY: u8 = 0x8;
pub const NAMEDAY_MONTH: u8 = 0x1;
pub const NAMEDAY_DAY: u8 = 0x1;
pub const CITY_STATE: u8 = 0x3;

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

pub const CHAR_NAME: &str = "Test User";

pub fn setup_default_environment() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template("admin.html", include_str!("../templates/admin.html"))
        .unwrap();
    env.add_template("web.html", include_str!("../templates/web.html"))
        .unwrap();
    env.add_template("login.html", include_str!("../templates/login.html"))
        .unwrap();
    env.add_template("register.html", include_str!("../templates/register.html"))
        .unwrap();
    env.add_template(
        "worldstatus.html",
        include_str!("../templates/worldstatus.html"),
    )
    .unwrap();

    env
}
