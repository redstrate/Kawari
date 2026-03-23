use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};

use physis::Language;
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
        21057
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
        21058
    }

    fn default_worlds_open() -> bool {
        true
    }

    fn default_login_open() -> bool {
        true
    }

    fn default_server_name() -> String {
        format!("http://frontier.ffxiv.localhost:{}", Self::default_port())
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
        21059
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

    /// Whether registrations are enabled, and by default they are.
    /// Turning this off also hides the UI for this feature.
    #[serde(default = "LoginConfig::default_enable_registration")]
    pub enable_registration: bool,
}

impl Default for LoginConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
            enable_registration: Self::default_enable_registration(),
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
        21060
    }

    fn default_server_name() -> String {
        format!(
            "http://ffxiv-login.square.localhost:{}",
            Self::default_port()
        )
    }

    fn default_enable_registration() -> bool {
        true
    }
}

/// Configuration for the patch server.
#[derive(Serialize, Deserialize)]
pub struct PatchConfig {
    #[serde(default = "PatchConfig::default_port")]
    pub port: u16,

    #[serde(default = "default_listen_address")]
    pub listen_address: String,

    /// Location of the patches directory on disk. Must be setup like so:
    /// ```ignore
    /// <channel> (e.g. ffxivneo_release_game) /
    ///     game/
    ///     ex1/
    /// ...
    /// ```
    #[serde(default = "PatchConfig::default_patches_location")]
    pub patches_location: String,

    #[serde(default = "PatchConfig::default_server_name")]
    pub server_name: String,

    #[serde(default = "PatchConfig::default_supported_platforms")]
    pub supported_platforms: Vec<String>,
}

impl Default for PatchConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            listen_address: default_listen_address(),
            patches_location: Self::default_patches_location(),
            server_name: Self::default_server_name(),
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
        21061
    }

    fn default_patches_location() -> String {
        "patches".to_string()
    }

    fn default_server_name() -> String {
        format!("http://patch.ffxiv.localhost:{}", Self::default_port())
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
        21062
    }

    fn default_server_name() -> String {
        format!("http://ffxiv.localhost:{}", Self::default_port())
    }
}

/// Configuration for the world server.
#[derive(Serialize, Deserialize)]
pub struct WorldConfig {
    #[serde(default = "WorldConfig::default_port")]
    pub port: u16,

    #[serde(default = "WorldConfig::default_healthcheck_port")]
    pub healthcheck_port: u16,

    #[serde(default = "default_listen_address")]
    pub listen_address: String,

    #[serde(default = "WorldConfig::default_server_name")]
    pub server_name: String,

    /// See the World Excel sheet.
    #[serde(default = "WorldConfig::default_world_id")]
    pub world_id: u16,

    /// Enable packet obsfucation. There's literally no reason to do this!
    #[serde(default = "WorldConfig::default_packet_obsfucation")]
    pub enable_packet_obsfucation: bool,

    /// Enable packet compression for packets from the server. It's recommended to keep this on.
    #[serde(default = "WorldConfig::default_packet_compression")]
    pub enable_packet_compression: bool,

    /// Default message received when logging into the world.
    #[serde(default = "WorldConfig::default_login_message")]
    pub login_message: String,

    /// Whether we generate new navmeshes on-demand. This consumes a lot of resources when entering a new zone!
    #[serde(default = "WorldConfig::default_generate_navmesh")]
    pub generate_navmesh: bool,

    /// The active festivals.
    #[serde(default = "WorldConfig::default_active_festivals")]
    pub active_festivals: [u16; 4],

    /// Whether the World should accept new characters.
    #[serde(default = "WorldConfig::default_accept_new_characters")]
    pub accept_new_characters: bool,

    /// Whether the World has the EXP bonus bonus.
    #[serde(default = "WorldConfig::default_exp_bonus")]
    pub exp_bonus: bool,

    /// The language to read game data as, should have no effect on regular gameplay but definitely does affect a lot of debug/GM commands.
    #[serde(default = "WorldConfig::default_language")]
    pub language: String,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            port: Self::default_port(),
            healthcheck_port: Self::default_healthcheck_port(),
            listen_address: default_listen_address(),
            server_name: Self::default_server_name(),
            world_id: Self::default_world_id(),
            enable_packet_obsfucation: Self::default_packet_obsfucation(),
            enable_packet_compression: Self::default_packet_compression(),
            login_message: Self::default_login_message(),
            generate_navmesh: Self::default_generate_navmesh(),
            active_festivals: Self::default_active_festivals(),
            accept_new_characters: Self::default_accept_new_characters(),
            exp_bonus: Self::default_exp_bonus(),
            language: Self::default_language(),
        }
    }
}

