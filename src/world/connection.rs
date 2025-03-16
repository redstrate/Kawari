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
                unk1: 14,
                unk2: 0,
                op_code: IPCOpCode::ActorSetPos,
                server_id: WORLD_ID,
                timestamp: timestamp_secs(),
                data: IPCStructData::ActorSetPos(ActorSetPos {
                    unk: 0x020fa3b8,
                    position,
                    ..Default::default()
                }),
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
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::UpdateClassInfo,
                server_id: 69, // lol
                timestamp: timestamp_secs(),
                data: IPCStructData::UpdateClassInfo(UpdateClassInfo {
                    class_id: 35,
                    unknown: 1,
                    synced_level: 90,
                    class_level: 90,
                    ..Default::default()
                }),
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // unk10
        {
            let ipc = IPCSegment {
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::Unk10,
                server_id: 69, // lol
                timestamp: timestamp_secs(),
                data: IPCStructData::Unk10 {
                    unk: 0x41a0000000000002,
                },
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // unk9
        {
            let ipc = IPCSegment {
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::Unk9,
                server_id: 69, // lol
                timestamp: timestamp_secs(),
                data: IPCStructData::Unk9 { unk: [0; 24] },
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // TODO: maybe only sent on initial login not every zone?
        // link shell information
        {
            let ipc = IPCSegment {
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::LinkShellInformation,
                server_id: 69, // lol
                timestamp: timestamp_secs(),
                data: IPCStructData::LinkShellInformation { unk: [0; 456] },
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // unk8
        {
            let ipc = IPCSegment {
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::Unk8,
                server_id: 69, // lol
                timestamp: timestamp_secs(),
                data: IPCStructData::Unk8 { unk: [0; 808] },
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }

        // Init Zone
        {
            let ipc = IPCSegment {
                unk1: 0,
                unk2: 0,
                op_code: IPCOpCode::InitZone,
                server_id: 0,
                timestamp: timestamp_secs(),
                data: IPCStructData::InitZone(InitZone {
                    server_id: WORLD_ID,
                    zone_id: self.zone.id,
                    weather_id: 1,
                    ..Default::default()
                }),
            };

            self.send_segment(PacketSegment {
                source_actor: self.player_id,
                target_actor: self.player_id,
                segment_type: SegmentType::Ipc { data: ipc },
            })
            .await;
        }
    }
}
