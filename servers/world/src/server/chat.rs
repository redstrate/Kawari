use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    FromServer, ToServer,
    server::{
        WorldServer,
        network::{DestinationNetwork, NetworkState},
    },
};

/// Process chat-related messages.
pub fn handle_chat_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    msg: &ToServer,
) -> bool {
    match msg {
        ToServer::Message(from_actor_id, msg) => {
            let mut network = network.lock();

            let data = data.lock();

            // First grab the sender's instance, since zone chat operates in the same zone as the sender.
            let Some(sender_instance) = data.find_actor_instance(*from_actor_id) else {
                panic!("Client is somehow not in an instance yet?!");
            };

            network.send_to_instance(
                *from_actor_id,
                sender_instance,
                FromServer::Message(msg.clone()),
                DestinationNetwork::ZoneClients,
            );

            true
        }
        ToServer::TellMessageSent(from_actor_id, recipient_actor_id, message_data) => {
            let mut network = network.lock();

            network.send_to_by_actor_id(
                *recipient_actor_id,
                FromServer::TellMessageReceived(*from_actor_id, message_data.clone()),
                DestinationNetwork::ChatClients,
            );

            true
        }
        ToServer::PartyMessageSent(party_message) => {
            let mut network = network.lock();

            // Find the party id from the chatchannel id. The ChatConnection isn't privy to the party id and has no need for it.
            let Some(id) = network.parties.iter().find_map(|(key, val)| {
                (val.chatchannel_id == party_message.party_chatchannel.channel_number)
                    .then_some(key)
            }) else {
                return true;
            };

            let party_id = *id;

            let from_actor_id = party_message.sender_actor_id;
            let msg = FromServer::PartyMessageReceived(party_message.clone());

            // Skip the sender to avoid echoing messages
            network.send_to_party(
                party_id,
                Some(from_actor_id),
                msg,
                DestinationNetwork::ChatClients,
            );

            true
        }
        ToServer::CWLSMessageSent(linkshell_message) => {
            let mut network = network.lock();

            let linkshell_id = linkshell_message.cwls_chatchannel.channel_number as u64;

            let from_actor_id = linkshell_message.sender_actor_id;
            let msg = FromServer::CWLSMessageReceived(linkshell_message.clone());

            network.send_to_linkshell(
                linkshell_id,
                Some(from_actor_id),
                msg,
                DestinationNetwork::ChatClients,
            );

            true
        }
        _ => false,
    }
}
