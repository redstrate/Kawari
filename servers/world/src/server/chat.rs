use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    FromServer, ToServer,
    server::{
        WorldServer,
        network::{DestinationNetwork, NetworkState},
    },
};
use kawari::ipc::chat::CWLinkshellMessage;

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
        ToServer::CWLSMessageSent(from_actor_id, message_info) => {
            let mut network = network.lock();
            let data = data.lock();

            let Some(instance) = data.find_actor_instance(*from_actor_id) else {
                return true;
            };

            let Some(sender_actor) = instance.find_actor(*from_actor_id) else {
                return true;
            };

            let Some(sender) = sender_actor.get_player_spawn() else {
                return true;
            };

            let cwls_message = CWLinkshellMessage {
                cwls_chatchannel: message_info.chatchannel,
                sender_account_id: sender.account_id,
                sender_content_id: sender.content_id,
                sender_home_world_id: sender.home_world_id,
                sender_current_world_id: sender.current_world_id,
                sender_actor_id: *from_actor_id,
                sender_name: sender.common.name.clone(),
                message: message_info.message.clone(),
            };

            let mut linkshell_id = None;

            // We need some info about the destination LS since the chat connection doesn't provide it.
            for (id, linkshell) in &network.linkshells {
                if linkshell.channel_number == message_info.chatchannel.channel_number {
                    linkshell_id = Some(*id);
                    break;
                }
            }

            let Some(linkshell_id) = linkshell_id else {
                return true;
            };

            let msg = FromServer::CWLSMessageSent(cwls_message);

            network.send_to_linkshell(
                linkshell_id,
                Some(*from_actor_id),
                msg,
                DestinationNetwork::ChatClients,
            );

            true
        }
        _ => false,
    }
}