impl WorldConfig {
    fn default_port() -> u16 {
        21063
    }

    fn default_healthcheck_port() -> u16 {
        21064
    }

    fn default_server_name() -> String {
        "127.0.0.1".to_string()
    }

    fn default_world_id() -> u16 {
        63 // Gilgamesh
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

    fn default_generate_navmesh() -> bool {
        true
    }

    fn default_active_festivals() -> [u16; 4] {
        [0; 4]
    }

    fn default_accept_new_characters() -> bool {
        true
    }

    fn default_exp_bonus() -> bool {
        false
    }

    fn default_language() -> String {
        "en".to_string()
    }

    pub fn language(&self) -> Language {
        // TODO: possibly de-duplicate this in Physis?
        match self.language.as_str() {
            "ja" => Language::Japanese,
            "en" => Language::English,
            "de" => Language::German,
            "fr" => Language::French,
            _ => panic!("Unsupported language code!"),
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

    pub fn get_public_socketaddr(&self) -> SocketAddr {
        SocketAddr::from((
            IpAddr::from_str(&self.server_name).expect("Invalid IP address format in config!"),
            self.port,
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
        21065
    }

    fn default_server_name() -> String {
        format!("http://launcher.ffxiv.localhost:{}", Self::default_port())
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
        21066
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
        21067
    }

    fn default_server_name() -> String {
        format!("http://dctravel.ffxiv.localhost:{}", Self::default_port())
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
    #[serde(default = "FilesystemConfig::default_navimesh_path")]
    pub navimesh_path: String,

    /// Additional search paths for *resource files*. Needs to have the same folder structure as ours.
    ///
    /// These are ordered from highest-to-lowest, and these are always preferred over our own resource files.
    ///
    /// Note that drop-ins and timelines are *not* combined. Web templates do not respect this option.
    #[serde(default)]
    pub additional_resource_paths: Vec<String>,
}

impl FilesystemConfig {
    fn default_navimesh_path() -> String {
        "navimesh".to_string()
    }

    /// Locates a script file and returns its path, taking into account additional search paths.
    ///
    /// This is infallible as it will always return our built-in path.
    pub fn locate_script_file(path: &str) -> String {
        let config = get_config();
        for search_path in config.filesystem.additional_resource_paths {
            let file_name = format!("{search_path}/scripts/{path}");
            if std::fs::exists(&file_name).unwrap_or_default() {
                return file_name;
            }
        }

        format!("resources/scripts/{path}")
    }

    /// Locates a timeline file and returns its path, taking into account additional search paths.
    ///
    /// This is infallible as it will always return our built-in path.
    pub fn locate_timeline_file(path: &str) -> String {
        let config = get_config();
        for search_path in config.filesystem.additional_resource_paths {
            let file_name = format!("{search_path}/timelines/{path}");
            if std::fs::exists(&file_name).unwrap_or_default() {
                return file_name;
            }
        }

        format!("resources/timelines/{path}")
    }
}

/// Configuration for various tweaks.
#[derive(Serialize, Deserialize)]
pub struct TweaksConfig {
    /// If true, always the player to skip cutscenes marked as unskippable.
    #[serde(default)]
    pub always_allow_skipping: bool,

    /// Enable various validity checks for version and file hashes that emulate retail.
    #[serde(default = "TweaksConfig::default_enforce_validity_checks")]
    pub enforce_validity_checks: bool,
}

impl Default for TweaksConfig {
    fn default() -> Self {
        Self {
            always_allow_skipping: false,
            enforce_validity_checks: Self::default_enforce_validity_checks(),
        }
    }
}

impl TweaksConfig {
    fn default_enforce_validity_checks() -> bool {
        true
    }
}

/// Global and all-encompassing config.
/// Settings that affect all servers belong here.
#[derive(Serialize, Deserialize, Default)]
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

    #[serde(default)]
    pub tweaks: TweaksConfig,
}

pub fn get_config() -> Config {
    if let Ok(data) = std::fs::read_to_string("config.yaml") {
        serde_yaml_ng::from_str(&data).expect("Failed to parse")
    } else {
        Config::default()
    }
}
