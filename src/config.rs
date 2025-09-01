use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use serde::{Deserialize, Serialize};

fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

/// Configuration for the admin server.
#[derive(Serialize, Deserialize)]
pub struct AdminConfig {
    #[serde(default = "AdminConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
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

    fn default_port() -> u16 {
        5800
    }
}

/// Configuration for the frontier server.
#[derive(Serialize, Deserialize)]
pub struct FrontierConfig {
    #[serde(default = "FrontierConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    #[serde(default = "FrontierConfig::default_server_name")]
    pub server_name: String,
    #[serde(default = "FrontierConfig::default_worlds_open")]
    pub worlds_open: bool,
    #[serde(default = "FrontierConfig::default_login_open")]
    pub login_open: bool,
}

impl Default for FrontierConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
            worlds_open: Self::default_worlds_open(),
            login_open: Self::default_login_open(),
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

    fn default_port() -> u16 {
        5857
    }

    fn default_worlds_open() -> bool {
        true
    }

    fn default_login_open() -> bool {
        true
    }

    fn default_server_name() -> String {
        "http://frontier.ffxiv.localhost".to_string()
    }
}

/// Configuration for the lobby server.
#[derive(Serialize, Deserialize)]
pub struct LobbyConfig {
    #[serde(default = "LobbyConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    #[serde(default = "LobbyConfig::default_server_name")]
    pub server_name: String,
}

impl Default for LobbyConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
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

    fn default_port() -> u16 {
        7000
    }

    fn default_server_name() -> String {
        "127.0.0.1".to_string()
    }
}

/// Configuration for the login server.
#[derive(Serialize, Deserialize)]
pub struct LoginConfig {
    #[serde(default = "LoginConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    /// Public-facing domain of the server.
    #[serde(default = "LoginConfig::default_server_name")]
    pub server_name: String,
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
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

    fn default_port() -> u16 {
        6700
    }

    fn default_server_name() -> String {
        "http://ffxiv-login.square.localhost".to_string()
    }
}

/// Configuration for the patch server.
#[derive(Serialize, Deserialize)]
pub struct PatchConfig {
    #[serde(default = "PatchConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    /// Publicly accessible URL to download patches from.
    /// For example, "patch-dl.ffxiv.localhost". Patch files must be served so they're accessible as: "http://patch-dl.ffxiv.localhost/game/ex4/somepatchfilename.patch"
    #[serde(default = "PatchConfig::default_patch_dl_url")]
    pub patch_dl_url: String,
    /// Location of the patches directory on disk. Must be setup like so:
    /// ```ignore
    /// <channel> (e.g. ffxivneo_release_game) /
    ///     game/
    ///     ex1/
    /// ...
    /// ```
    #[serde(default = "PatchConfig::default_patches_location")]
    pub patches_location: String,
    #[serde(default = "PatchConfig::default_game_server_name")]
    pub game_server_name: String,
    #[serde(default = "PatchConfig::default_boot_server_name")]
    pub boot_server_name: String,
    #[serde(default = "PatchConfig::default_supported_platforms")]
    pub supported_platforms: Vec<String>,
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            patch_dl_url: Self::default_patch_dl_url(),
            patches_location: Self::default_patches_location(),
            boot_server_name: Self::default_boot_server_name(),
            game_server_name: Self::default_game_server_name(),
            supported_platforms: Self::default_supported_platforms(),
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

    fn default_port() -> u16 {
        6900
    }

    fn default_patch_dl_url() -> String {
        "http://patch-dl.ffxiv.localhost".to_string()
    }

    fn default_patches_location() -> String {
        "patches".to_string()
    }

    fn default_boot_server_name() -> String {
        "http://patch-bootver.ffxiv.localhost".to_string()
    }

    fn default_game_server_name() -> String {
        "http://patch-gamever.ffxiv.localhost".to_string()
    }

    fn default_supported_platforms() -> Vec<String> {
        vec!["win32".to_string()]
    }

    pub fn supports_platform(&self, platform: &String) -> bool {
        self.supported_platforms.contains(platform)
    }
}

/// Configuration for the web server.
#[derive(Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default = "WebConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    /// Public-facing domain of the server.
    #[serde(default = "WebConfig::default_server_name")]
    pub server_name: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
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

    fn default_port() -> u16 {
        5801
    }

    fn default_server_name() -> String {
        "http://ffxiv.localhost".to_string()
    }
}

/// Configuration for the world server.
#[derive(Serialize, Deserialize)]
pub struct WorldConfig {
    #[serde(default = "WorldConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    #[serde(default = "WorldConfig::default_server_name")]
    pub server_name: String,
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
    /// Enable packet obsfucation. There's literally no reason to do this!
    #[serde(default = "WorldConfig::default_packet_obsfucation")]
    pub enable_packet_obsfucation: bool,
    /// Enable packet compression for packets from the server. It's recommended to keep this on.
    #[serde(default = "WorldConfig::default_packet_compression")]
    pub enable_packet_compression: bool,
    /// Default message received when logging into the world.
    #[serde(default = "WorldConfig::default_login_message")]
    pub login_message: String,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
            world_id: Self::default_world_id(),
            scripts_location: Self::default_scripts_location(),
            rcon_port: Self::default_rcon_port(),
            rcon_password: Self::default_rcon_password(),
            enable_packet_obsfucation: Self::default_packet_obsfucation(),
            enable_packet_compression: Self::default_packet_compression(),
            login_message: Self::default_login_message(),
        }
    }
}

