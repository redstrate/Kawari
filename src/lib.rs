use minijinja::Environment;
use rand::Rng;
use rand::distributions::Alphanumeric;

pub mod blowfish;
pub mod client_select_data;
mod common;
mod compression;
pub mod config;
pub mod encryption;
pub mod ipc;
pub mod oodle;
pub mod packet;
pub mod patchlist;
pub mod world;

// TODO: make this configurable
// See https://ffxiv.consolegameswiki.com/wiki/Servers for a list of possible IDs
pub const WORLD_ID: u16 = 63;
pub const WORLD_NAME: &str = "KAWARI";

pub const ZONE_ID: u16 = 1255;

pub const CONTENT_ID: u64 = 11111111111111111;

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

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
