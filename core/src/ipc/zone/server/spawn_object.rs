use binrw::binrw;

use crate::{
    common::{
        EventState, HandlerId, ObjectId, Position, read_bool_from, read_quantized_rotation,
        write_bool_as, write_quantized_rotation,
    },
    ipc::zone::ObjectKind,
};

#[binrw]
#[brw(little)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SpawnObject {
    /// The object's spawn index. Note that this is a completely separate index from actors.
    pub spawn_index: u8,
    /// What kind of object this is.
    pub kind: ObjectKind,
    /// Whether this object should be targetable or not.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub not_targetable: bool,
    /// Whether this object is initially hidden or not.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub is_hidden: bool,
    /// If this is an ENPC, represents an index into the EObj Excel sheet.
    /// If this is an AreaObject, represents an index into the VFX Excel sheet.
    pub base_id: u32,
    /// Unique ID of this object.
    pub entity_id: ObjectId,
    /// Instance ID of the EventObject in the LGB.
    pub layout_id: u32,
    /// The event handler that owns this object.
    pub handler_id: HandlerId,
    /// The owner actor for this object.
    pub owner_id: ObjectId,
    /// Bound ID of the EventObject in the LGB, usually a SharedGroup.
    pub bind_layout_id: u32,
    /// Radius of the hitbox(?) If `bind_layout_id` is set, this value is ignored.
    #[brw(pad_after = 2)] // padding for alignment, not read by the client
    pub radius: f32,
    /// The rotation to create the object, in radians.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// The FATE to associate with.
    pub fate_id: u16,
    /// Controls the visibility of the event object.
    #[brw(pad_after = 5)] // padding for alignment, and then an unused u32
    pub event_state: EventState,
    /// For EventObjs, this is the default SharedGroupTimelineState.
    pub args1: u32,
    /// Part of this is used for housing entrances. EventObj uses this too, but it varies based on SubKind.
    pub args2: u32,
    /// The position to place this object at.
    pub position: Position,
}

#[cfg(feature = "server")]
impl mlua::UserData for SpawnObject {}

#[cfg(feature = "server")]
impl mlua::FromLua for SpawnObject {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}
