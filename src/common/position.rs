use binrw::binrw;
use serde::{Deserialize, Serialize};

#[binrw]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn lerp(a: Position, b: Position, t: f32) -> Position {
        let lerp = |v0: f32, v1: f32, t: f32| v0 + t * (v1 - v0);

        Position {
            x: lerp(a.x, b.x, t),
            y: lerp(a.y, b.y, t),
            z: lerp(a.z, b.z, t),
        }
    }

    pub fn distance(a: Position, b: Position) -> f32 {
        let delta_x = b.x - a.x;
        let delta_y = b.y - a.y;
        let delta_z = b.z - a.z;
        delta_x.powi(2) + delta_y.powi(2) + delta_z.powi(2)
    }
}
