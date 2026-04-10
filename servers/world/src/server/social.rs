use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    FromServer, ToServer,
    server::{DestinationNetwork, WorldServer, network::NetworkState},
};
use kawari::{common::LogMessageType, ipc::zone::InviteType};

/// Process social invitation and moogle mail-related messages.
pub fn handle_social_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    msg: &ToServer,
) -> bool {
    match msg {
        ToServer::InvitePlayerTo(
            from_actor_id,
            from_account_id,
            from_content_id,
            from_name,
            recipient_actor_id,
            recipient_content_id,
            recipient_character_name,
            invite_type,
        ) => {
            let mut log_message = LogMessageType::Default;
            let mut network = network.lock();

            let is_online;
            {
                let data = data.lock();
                is_online = data.find_actor_instance(*recipient_actor_id).is_some();
            }

            // The client seems to enforce offline friend list requests itself, but we'll still block it.
            if !is_online {
                log_message = LogMessageType::UnableToPerformPlayerOffline;
            } else {
                match invite_type {
                    InviteType::FriendList => {}
                    InviteType::Party => {
                        if network
                            .parties
                            .iter()
                            .filter(|(_, party)| {
                                party
                                    .members
                                    .iter()
                                    .any(|m| m.actor_id == *recipient_actor_id)
                            })
                            .count()
                            > 0
                        {
                            log_message = LogMessageType::PlayerAlreadyInAnotherParty;
                        }
                    }
                    _ => {
                        tracing::warn!(
                            "Unsupported invite type {:#?} sent to ToServer::InvitePlayerTo!",
                            *invite_type
                        );
                        return true;
                    }
                }
            }

            // If all is well, send the invite to the recipient.
            if log_message == LogMessageType::Default {
                let msg = FromServer::SocialInvite(
                    *from_account_id,
                    *from_content_id,
                    from_name.clone(),
                    *invite_type,
                );
                network.send_to_by_actor_id(
                    *recipient_actor_id,
                    msg,
                    DestinationNetwork::ZoneClients,
                );
            }

            // Inform the sender of the invite they just sent.
            let msg = FromServer::InviteCharacterResult(
                *recipient_content_id,
                log_message,
                *invite_type,
                recipient_character_name.clone(),
            );
            network.send_to_by_actor_id(*from_actor_id, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::InvitationResponse(
            from_actor_id,
            from_account_id,
            from_content_id,
            from_name,
            inviter_actor_id,
            inviter_content_id,
            inviter_name,
            invite_type,
            response,
        ) => {
            let mut network = network.lock();

            // TODO: Tell the inviter the invitee has gone offline, if applicable? Does this make sense, and does retail do this? Need to investigate.
            // Tell the invite sender about the invitee's response.
            network.send_to_by_actor_id(
                *inviter_actor_id,
                FromServer::InvitationResult(
                    *from_account_id,
                    *from_content_id,
                    from_name.clone(),
                    *invite_type,
                    *response,
                ),
                DestinationNetwork::ZoneClients,
            );

            // TODO: Send errors back to the invitee if the inviter is offline? Need a capture of this. It's likely that padding in InviteReplyResult is mistaken for a LogMessageType field, similar to InviteCharacterResult.
            // Tell the invitee about their own reply to the inviter.
            network.send_to_by_actor_id(
                *from_actor_id,
                FromServer::InvitationReplyResult(
                    *inviter_content_id,
                    inviter_name.clone(),
                    *invite_type,
                    *response,
                ),
                DestinationNetwork::ZoneClients,
            );

            true
        }
        ToServer::FriendRemoved(
            from_actor_id,
            from_content_id,
            from_name,
            their_actor_id,
            their_content_id,
            their_name,
        ) => {
            let mut network = network.lock();
            // Inform both of them about the removal.
            let msg = FromServer::FriendRemoved(*their_content_id, their_name.clone());
            network.send_to_by_actor_id(*from_actor_id, msg, DestinationNetwork::ZoneClients);

            let msg = FromServer::FriendRemoved(*from_content_id, from_name.clone());
            network.send_to_by_actor_id(*their_actor_id, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::SendLetterTo(recipient_actor_id) => {
            let mut network = network.lock();
            network.send_to_by_actor_id(
                *recipient_actor_id,
                FromServer::NewLetterArrived(),
                DestinationNetwork::ZoneClients,
            );

            true
        }
        _ => false,
    }
}
