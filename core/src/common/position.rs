use binrw::binrw;
use serde::{Deserialize, Serialize};

/// Represents a point in space.
#[binrw]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Text))]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(feature = "server")]
impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for Position {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(serde_json::to_string(&self).unwrap());
        Ok(diesel::serialize::IsNull::No)
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::sqlite::Sqlite> for Position {
    fn from_sql(
        mut bytes: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        Ok(serde_json::from_str(bytes.read_text()).ok().unwrap())
    }
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
        f32::sqrt(Self::sqr_distance(a, b))
    }

    /// Returns the distance between the two positions `a` and `b`.
    pub fn sqr_distance(a: Position, b: Position) -> f32 {
        let delta = Position {
            x: a.x - b.x,
            y: a.y - b.y,
            z: a.z - b.z,
        };
        Self::dot(delta, delta)
    }

    pub fn dot(a: Position, b: Position) -> f32 {
        a.x * b.x + a.y * b.y + a.z * b.z
    }
}

#[cfg(feature = "server")]
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
        assert_eq!(Position::distance(a, b), 10.0);
    }

    #[test]
    fn test_sqr_distance() {
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
        assert_eq!(Position::sqr_distance(a, a), 0.0);
        assert_eq!(Position::sqr_distance(b, b), 0.0);
        assert_eq!(Position::sqr_distance(a, b), 100.0);
    }

    #[test]
    fn test_dot() {
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
        assert_eq!(Position::dot(a, a), 0.0);
        assert_eq!(Position::dot(b, b), 100.0);
        assert_eq!(Position::dot(a, b), 0.0);
    }
}
