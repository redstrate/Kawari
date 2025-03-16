use std::io::Cursor;

use binrw::BinRead;
use tokio::net::TcpStream;

use crate::{
    WORLD_ID,
    common::timestamp_secs,
    ipc::{ActorSetPos, IPCOpCode, IPCSegment, IPCStructData},
    packet::{
        CompressionType, ConnectionType, PacketSegment, SegmentType, State, parse_packet,
        send_packet,
    },
};

use super::{InitZone, Position, UpdateClassInfo, Zone};

pub struct ZoneConnection {
    pub socket: TcpStream,

    pub state: State,
    pub player_id: u32,

    pub zone: Zone,
    pub spawn_index: u8,
}

impl ZoneConnection {
    pub async fn parse_packet(&mut self, data: &[u8]) -> (Vec<PacketSegment>, ConnectionType) {
        parse_packet(data, &mut self.state).await
    }

    pub async fn send_segment(&mut self, segment: PacketSegment) {
        send_packet(
            &mut self.socket,
            &[segment],
            &mut self.state,
            CompressionType::Oodle,
        )
        .await;
    }

    pub async fn set_player_position(&mut self, position: Position) {
        // set pos
        {
            let ipc = IPCSegment {
                op_code: IPCOpCode::ActorSetPos,
                timestamp: timestamp_secs(),
                data: IPCStructData::ActorSetPos(ActorSetPos {
                    unk: 0x020fa3b8,
                    position,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let response_packet = PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            };
            send_packet(
                &mut self.socket,
                &[response_packet],
                &mut self.state,
                CompressionType::Oodle,
            )
            .await;
        }
    }

    pub async fn change_zone(&mut self, new_zone_id: u16) {
        self.zone = Zone::load(new_zone_id);

        // Player Class Info
        {
            let ipc = IPCSegment {
                op_code: IPCOpCode::UpdateClassInfo,
                timestamp: timestamp_secs(),
                data: IPCStructData::UpdateClassInfo(UpdateClassInfo {
                    class_id: 35,
                    unknown: 1,
                    synced_level: 90,
                    class_level: 90,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // link shell information
        {
            let ipc = IPCSegment {
                op_code: IPCOpCode::LinkShellInformation,
                timestamp: timestamp_secs(),
                data: IPCStructData::LinkShellInformation { unk: [0; 456] },
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // TODO: send unk16?

        // Init Zone
        {
            let ipc = IPCSegment {
                op_code: IPCOpCode::InitZone,
                timestamp: timestamp_secs(),
                data: IPCStructData::InitZone(InitZone {
                    server_id: 0,
                    zone_id: self.zone.id,
                    weather_id: 1,
                    ..Default::default()
                }),
                ..Default::default()
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }

    pub fn get_free_spawn_index(&mut self) -> u8 {
        self.spawn_index += 1;
        return self.spawn_index;
    }
}
