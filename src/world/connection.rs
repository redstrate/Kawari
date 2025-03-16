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

use super::Position;

pub struct ZoneConnection {
    pub socket: TcpStream,

    pub state: State,
    pub player_id: u32,
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
}
