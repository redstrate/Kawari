use client_select_data::ClientCustomizeData;
use minijinja::Environment;
use rand::Rng;
use rand::distributions::Alphanumeric;

pub mod blowfish;
pub mod chara_make;
pub mod client_select_data;
mod common;
pub use common::timestamp_secs;
mod compression;
pub mod config;
pub mod encryption;
pub mod ipc;
pub mod lobby;
pub mod oodle;
pub mod packet;
pub mod patchlist;
pub mod world;

// TODO: make this configurable
// See https://ffxiv.consolegameswiki.com/wiki/Servers for a list of possible IDs
pub const WORLD_ID: u16 = 63;
pub const WORLD_NAME: &str = "KAWARI";

pub const ZONE_ID: u16 = 132;

pub const CONTENT_ID: u64 = 11111111111111111;

pub const CUSTOMIZE_DATA: ClientCustomizeData = ClientCustomizeData {
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

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

pub const CHAR_NAME: &str = "Test User";

pub fn generate_sid() -> String {
    let random_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(56)
        .map(char::from)
        .collect();
    random_id.to_lowercase()
}

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
