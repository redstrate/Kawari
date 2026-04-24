use binrw::binrw;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Represents a point in space.
///
/// This is a newtype around glam's Vec3 in cases where we need to:
/// 1. Exposing this into Lua.
/// 2. Serializing in SQL or JSON.
/// 3. Read/write from bytes using binrw.
#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Text))]
#[serde(from = "PositionHelper", into = "PositionHelper")]
pub struct Position(
    #[br(map = read_position)]
    #[bw(map = write_position)]
    pub Vec3,
);

// NOTE: To keep compatibility with Lua, drop-ins etc but should we keep it long-term?
#[derive(Serialize, Deserialize)]
struct PositionHelper {
    x: f32,
    y: f32,
    z: f32,
}

impl From<PositionHelper> for Position {
    fn from(value: PositionHelper) -> Self {
        Self(Vec3 {
            x: value.x,
            y: value.y,
            z: value.z,
        })
    }
}

impl From<Position> for PositionHelper {
    fn from(value: Position) -> Self {
        Self {
            x: value.0.x,
            y: value.0.y,
            z: value.0.z,
        }
    }
}

fn read_position(packed: [f32; 3]) -> Vec3 {
    Vec3::from_array(packed)
}

fn write_position(pos: &Vec3) -> [f32; 3] {
    [pos.x, pos.y, pos.z]
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

#[cfg(feature = "server")]
impl mlua::UserData for Position {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.0.x));
        fields.add_field_method_get("y", |_, this| Ok(this.0.y));
        fields.add_field_method_get("z", |_, this| Ok(this.0.z));
    }
}
