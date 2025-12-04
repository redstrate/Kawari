use binrw::binrw;
use serde::{Deserialize, Serialize};

/// Represents a point in space.
#[binrw]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    /// Returns the interpolated position between `a` and `b` at time `t` (0.0 to 1.0).
    pub fn lerp(a: Position, b: Position, t: f32) -> Position {
        let lerp = |v0: f32, v1: f32, t: f32| v0 + t * (v1 - v0);

        Position {
            x: lerp(a.x, b.x, t),
            y: lerp(a.y, b.y, t),
            z: lerp(a.z, b.z, t),
        }
    }

    /// Returns the distance between the two positions `a` and `b`.
    pub fn distance(a: Position, b: Position) -> f32 {
        let delta_x = b.x - a.x;
        let delta_y = b.y - a.y;
        let delta_z = b.z - a.z;
        delta_x.powi(2) + delta_y.powi(2) + delta_z.powi(2)
    }
}

#[cfg(all(not(target_family = "wasm"), feature = "server"))]
impl mlua::UserData for Position {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.x));
        fields.add_field_method_get("y", |_, this| Ok(this.y));
        fields.add_field_method_get("z", |_, this| Ok(this.z));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        let a = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let b = Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(Position::lerp(a, b, 0.0), a);
        assert_eq!(Position::lerp(a, b, 1.0), b);
        assert_eq!(
            Position::lerp(a, b, 0.5),
            Position {
                x: 5.0,
                y: 0.0,
                z: 0.0
            }
        );
    }

    #[test]
    fn test_distance() {
        let a = Position {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
        let b = Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        };
        assert_eq!(Position::distance(a, a), 0.0);
        assert_eq!(Position::distance(b, b), 0.0);
        assert_eq!(Position::distance(a, b), 100.0);
    }
}
