use std::sync::{Arc, Mutex};

use crate::{
    common::INVALID_OBJECT_ID,
    ipc::chat::{PartyMessage, TellNotFoundError},
    world::{
        FromServer, MessageInfo, ToServer,
        server::{
            WorldServer,
            network::{DestinationNetwork, NetworkState},
            social::PartyMember,
        },
    },
};

/// Process chat-related messages.
pub fn handle_chat_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    msg: &ToServer,
) {
    match msg {
        ToServer::Message(from_id, msg) => {
            let mut network = network.lock().unwrap();

            network.send_to_all(
                Some(*from_id),
                FromServer::Message(msg.clone()),
                DestinationNetwork::ZoneClients,
            );
        }
        ToServer::TellMessageSent(from_id, from_actor_id, message_info) => {
            // TODO: Maybe this can be simplified with fewer loops?

            let mut network = network.lock().unwrap();
            let data = data.lock().unwrap();

            // First pull up some info about the sender, as tell packets require it
            let Some(sender_instance) = data.find_actor_instance(*from_actor_id) else {
                panic!("ToServer::TellMessageSent: Unable to find the sender! What happened?");
            };

            let mut sender_name = "".to_string();
            let mut sender_world_id = 0;
            let mut sender_account_id = 0;

            for (id, actor) in &sender_instance.actors {
                if id.0 == *from_actor_id {
                    let Some(spawn) = actor.get_player_spawn() else {
                        panic!("Why are we trying to get the PlayerSpawn of an NPC?");
                    };

                    sender_name = spawn.common.name.clone();
                    sender_world_id = spawn.home_world_id;
                    sender_account_id = spawn.account_id;
                    break;
                }
            }

            // If the sender wasn't found in the instance we already found them to be in, reality has apparently broken
            assert!(sender_world_id != 0);

            let mut recipient_actor_id = INVALID_OBJECT_ID;

            // Second, look up the recipient by name, since that and their world id are all we're given by the sending client.
            // Since we don't implement multiple worlds, the world id isn't useful for anything here.
            'outer: for instance in data.instances.values() {
                for (id, actor) in &instance.actors {
                    if actor.get_common_spawn().name == message_info.recipient_name {
                        recipient_actor_id = *id;
                        break 'outer;
                    }
                }
            }

            // Next, if the recipient is online, fetch their handle from the network and send them the message!
            if recipient_actor_id != INVALID_OBJECT_ID {
                let message_info = MessageInfo {
                    sender_actor_id: *from_actor_id,
                    sender_account_id,
                    sender_name: sender_name.clone(),
                    sender_world_id,
                    message: message_info.message.clone(),
                    ..Default::default()
                };

                network.send_to_by_actor_id(
                    recipient_actor_id.0,
                    FromServer::TellMessageSent(message_info),
                    DestinationNetwork::ChatClients,
                );
            } else {
                // Else, if the recipient is offline, inform the sender.
                let response = TellNotFoundError {
                    sender_account_id,
                    recipient_world_id: sender_world_id, // It doesn't matter if it's the sender's, since we don't implement multiple worlds.
                    recipient_name: message_info.recipient_name.clone(),
                    ..Default::default()
                };

                network.send_to(
                    *from_id,
                    FromServer::TellRecipientNotFound(response),
                    DestinationNetwork::ChatClients,
                );
            }
        }
        ToServer::PartyMessageSent(from_actor_id, message_info) => {
            let mut network = network.lock().unwrap();

            let mut sender = PartyMember::default();
            let mut party_id = 0;

            // We need some info about the sender since our chat connection doesn't provide it.
            for (id, party) in &network.parties {
                if party.chatchannel_id == message_info.chatchannel.channel_number {
                    party_id = *id;
                    for member in &party.members {
                        if member.actor_id.0 == *from_actor_id {
                            sender = member.clone();
                        }
                    }
                }
            }

            assert!(party_id != 0 && sender.actor_id != INVALID_OBJECT_ID);

            let party_message = PartyMessage {
                party_chatchannel: message_info.chatchannel,
                sender_account_id: sender.account_id,
                sender_content_id: sender.content_id,
                sender_world_id: sender.world_id,
                sender_actor_id: sender.actor_id.0,
                sender_name: sender.name.clone(),
                message: message_info.message.clone(),
            };
            let msg = FromServer::PartyMessageSent(party_message);

            // Skip the original sender to avoid echoing messages
            network.send_to_party(
                party_id,
                Some(*from_actor_id),
                msg,
                DestinationNetwork::ChatClients,
            );
        }
        _ => {}
    }
}
