use std::{sync::Arc, time::Instant};

use parking_lot::Mutex;
use tokio::net::TcpStream;

use super::common::ClientId;
use crate::{ServerHandle, ToServer, WorldDatabase, database::Character};
use kawari::{
    common::{ObjectId, timestamp_secs},
    config::WorldConfig,
    ipc::{
        chat::{
            CWLinkshellMessage, ChatChannel, ChatChannelType, ClientChatIpcSegment, PartyMessage,
            SendCWLinkshellMessage, SendPartyMessage, SendTellMessage, ServerChatIpcData,
            ServerChatIpcSegment, TellMessage, TellNotFoundError,
        },
        zone::{CrossworldLinkshellEx, OnlineStatus},
    },
    opcodes::ServerChatIpcType,
    packet::{
        CompressionType, ConnectionState, ConnectionType, IpcSegmentHeader, PacketSegment,
        SegmentData, SegmentType, ServerIpcSegmentHeader, parse_packet, send_keep_alive,
        send_packet,
    },
};

/// Represents a single connection between an instance of the client and chat portion of the world server.
pub struct ChatConnection {
    pub socket: TcpStream,
    pub id: ClientId,
    pub state: ConnectionState,
    pub database: Arc<Mutex<WorldDatabase>>,
    pub player_data: ChatPlayerData,
    pub config: WorldConfig,
    pub last_keep_alive: Instant,
    pub handle: ServerHandle,
    pub chatchannels: ChatConnectionChannels,
}

/// Miniature version of the ZoneConnection's PlayerData. We cache a few things so the global server doesn't have to constantly look it up on our behalf when we send messages.
#[derive(Default)]
pub struct ChatPlayerData {
    pub actor_id: ObjectId,
    pub account_id: u64,
    pub content_id: u64,
    pub name: String,
}

/// Holds all of the ChatConnection's ChatChannels.
#[derive(Default)]
pub struct ChatConnectionChannels {
    ///The party's ChatChannel.
    pub party: ChatChannel,
    /// Cross-world linkshells' ChatChannels.
    pub cwls: [ChatChannel; CrossworldLinkshellEx::COUNT],
    /// Local-world linkshells' ChatChannels.
    pub lwls: [ChatChannel; CrossworldLinkshellEx::COUNT],
}

impl ChatConnection {
    pub fn parse_packet(&mut self, data: &[u8]) -> Vec<PacketSegment<ClientChatIpcSegment>> {
        parse_packet(data, &mut self.state)
    }

    /// Sends an IPC segment to the player, where the source actor is also the player.
    pub async fn send_ipc_self(&mut self, ipc: ServerChatIpcSegment) {
        // This is meant to protect against stack-smashing in nested futures
        Box::pin(self.send_ipc_from(self.player_data.actor_id, ipc)).await;
    }

    /// Sends an IPC segment to the player, where the source actor can be specified.
    pub async fn send_ipc_from(&mut self, source_actor: ObjectId, ipc: ServerChatIpcSegment) {
        let segment = PacketSegment {
            source_actor,
            target_actor: self.player_data.actor_id,
            segment_type: SegmentType::Ipc,
            data: SegmentData::Ipc(ipc),
        };

        // Ditto from above
        Box::pin(self.send_segment(segment)).await;
    }

    pub async fn send_segment(&mut self, segment: PacketSegment<ServerChatIpcSegment>) {
        // Ditto as above
        Box::pin(send_packet(
            &mut self.socket,
            &mut self.state,
            ConnectionType::Chat,
            if self.config.enable_packet_compression {
                CompressionType::Oodle
            } else {
                CompressionType::Uncompressed
            },
            &[segment],
        ))
        .await;
    }

