use super::common::ClientId;
use crate::common::timestamp_secs;
use crate::config::WorldConfig;
use crate::ipc::chat::{
    ClientChatIpcSegment, ServerChatIpcData, ServerChatIpcSegment, TellMessage, TellNotFoundError,
};
use crate::packet::{
    CompressionType, ConnectionState, ConnectionType, PacketSegment, SegmentData, SegmentType,
    parse_packet, send_keep_alive, send_packet,
};
use crate::world::{MessageInfo, ServerHandle};
use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::TcpStream;

/// Represents a single connection between an instance of the client and chat portion of the world server.
pub struct ChatConnection {
    pub socket: TcpStream,
    pub ip: SocketAddr,
    pub id: ClientId,
    pub state: ConnectionState,
    pub actor_id: u32,
    pub config: WorldConfig,
    pub last_keep_alive: Instant,
    pub handle: ServerHandle,
}

impl ChatConnection {
    pub fn parse_packet(&mut self, data: &[u8]) -> Vec<PacketSegment<ClientChatIpcSegment>> {
        parse_packet(data, &mut self.state)
    }

    /// Sends an IPC segment to the player, where the source actor is also the player.
    pub async fn send_ipc_self(&mut self, ipc: ServerChatIpcSegment) {
        let segment = PacketSegment {
            source_actor: self.actor_id,
            target_actor: self.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        };

        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Chat,
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        )
        .await;
    }

    pub async fn send_ipc(&mut self, ipc: ServerChatIpcSegment, from_actor_id: u32) {
        let segment = PacketSegment {
            source_actor: from_actor_id,
            target_actor: self.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        };

        send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Chat,
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        )
        .await;
    }

    pub async fn initialize(&mut self) {
        {
            // We have to send the client a keep alive!
            let response = PacketSegment::<ServerChatIpcSegment> {
                segment_type: SegmentType::KeepAliveRequest,
                data: SegmentData::KeepAliveRequest {
                    id: 0xE0037603u32,
                    timestamp: timestamp_secs(),
                },
                ..Default::default()
            };
            send_packet(
                &mut self.socket,
                &mut self.state,
                ConnectionType::Chat,
                CompressionType::Oodle,
                &[response],
            )
            .await;
        }

        {
            // initialize connection
            let response = PacketSegment::<ServerChatIpcSegment> {
                segment_type: SegmentType::Initialize,
                data: SegmentData::Initialize {
                    actor_id: self.actor_id,
                    timestamp: timestamp_secs(),
                },
                ..Default::default()
            };
            send_packet(
                &mut self.socket,
                &mut self.state,
                ConnectionType::Chat,
                CompressionType::Oodle,
                &[response],
            )
            .await;
        }

        // send login reply
        {
            let ipc = ServerChatIpcSegment::new(ServerChatIpcData::LoginReply {
                timestamp: 0,
                sid: 0,
            });
            let response = PacketSegment::<ServerChatIpcSegment> {
                segment_type: SegmentType::Ipc,
                data: SegmentData::Ipc(ipc),
                source_actor: self.actor_id,
                target_actor: self.actor_id,
            };
            send_packet(
                &mut self.socket,
                &mut self.state,
                ConnectionType::Chat,
                CompressionType::Oodle,
                &[response],
            )
            .await;
        }
    }

    pub async fn tell_message_received(&mut self, message_info: MessageInfo) {
        let ipc = ServerChatIpcSegment::new(ServerChatIpcData::TellMessage(TellMessage {
            sender_account_id: message_info.sender_account_id,
            sender_world_id: message_info.sender_world_id,
            sender_name: message_info.sender_name,
            message: message_info.message,
            ..Default::default()
        }));

        self.send_ipc(ipc, message_info.sender_actor_id).await;
    }

    pub async fn tell_recipient_not_found(&mut self, error_info: TellNotFoundError) {
        let ipc = ServerChatIpcSegment::new(ServerChatIpcData::TellNotFoundError(error_info));

        self.send_ipc_self(ipc).await;
    }

    pub async fn send_keep_alive(&mut self, id: u32, timestamp: u32) {
        send_keep_alive::<ServerChatIpcSegment>(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Chat,
            id,
            timestamp,
        )
        .await;
    }
}
