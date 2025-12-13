use binrw::binrw;

use crate::{
    common::{
        InvisibilityFlags, ObjectId, Position, read_bool_from, read_quantized_rotation,
        write_bool_as, write_quantized_rotation,
    },
    ipc::zone::ObjectKind,
};

#[binrw]
#[brw(little)]
#[derive(Debug, Copy, Clone, Default)]
pub struct ObjectSpawn {
    /// The object's spawn index. Note that this is a completely separate index from actors.
    pub spawn_index: u8,
    /// What kind of object this is.
    pub kind: ObjectKind,
    /// Seems to control whether or not its targetable?
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub unselectable: bool,
    /// Controls the visibility of the object.
    pub visibility: InvisibilityFlags,
    /// If this is an ENPC, represents an index into the EObj Excel sheet.
    /// If this is an AreaObject, represents an index into the VFX Excel sheet.
    pub base_id: u32,
    pub entity_id: ObjectId,
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
