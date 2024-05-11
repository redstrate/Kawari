use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub gate_open: bool
}

impl Default for Config {
    fn default() -> Self {
        Self {
            gate_open: false,
        }
    }
}