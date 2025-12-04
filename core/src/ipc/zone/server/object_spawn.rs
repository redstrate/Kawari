use binrw::binrw;

use crate::common::{ObjectId, Position, read_quantized_rotation, write_quantized_rotation};
use serde::Deserialize;

// TODO: this is all kinds of wrong, take the fields with a grain of salt
#[binrw]
#[brw(little)]
#[derive(Debug, Copy, Clone, Default, Deserialize)]
pub struct ObjectSpawn {
    pub index: u8,
    /// See `ObjectKind`.
    pub kind: u8,
    /// Seems to control whether or not its targetable?
    #[brw(pad_after = 1)]
    pub flag: u8,
    /// If this is an ENPC, represents an index into the EObj Excel sheet.
    /// If this is an AreaObject, represents an index into the VFX Excel sheet.
    pub base_id: u32,
    pub entity_id: u32,
    /// Instance ID of the EventObject in the LGB.
    pub layout_id: u32,
    pub event_id: u32,
    pub owner_id: ObjectId,
    /// Bound ID of the EventObject in the LGB, usually a SharedGroup.
    /// If set to 0, then `radius` is used.
    pub bind_layout_id: u32,
    /// Radius of the hitbox(?)
    pub radius: f32,
    pub shared_group_timeline_state: u16,
    /// The rotation to create the object, in radians.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// The FATE to associate with.
    pub fate_id: u16,
    pub event_state: u8,
    pub args1: u8,
    pub args2: u32,
    pub args3: u32,
    pub unk1: u32,
    /// The position to create the object at.
    pub position: Position,
}

#[cfg(feature = "server")]
impl mlua::UserData for ObjectSpawn {}

#[cfg(feature = "server")]
impl mlua::FromLua for ObjectSpawn {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}
