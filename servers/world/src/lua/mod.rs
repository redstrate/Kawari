mod effects_builder;
pub use effects_builder::EffectsBuilder;

mod inventory;

mod player;
use mlua::{UserData, UserDataFields};
pub use player::{LuaContent, LuaPlayer};

mod state;
pub use state::{KawariLua, KawariLuaState};

mod task;
pub use task::LuaTask;

mod zone;
pub use zone::LuaZone;

use kawari::{
    common::ObjectId,
    ipc::zone::ServerZoneIpcSegment,
    packet::{PacketSegment, SegmentData, SegmentType},
};

use super::zone_connection::TeleportQuery;

trait QueueSegments {
    fn queue_segment(&mut self, ipc: PacketSegment<ServerZoneIpcSegment>);
}

fn create_ipc_self<T: QueueSegments>(
    user_data: &mut T,
    ipc: ServerZoneIpcSegment,
    source_actor: ObjectId,
) {
    create_ipc_target(user_data, ipc, source_actor, source_actor);
}

fn create_ipc_target<T: QueueSegments>(
    user_data: &mut T,
    ipc: ServerZoneIpcSegment,
    source_actor: ObjectId,
    target_actor: ObjectId,
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
