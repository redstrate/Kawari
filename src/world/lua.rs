use mlua::{UserData, UserDataMethods};

use crate::{
    common::timestamp_secs,
    opcodes::ServerZoneIpcType,
    packet::{PacketSegment, SegmentType},
};

use super::{
    PlayerData, StatusEffects,
    ipc::{ServerZoneIpcData, ServerZoneIpcSegment},
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
    }
}
