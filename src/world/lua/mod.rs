mod effects;
pub use effects::EffectsBuilder;

mod inventory;

mod player;
use mlua::{FromLua, Lua, UserData, UserDataFields, Value};
pub use player::LuaPlayer;

mod state;
pub use state::{ExtraLuaState, initial_setup, load_init_script};

mod task;
pub use task::Task;

mod zone;
pub use zone::LuaZone;

use crate::{
    common::{ObjectTypeId, Position},
    ipc::zone::{ObjectSpawn, ServerZoneIpcSegment},
    packet::{PacketSegment, SegmentData, SegmentType},
};

use super::zone_connection::TeleportQuery;

trait QueueSegments {
    fn queue_segment(&mut self, ipc: PacketSegment<ServerZoneIpcSegment>);
}

fn create_ipc_self<T: QueueSegments>(
    user_data: &mut T,
    ipc: ServerZoneIpcSegment,
    source_actor: u32,
) {
    create_ipc_target(user_data, ipc, source_actor, source_actor);
}

fn create_ipc_target<T: QueueSegments>(
    user_data: &mut T,
    ipc: ServerZoneIpcSegment,
    source_actor: u32,
    target_actor: u32,
) {
    let segment = PacketSegment {
        source_actor,
        target_actor,
        segment_type: SegmentType::Ipc,
        data: SegmentData::Ipc(ipc),
    };

    user_data.queue_segment(segment);
}

impl UserData for TeleportQuery {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("aetheryte_id", |_, this| Ok(this.aetheryte_id));
    }
}

impl UserData for Position {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("x", |_, this| Ok(this.x));
        fields.add_field_method_get("y", |_, this| Ok(this.y));
        fields.add_field_method_get("z", |_, this| Ok(this.z));
    }
}

impl UserData for ObjectTypeId {}

impl FromLua for ObjectTypeId {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}

impl UserData for ObjectSpawn {}

impl FromLua for ObjectSpawn {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(*ud.borrow::<Self>()?),
            _ => unreachable!(),
        }
    }
}
