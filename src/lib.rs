//! A server replacement for a certain MMO.

#![allow(clippy::large_enum_variant)]

use std::collections::HashMap;

use minijinja::Environment;
use patch::Version;

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

/// Patch server-specific code.
pub mod patch;

/// Used in the encryption key.
const GAME_VERSION: u16 = 7000;

/// Supported boot version.
pub const SUPPORTED_BOOT_VERSION: Version = Version("2025.01.10.0000.0001");

/// Supported game version.
pub const SUPPORTED_GAME_VERSION: Version = Version("2025.02.27.0000.0000");

const SUPPORTED_EXPAC_VERSIONS: [(&str, Version); 5] = [
    ("ex1", Version("2025.01.09.0000.0000")),
    ("ex2", Version("2025.01.14.0000.0000")),
    ("ex3", Version("2025.02.27.0000.0000")),
    ("ex4", Version("2025.02.27.0000.0000")),
    ("ex5", Version("2025.02.27.0000.0000")),
];

/// Supported expansion versions.
pub fn get_supported_expac_versions() -> HashMap<&'static str, Version<'static>> {
    HashMap::from(SUPPORTED_EXPAC_VERSIONS)
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
