//! A server replacement for a certain MMO.

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

/// Logic server-specific code.
pub mod login;

/// The zone ID you initially spawn in.
/// See the TerritoryType excel sheet for a list of possible IDs.
pub const ZONE_ID: u16 = 132;

pub const INVALID_OBJECT_ID: u32 = 0xE0000000;

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

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
