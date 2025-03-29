use mlua::{UserData, UserDataMethods};

use crate::{
    common::{ObjectId, ObjectTypeId, timestamp_secs},
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentType},
};

use super::{
    PlayerData, StatusEffects,
    ipc::{EventPlay, ServerZoneIpcData, ServerZoneIpcSegment},
};

#[derive(Default)]
pub struct LuaPlayer {
    pub player_data: PlayerData,
    pub status_effects: StatusEffects,
    pub queued_segments: Vec<PacketSegment<ServerZoneIpcSegment>>,
}

impl LuaPlayer {
    fn queue_segment(&mut self, segment: PacketSegment<ServerZoneIpcSegment>) {
        self.queued_segments.push(segment);
    }

    fn send_message(&mut self, message: &str) {
        let ipc = ServerZoneIpcSegment {
            op_code: ServerZoneIpcType::ServerChatMessage,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::ServerChatMessage {
                message: message.to_string(),
                unk: 0,
            },
            ..Default::default()
        };

        self.queue_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        });
    }

    fn give_status_effect(&mut self, effect_id: u16, duration: f32) {
        self.status_effects.add(effect_id, duration);
    }

    fn play_scene(&mut self, event_id: u32, scene: u16, scene_flags: u32, param: u8) {
        let ipc = ServerZoneIpcSegment {
            unk1: 20,
            unk2: 0,
            op_code: ServerZoneIpcType::EventPlay,
            server_id: 0,
            timestamp: timestamp_secs(),
            data: ServerZoneIpcData::EventPlay(EventPlay {
                actor_id: ObjectTypeId {
                    object_id: ObjectId(self.player_data.actor_id),
                    object_type: 0,
                },
                event_id,
                scene,
                scene_flags,
                unk2: param,
                ..Default::default()
            }),
        };

        self.queue_segment(PacketSegment {
            source_actor: self.player_data.actor_id,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc { data: ipc },
        });
    }
}

impl UserData for LuaPlayer {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("send_message", |_, this, message: String| {
            this.send_message(&message);
            Ok(())
        });
        methods.add_method_mut(
            "give_status_effect",
            |_, this, (effect_id, duration): (u16, f32)| {
                this.give_status_effect(effect_id, duration);
                Ok(())
            },
        );
        methods.add_method_mut(
            "play_scene",
            |_, this, (event_id, scene, scene_flags, param): (u32, u16, u32, u8)| {
                this.play_scene(event_id, scene, scene_flags, param);
                Ok(())
            },
        );
    }
}