    pub async fn initialize(&mut self) {
        {
            tracing::info!(
                "Client {} is initializing chat session...",
                self.player_data.actor_id
            );

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
                    actor_id: self.player_data.actor_id,
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
                source_actor: self.player_data.actor_id,
                target_actor: self.player_data.actor_id,
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
        self.chatchannels.party.world_id = self.config.world_id;
        self.chatchannels.party.channel_type = ChatChannelType::Party;

        for linkshell in self.chatchannels.cwls.iter_mut() {
            linkshell.world_id = 10008; // This seems to always be used for CWLSes.
            linkshell.channel_type = ChatChannelType::CWLinkshell;
        }

        for linkshell in self.chatchannels.lwls.iter_mut() {
            linkshell.world_id = self.config.world_id;
            linkshell.channel_type = ChatChannelType::Linkshell;
        }
    }

    pub async fn send_tell_message(&mut self, tell_data: &SendTellMessage) {
        // Start with the assumption that the recipient doesn't exist or is offline.
        let mut recipient_ids = Character::default();
        let mut recipient_is_online = false;
        {
            let mut db = self.database.lock();

            // Tells only give us the recipient's name which isn't very helpful, so we need further info.
            if let Some(recipient_id) =
                db.find_character_ids(None, Some(tell_data.recipient_name.clone()))
            {
                recipient_ids = recipient_id;
                let mask = db.determine_online_status_mask(recipient_ids.content_id);
                recipient_is_online = mask.has_status(OnlineStatus::Online);
            }
        }

        // Next, if they do exist and are online, tell the server where to send the message.
        if recipient_is_online && recipient_ids.actor_id != ObjectId::default() {
            self.handle
                .send(ToServer::TellMessageSent(
                    self.player_data.actor_id,
                    recipient_ids.actor_id,
                    TellMessage {
                        sender_account_id: self.player_data.account_id,
                        sender_content_id: self.player_data.content_id,
                        sender_world_id: self.config.world_id,
                        sender_name: self.player_data.name.clone(),
                        message: tell_data.message.clone(),
                        ..Default::default()
                    },
                ))
                .await;

            // Quit early, everything after is error handling.
            return;
        }

        let ipc =
            ServerChatIpcSegment::new(ServerChatIpcData::TellNotFoundError(TellNotFoundError {
                recipient_account_id: recipient_ids.service_account_id as u64,
                sender_account_id: self.player_data.account_id,
                recipient_world_id: self.config.world_id,
                recipient_name: tell_data.recipient_name.clone(),
                unk: 0x68, // No clue what this is, but it's often seen when receiving a tell error.
            }));

        self.send_ipc_self(ipc).await;
    }

    pub async fn tell_message_received(
        &mut self,
        sender_actor_id: ObjectId,
        message_info: TellMessage,
    ) {
        let ipc = ServerChatIpcSegment::new(ServerChatIpcData::TellMessage(message_info));

        self.send_ipc_from(sender_actor_id, ipc).await;
    }

    pub async fn send_party_message(&mut self, message_data: &SendPartyMessage) {
        if message_data.chatchannel == self.chatchannels.party {
            let party_message = PartyMessage {
                party_chatchannel: self.chatchannels.party,
                sender_account_id: self.player_data.account_id,
                sender_content_id: self.player_data.content_id,
                sender_actor_id: self.player_data.actor_id,
                sender_world_id: self.config.world_id,
                sender_name: self.player_data.name.clone(),
                message: message_data.message.clone(),
            };
            self.handle
                .send(ToServer::PartyMessageSent(party_message))
                .await;
        } else {
            tracing::error!(
                "The client tried to send a party message to an invalid ChatChannel: {:#?}, while ours is {:#?}",
                message_data.chatchannel,
                self.chatchannels.party
            );
        }
    }

    pub async fn party_message_received(&mut self, message_info: PartyMessage) {
        if message_info.party_chatchannel == self.chatchannels.party {
            let sender_actor_id = message_info.sender_actor_id;
            let ipc = ServerChatIpcSegment::new(ServerChatIpcData::PartyMessage(message_info));

            self.send_ipc_from(sender_actor_id, ipc).await;
        } else {
            tracing::error!(
                "party_message_received: We received a message not destined for our party! What happened? Discarding message. The destination chatchannel was {:#?}",
                message_info.party_chatchannel
            );
        }
    }

    // TODO: Probably see if we can have one generic function for both cwls and lcls
    pub async fn send_linkshell_message(&mut self, message_data: &SendCWLinkshellMessage) {
        if self.chatchannels.cwls.contains(&message_data.chatchannel) {
            self.handle
                .send(ToServer::CWLSMessageSent(CWLinkshellMessage {
                    cwls_chatchannel: message_data.chatchannel,
                    sender_account_id: self.player_data.account_id,
                    sender_content_id: self.player_data.content_id,
                    sender_home_world_id: self.config.world_id,
                    sender_current_world_id: self.config.world_id,
                    sender_actor_id: self.player_data.actor_id,
                    sender_name: self.player_data.name.clone(),
                    message: message_data.message.clone(),
                }))
                .await;
        } else {
            tracing::error!(
                "The client tried to send a linkshell message to an invalid ChatChannel: {:#?}, while ours are {:#?}",
                message_data.chatchannel,
                self.chatchannels.cwls
            );
        }
    }

    pub async fn cwls_message_received(&mut self, message_info: CWLinkshellMessage) {
        if !self
            .chatchannels
            .cwls
            .contains(&message_info.cwls_chatchannel)
        {
            tracing::error!(
                "cwls_message_received: We received a message not destined for one of our linkshells, what happened? Discarding message. The destination linkshell was {:#?}",
                message_info.cwls_chatchannel
            );
            return;
        }

        // TODO: Filter messages if our rank is Invitee
        let sender_actor_id = message_info.sender_actor_id;
        let ipc = ServerChatIpcSegment::new(ServerChatIpcData::CWLinkshellMessage(message_info));

        self.send_ipc_from(sender_actor_id, ipc).await;
    }

    pub async fn refresh_chatchannels(&mut self) {
        let linkshells;
        {
            let mut db = self.database.lock();
            linkshells = db.find_linkshells(self.player_data.content_id as i64);
        }

        if let Some(linkshells) = linkshells {
            // TODO: local shells
            for (index, shell) in linkshells.iter().enumerate() {
                if index >= self.chatchannels.cwls.len() {
                    break;
                }
                self.chatchannels.cwls[index].channel_number = shell.ids.linkshell_id as u32;
            }
        }
    }

    pub async fn set_party_chatchannel(&mut self, party_channel_number: u32) {
        self.chatchannels.party.channel_number = party_channel_number;
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
