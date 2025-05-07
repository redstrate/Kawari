use binrw::binrw;
use serde::{Deserialize, Serialize};

#[binrw]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
