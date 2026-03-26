use super::common::ClientId;
use crate::{MessageInfo, ServerHandle};
use kawari::common::{ObjectId, timestamp_secs};
use kawari::config::WorldConfig;
use kawari::ipc::{
    chat::{
        CWLinkshellMessage, ChatChannel, ChatChannelType, ClientChatIpcSegment, PartyMessage,
        ServerChatIpcData, ServerChatIpcSegment, TellMessage, TellNotFoundError,
    },
    zone::CrossworldLinkshellEx,
};

use kawari::opcodes::ServerChatIpcType;
use kawari::packet::IpcSegmentHeader;
use kawari::packet::{
    CompressionType, ConnectionState, ConnectionType, PacketSegment, SegmentData, SegmentType,
    ServerIpcSegmentHeader, parse_packet, send_keep_alive, send_packet,
};
use std::time::Instant;
use tokio::net::TcpStream;

/// Represents a single connection between an instance of the client and chat portion of the world server.
pub struct ChatConnection {
    pub socket: TcpStream,
    pub id: ClientId,
    pub state: ConnectionState,
    pub actor_id: ObjectId,
    pub config: WorldConfig,
    pub last_keep_alive: Instant,
    pub handle: ServerHandle,
    pub party_chatchannel: ChatChannel,
    pub cwls_chatchannels: [ChatChannel; CrossworldLinkshellEx::COUNT],
    pub local_ls_chatchannels: [ChatChannel; CrossworldLinkshellEx::COUNT],
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

    pub async fn send_ipc(&mut self, ipc: ServerChatIpcSegment, from_actor_id: ObjectId) {
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

        // Do some initial setup to prepare all of our chatchannels. Our chat connection mainly acts as a filter between the client's chat connection and our global server state. The global state will eventually fill in our channel numbers as needed.
        self.party_chatchannel.world_id = self.config.world_id;
        self.party_chatchannel.channel_type = ChatChannelType::Party;

        for linkshell in self.cwls_chatchannels.iter_mut() {
            linkshell.world_id = 10008; // This seems to always be used for CWLSes.
            linkshell.channel_type = ChatChannelType::CWLinkshell;
        }

        for linkshell in self.local_ls_chatchannels.iter_mut() {
            linkshell.world_id = self.config.world_id;
            linkshell.channel_type = ChatChannelType::Linkshell
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

    pub async fn party_message_received(&mut self, message_info: PartyMessage) {
        if message_info.party_chatchannel != self.party_chatchannel {
            tracing::error!(
                "party_message_received: We received a message not destined for our party! What happened? Discarding message. The destination chatchannel was {:#?}",
                message_info.party_chatchannel
            );
            return;
        }

        let sender_actor_id = message_info.sender_actor_id;
        let ipc = ServerChatIpcSegment::new(ServerChatIpcData::PartyMessage(message_info));

        self.send_ipc(ipc, sender_actor_id).await;
    }

    pub async fn cwls_message_received(&mut self, message_info: CWLinkshellMessage) {
        if !self
            .cwls_chatchannels
            .contains(&message_info.cwls_chatchannel)
        {
            tracing::error!(
                "cwls_message_received: We received a message not destined for one of our linkshells, what happened? Discarding message. The destination linkshell was {:#?}",
                message_info.cwls_chatchannel
            );
            return;
        }

        let sender_actor_id = message_info.sender_actor_id;
        let ipc = ServerChatIpcSegment::new(ServerChatIpcData::CWLinkshellMessage(message_info));

        self.send_ipc(ipc, sender_actor_id).await;
    }

    pub async fn set_linkshell_chatchannels(&mut self, cwlses: Vec<u32>, locals: Vec<u32>) {
        if (cwlses.len() > self.cwls_chatchannels.len())
            || (locals.len() > self.local_ls_chatchannels.len())
        {
            tracing::error!(
                "set_linkshell_chatchannels: cwlses ({}) or locals ({})vecs had too many entries! What happened?",
                cwlses.len(),
                locals.len()
            );
            return;
        }

        for (index, ls) in cwlses.iter().enumerate() {
            self.cwls_chatchannels[index].channel_number = *ls;
        }

        for (index, ls) in locals.iter().enumerate() {
            self.local_ls_chatchannels[index].channel_number = *ls;
        }
    }

    pub async fn linkshell_disbanded(&mut self, channel_number: u32) {
        for linkshell in self.cwls_chatchannels.iter_mut() {
            if linkshell.channel_number == channel_number {
                linkshell.channel_number = 0;
                return;
            }
        }

        for linkshell in self.local_ls_chatchannels.iter_mut() {
            if linkshell.channel_number == channel_number {
                linkshell.channel_number = 0;
                break;
            }
        }
    }

    pub async fn linkshell_left(&mut self, from_actor_id: ObjectId, channel_number: u32) {
        for linkshell in self.cwls_chatchannels.iter_mut() {
            if linkshell.channel_number == channel_number && self.actor_id == from_actor_id {
                linkshell.channel_number = 0;
                return;
            }
        }

        for linkshell in self.local_ls_chatchannels.iter_mut() {
            if linkshell.channel_number == channel_number && self.actor_id == from_actor_id {
                linkshell.channel_number = 0;
                break;
            }
        }
    }

    pub async fn set_party_chatchannel(&mut self, party_channel_number: u32) {
        self.party_chatchannel.channel_number = party_channel_number;
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

    pub async fn send_arbitrary_packet(&mut self, op_code: u16, data: Vec<u8>) {
        let ipc = ServerChatIpcSegment {
            header: ServerIpcSegmentHeader::from_opcode(ServerChatIpcType::Unknown(op_code)),
            data: ServerChatIpcData::Unknown { unk: data },
        };
        self.send_ipc_self(ipc).await;
    }
}
