//! All things object ID-related.

use binrw::binrw;
use serde::Deserialize;

#[binrw]
#[brw(little)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::BigInt))]
pub struct ObjectId(pub u32);

impl ObjectId {
    /// Returns true if it points to a *valid-looking* object id.
    pub fn is_valid(&self) -> bool {
        *self != INVALID_OBJECT_ID
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        INVALID_OBJECT_ID
    }
}

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            write!(f, "{}", self.0)
        } else {
            write!(f, "INVALID_ACTOR")
        }
    }
}

impl std::fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ObjectId ({self})")
    }
}

#[cfg(feature = "server")]
impl diesel::serialize::ToSql<diesel::sql_types::BigInt, diesel::sqlite::Sqlite> for ObjectId {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.0 as i64);
        Ok(diesel::serialize::IsNull::No)
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::BigInt, diesel::sqlite::Sqlite> for ObjectId {
    fn from_sql(
        mut integer: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        Ok(ObjectId(integer.read_long() as u32))
    }
}

// This is unrelated to the ObjectKind struct as named by ClientStructs; it's used for ACT::SetTarget, ACT::Emote, and probably more.
// Instead it correlates to the Type field in the GameObjectId client struct.
// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Object/GameObject.cs#L230
#[binrw]
#[brw(repr = u32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ObjectTypeKind {
    /// Everything that has a proper entity/actor ID.
    #[default]
    None = 0,
    /// Orchestrions, static NPCs in towns, etc., and possibly more.
    EObjOrNpc = 1,
    /// Unclear when this is used, more research is needed.
    /// ClientStructs describes it as "if (BaseId == 0 || (ObjectIndex >= 200 && ObjectIndex < 244)) ObjectId = ObjectIndex, Type = 2"
    Unknown = 2,
    /// Player-summoned minions (not to be confused with chocobos or other bnpc pets), and possibly more.
    Minion = 4,
}

// See https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Object/GameObject.cs#L238
#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTypeId {
    pub object_id: ObjectId,
    pub object_type: ObjectTypeKind,
}

impl Default for ObjectTypeId {
    fn default() -> Self {
        Self {
            object_id: INVALID_OBJECT_ID,
            object_type: ObjectTypeKind::None,
        }
    }
}

#[cfg(feature = "server")]
impl mlua::UserData for ObjectTypeId {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("object_id", |_, this| Ok(this.object_id.0));
    }
}

#[cfg(feature = "server")]
impl mlua::FromLua for ObjectTypeId {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            // Currently always assume its referring to an ENPC or something:
            mlua::Value::Integer(integer) => Ok(Self {
                object_id: ObjectId(integer as u32),
                object_type: ObjectTypeKind::EObjOrNpc,
            }),
            _ => unreachable!(),
        }
    }
}

/// An invalid actor/object id.
const INVALID_OBJECT_ID: ObjectId = ObjectId(0xE0000000);
