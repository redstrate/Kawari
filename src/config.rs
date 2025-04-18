use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

/// Configuration for the admin server.
#[derive(Serialize, Deserialize)]
pub struct AdminConfig {
    pub port: u16,
    pub listen_address: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            port: 5800,
            listen_address: "127.0.0.1".to_string(),
        }
    }
}

impl AdminConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }
}

/// Configuration for the frontier server.
#[derive(Serialize, Deserialize)]
pub struct FrontierConfig {
    pub port: u16,
    pub listen_address: String,
    pub worlds_open: bool,
    pub login_open: bool,
}

impl Default for FrontierConfig {
    fn default() -> Self {
        Self {
            port: 5857,
            listen_address: "127.0.0.1".to_string(),
            worlds_open: true,
            login_open: true,
        }
    }
}

impl FrontierConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }
}

/// Configuration for the lobby server.
#[derive(Serialize, Deserialize)]
pub struct LobbyConfig {
    pub port: u16,
    pub listen_address: String,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            port: 7000,
            listen_address: "127.0.0.1".to_string(),
        }
    }
}

impl LobbyConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }
}

/// Configuration for the login server.
#[derive(Serialize, Deserialize)]
pub struct LoginConfig {
    pub port: u16,
    pub listen_address: String,
    /// Public-facing domain of the server.
    pub server_name: String,
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            port: 6700,
            listen_address: "127.0.0.1".to_string(),
            server_name: "ffxiv-login.square.localhost".to_string(),
        }
    }
}

impl LoginConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }
}

/// Configuration for the patch server.
#[derive(Serialize, Deserialize)]
pub struct PatchConfig {
    pub port: u16,
    pub listen_address: String,
    /// Publicly accessible URL to download patches from.
    /// For example, "patch-dl.ffxiv.localhost". Patch files must be served so they're accessible as: "http://patch-dl.ffxiv.localhost/game/ex4/somepatchfilename.patch"
    pub patch_dl_url: String,
    /// Location of the patches directory on disk. Must be setup like so:
    /// ```ignore
    /// <channel> (e.g. ffxivneo_release_game) /
    ///     game/
    ///     ex1/
    /// ...
    /// ```
    pub patches_location: String,
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            port: 6900,
            listen_address: "127.0.0.1".to_string(),
            patch_dl_url: "patch-dl.ffxiv.localhost".to_string(),
            patches_location: String::new(),
        }
    }
}

impl PatchConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }
}

/// Configuration for the web server.
#[derive(Serialize, Deserialize)]
pub struct WebConfig {
    pub port: u16,
    pub listen_address: String,
    /// Public-facing domain of the server.
    pub server_name: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: 5801,
            listen_address: "127.0.0.1".to_string(),
            server_name: "ffxiv.localhost".to_string(),
        }
    }
}

impl WebConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }
}

/// Configuration for the world server.
#[derive(Serialize, Deserialize)]
pub struct WorldConfig {
    #[serde(default = "WorldConfig::default_port")]
    pub port: u16,
    #[serde(default = "WorldConfig::default_listen_address")]
    pub listen_address: String,
    /// See the World Excel sheet.
    #[serde(default = "WorldConfig::default_world_id")]
    pub world_id: u16,
    /// Location of the scripts directory.
    /// Defaults to a sensible value if the project is self-built.
    #[serde(default = "WorldConfig::default_scripts_location")]
    pub scripts_location: String,
    /// Port of the RCON server.
    #[serde(default = "WorldConfig::default_rcon_port")]
    pub rcon_port: u16,
    /// Password of the RCON server, if left blank (the default) RCON is disabled.
    #[serde(default = "WorldConfig::default_rcon_password")]
    pub rcon_password: String,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: Self::default_listen_address(),
            world_id: Self::default_world_id(),
            scripts_location: Self::default_scripts_location(),
            rcon_port: Self::default_rcon_port(),
            rcon_password: Self::default_rcon_password(),
        }
    }
}

impl WorldConfig {
    fn default_port() -> u16 {
        7100
    }

    fn default_listen_address() -> String {
        "127.0.0.1".to_string()
    }

    fn default_world_id() -> u16 {
        63 // Gilgamesh
    }

    fn default_scripts_location() -> String {
        "resources/scripts".to_string()
    }

    fn default_rcon_port() -> u16 {
        25575
    }

    fn default_rcon_password() -> String {
        String::default()
    }
}

impl WorldConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }

    /// Returns the configured IP address & port as a `SocketAddr` for RCON.
    pub fn get_rcon_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.rcon_port,
        ))
    }
}

/// Global and all-encompassing config.
/// Settings that affect all servers belong here.
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_supported_platforms")]
    pub supported_platforms: Vec<String>,

    #[serde(default)]
    pub game_location: String,

    #[serde(default)]
    pub admin: AdminConfig,

    #[serde(default)]
    pub frontier: FrontierConfig,

    #[serde(default)]
    pub lobby: LobbyConfig,

    #[serde(default)]
    pub login: LoginConfig,

    #[serde(default)]
    pub patch: PatchConfig,

    #[serde(default)]
    pub web: WebConfig,

    #[serde(default)]
    pub world: WorldConfig,

    /// Enable various packet debug functions. This will clutter your working directory!
    #[serde(default)]
    pub packet_debugging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            supported_platforms: default_supported_platforms(),
            game_location: String::new(),
            admin: AdminConfig::default(),
            frontier: FrontierConfig::default(),
            lobby: LobbyConfig::default(),
            login: LoginConfig::default(),
            patch: PatchConfig::default(),
            web: WebConfig::default(),
            world: WorldConfig::default(),
            packet_debugging: false,
        }
    }
}

impl Config {
    pub fn supports_platform(&self, platform: &String) -> bool {
        self.supported_platforms.contains(platform)
    }
}

fn default_supported_platforms() -> Vec<String> {
    vec!["win32".to_string()]
}

pub fn get_config() -> Config {
    if let Ok(data) = std::fs::read_to_string("config.yaml") {
        serde_yaml_ng::from_str(&data).expect("Failed to parse")
    } else {
        Config::default()
    }
}
