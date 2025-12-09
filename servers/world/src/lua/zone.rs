use std::collections::HashMap;

use mlua::{LuaSerdeExt, UserData, UserDataFields, UserDataMethods, Value};

use kawari::{
    common::ObjectId,
    ipc::zone::{ObjectSpawn, ServerZoneIpcData, ServerZoneIpcSegment},
    packet::PacketSegment,
};

use super::{QueueSegments, create_ipc_target};

#[derive(Default, Debug, Clone)]
pub struct LuaZone {
    pub zone_id: u16,
    pub weather_id: u16,
    pub internal_name: String,
    pub region_name: String,
    pub place_name: String,
    pub intended_use: u8,
    pub map_id: u16,
    pub queued_segments: Vec<PacketSegment<ServerZoneIpcSegment>>,
    // TODO: lol, this is only here for the get_npc_base_id function
    pub cached_npc_base_ids: HashMap<u32, u32>,
}

impl UserData for LuaZone {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.zone_id));
        fields.add_field_method_get("weather_id", |_, this| Ok(this.weather_id));
        fields.add_field_method_get("internal_name", |_, this| Ok(this.internal_name.clone()));
        fields.add_field_method_get("region_name", |_, this| Ok(this.region_name.clone()));
        fields.add_field_method_get("place_name", |_, this| Ok(this.place_name.clone()));
        fields.add_field_method_get("intended_use", |_, this| Ok(this.intended_use));
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("spawn_eobj", |lua, this, eobj: Value| {
            let eobj: ObjectSpawn = lua.from_value(eobj).unwrap();
            this.spawn_eobj(eobj);
            Ok(())
        });
        methods.add_method("get_npc_base_id", |_, this, instance_id: u32| {
            Ok(this.cached_npc_base_ids.get(&instance_id).copied())
        });
    }
}

impl QueueSegments for LuaZone {
    fn queue_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        self.queued_segments.push(segment);
    }
}

impl LuaZone {
    fn spawn_eobj(&mut self, eobj: ObjectSpawn) {
        let data = ServerZoneIpcSegment::new(ServerZoneIpcData::ObjectSpawn(eobj));

        create_ipc_target(self, data, ObjectId(eobj.entity_id), ObjectId(0)); // NOTE: Setting the target actor id to 0 for later post-processing.
    }
}
