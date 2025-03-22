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
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            port: 6700,
            listen_address: "127.0.0.1".to_string(),
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
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            port: 6900,
            listen_address: "127.0.0.1".to_string(),
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
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: 5801,
            listen_address: "127.0.0.1".to_string(),
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
    pub port: u16,
    pub listen_address: String,
    /// See the World Excel sheet.
    pub world_id: u16,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            port: 7100,
            listen_address: "127.0.0.1".to_string(),
            world_id: 1, // Dev
        }
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
}

/// Global and all-encompassing config.
/// Settings that affect all servers belong here.
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_supported_platforms")]
    pub supported_platforms: Vec<String>,

    #[serde(default)]
    pub boot_patches_location: String,

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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            boot_patches_location: String::new(),
            supported_platforms: default_supported_platforms(),
            game_location: String::new(),
            admin: AdminConfig::default(),
            frontier: FrontierConfig::default(),
            lobby: LobbyConfig::default(),
            login: LoginConfig::default(),
            patch: PatchConfig::default(),
            web: WebConfig::default(),
            world: WorldConfig::default(),
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