impl WorldConfig {
    fn default_port() -> u16 {
        7100
    }

    fn default_server_name() -> String {
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

    fn default_packet_obsfucation() -> bool {
        false
    }

    fn default_packet_compression() -> bool {
        true
    }

    fn default_login_message() -> String {
        "Welcome to Kawari!".to_string()
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

    pub fn get_public_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.server_name).expect("Invalid IP address format in config!"),
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

/// Configuration for the launcher server.
#[derive(Serialize, Deserialize)]
pub struct LauncherConfig {
    #[serde(default = "LauncherConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    #[serde(default = "LauncherConfig::default_server_name")]
    pub server_name: String,
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
        }
    }
}

impl LauncherConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }

    fn default_port() -> u16 {
        5802
    }

    fn default_server_name() -> String {
        "http://launcher.ffxiv.localhost".to_string()
    }
}

/// Configuration for the save data bank server.
#[derive(Serialize, Deserialize)]
pub struct SaveDataBankConfig {
    #[serde(default = "SaveDataBankConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
}

impl Default for SaveDataBankConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
        }
    }
}

impl SaveDataBankConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }

    fn default_port() -> u16 {
        5803
    }
}

/// Configuration for the data center travel server.
#[derive(Serialize, Deserialize)]
pub struct DataCenterTravelConfig {
    #[serde(default = "DataCenterTravelConfig::default_port")]
    pub port: u16,
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    #[serde(default = "DataCenterTravelConfig::default_server_name")]
    pub server_name: String,
}

impl Default for DataCenterTravelConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
        }
    }
}

impl DataCenterTravelConfig {
    /// Returns the configured IP address & port as a `SocketAddr`.
    pub fn get_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.listen_address).expect("Invalid IP address format in config!"),
            self.port,
        ))
    }

    fn default_port() -> u16 {
        5860
    }

    fn default_server_name() -> String {
        "http://dctravel.ffxiv.localhost".to_string()
    }
}

/// Configuration for the game filesystem.
#[derive(Serialize, Deserialize, Default)]
pub struct FilesystemConfig {
    /// Path to the game directory. For example, "C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\game".
    #[serde(default)]
    pub game_path: String,

    /// Additional search paths for *unpacked game files*.
    /// These are ordered from highest-to-lowest, these are always preferred over retail game files.
    #[serde(default)]
    pub additional_search_paths: Vec<String>,

    /// Unpack used files to the specified directory.
    /// If the directory is not specified, Kawari won't save file contents.
    #[serde(default)]
    pub unpack_path: String,

    /// Navimesh file directory.
    #[serde(default)]
    pub navimesh_path: String,
}

/// Global and all-encompassing config.
/// Settings that affect all servers belong here.
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub filesystem: FilesystemConfig,

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

    #[serde(default)]
    pub launcher: LauncherConfig,

    #[serde(default)]
    pub save_data_bank: SaveDataBankConfig,

    #[serde(default)]
    pub datacenter_travel: DataCenterTravelConfig,

    /// Enable various validity checks for version and file hashes that emulate retail.
    #[serde(default = "Config::default_enforce_validity_checks")]
    pub enforce_validity_checks: bool,

    /// Enables running in front of Sapphire as a proxy. This only has an effect on some servers (e.g. login.)
    /// This *DOES* affect whether how the server normally runs, so you can't use a Sapphire-proxy as a regular game server.
    #[serde(default = "Config::default_enable_sapphire_proxy")]
    pub enable_sapphire_proxy: bool,

    /// The URL to the Sapphire API (e.g. 127.0.0.1:80)
    #[serde(default)]
    pub sapphire_api_server: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            filesystem: FilesystemConfig::default(),
            admin: AdminConfig::default(),
            frontier: FrontierConfig::default(),
            lobby: LobbyConfig::default(),
            login: LoginConfig::default(),
            patch: PatchConfig::default(),
            web: WebConfig::default(),
            world: WorldConfig::default(),
            launcher: LauncherConfig::default(),
            save_data_bank: SaveDataBankConfig::default(),
            datacenter_travel: DataCenterTravelConfig::default(),
            enforce_validity_checks: Self::default_enforce_validity_checks(),
            enable_sapphire_proxy: Self::default_enable_sapphire_proxy(),
            sapphire_api_server: String::default(),
        }
    }
}

impl Config {
    fn default_enforce_validity_checks() -> bool {
        true
    }

    fn default_enable_sapphire_proxy() -> bool {
        false
    }
}

pub fn get_config() -> Config {
    if let Ok(data) = std::fs::read_to_string("config.yaml") {
        serde_yaml_ng::from_str(&data).expect("Failed to parse")
    } else {
        Config::default()
    }
}
