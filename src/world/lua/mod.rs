mod effects;
pub use effects::EffectsBuilder;

mod inventory;

mod player;
use mlua::{FromLua, Lua, UserData, UserDataFields, UserDataMethods, Value};
pub use player::LuaPlayer;

mod state;
pub use state::{ExtraLuaState, initial_setup, load_init_script};

mod task;
pub use task::Task;

mod zone;
pub use zone::LuaZone;

use crate::{
    common::{GameData, ObjectTypeId, Position},
    ipc::zone::{ObjectSpawn, ServerZoneIpcSegment, StatusEffect},
    packet::{PacketSegment, SegmentData, SegmentType},
};

use super::{Event, zone_connection::TeleportQuery};

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

impl UserData for ObjectTypeId {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("object_id", |_, this| Ok(this.object_id.0));
    }
}

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

impl FromLua for Event {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        match value {
            Value::UserData(ud) => Ok(ud.borrow::<Self>()?.to_owned()),
            _ => unreachable!(),
        }
    }
}

impl UserData for Event {}

impl UserData for GameData {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("is_aetheryte", |_, this, aetheryte_id: u32| {
            Ok(this.is_aetheryte(aetheryte_id))
        });
        methods.add_method_mut("get_warp_logic_name", |_, this, warp_id: u32| {
            Ok(this.get_warp_logic_name(warp_id))
        });
        methods.add_method_mut("get_custom_talk_name", |_, this, custom_talk_id: u32| {
            Ok(this.get_custom_talk_name(custom_talk_id))
        });
        methods.add_method_mut("get_opening_name", |_, this, opening_id: u32| {
            Ok(this.get_opening_name(opening_id))
        });
        methods.add_method_mut("get_pre_handler_target", |_, this, pre_handler_id: u32| {
            Ok(this.get_pre_handler_target(pre_handler_id))
        });
        methods.add_method_mut("get_switch_talk_target", |_, this, switch_talk_id: u32| {
            Ok(this.get_switch_talk_target(switch_talk_id))
        });
        methods.add_method_mut("get_halloween_npc_transform", |_, this, npc_id: u32| {
            Ok(this.get_halloween_npc_transform(npc_id))
        });
        methods.add_method_mut("get_quest_name", |_, this, quest_id: u32| {
            Ok(this.get_quest_name(quest_id))
        });
        methods.add_method_mut(
            "get_topic_select_target",
            |_, this, (topic_select_id, selected_topic): (u32, usize)| {
                Ok(this.get_topic_select_target(topic_select_id, selected_topic))
            },
        );
    }
}

impl UserData for StatusEffect {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("param", |_, this| Ok(this.param));
    }
}
