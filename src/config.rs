use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub worlds_open: bool,

    #[serde(default)]
    pub login_open: bool,

    #[serde(default = "default_supported_platforms")]
    pub supported_platforms: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            worlds_open: false,
            login_open: false,
            supported_platforms: default_supported_platforms()
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
    if let Ok(data) = std::fs::read_to_string("config.json") {
        serde_json::from_str(&data).expect("Failed to parse")
    } else {
        Config::default()
    }
}