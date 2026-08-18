use std::sync::Arc;

use bstr::BString;
use kawari::{
    common::{DEBUG_COMMAND_TRIGGER, DirectorEvent, ObjectId, WarpType},
    ipc::zone::{
        ActionRequest, ActionType, ActorControlCategory, ServerNoticeMessage, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, ToServer,
    lua::KawariLua,
    server::{
        WorldServer,
        action::execute_action,
        actor::spawn_custom_bnpc,
        fate::{FateInstance, inform_fate_spawn_globally},
        network::{DestinationNetwork, NetworkState},
        zone::change_zone_warp_to_pop_range,
    },
};

/// Process chat-related messages.
pub fn handle_chat_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    game_data: Arc<Mutex<GameData>>,
    lua: Arc<Mutex<KawariLua>>,
    msg: &ToServer,
) -> bool {
    match msg {
        ToServer::Message(from_id, from_actor_id, msg) => {
            if msg.message.to_string().starts_with(DEBUG_COMMAND_TRIGGER) {
                // Process any server-side debug commands
                if !process_debug_commands(
                    network.clone(),
                    data.clone(),
                    game_data.clone(),
                    lua.clone(),
                    *from_id,
                    *from_actor_id,
                    &msg.message,
                ) {
                    // If it's truly not existent...
                    let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
                        ServerNoticeMessage {
                            message: format!("Unknown command {}", msg.message),
                            ..Default::default()
                        },
                    ));

                    let mut network = network.lock();
                    network.send_to(
                        *from_id,
                        FromServer::PacketSegment(ipc, *from_actor_id),
                        DestinationNetwork::ZoneClients,
                    );

                    return true; // Don't broadcast to other players.
                }
            }

            // If it wasn't a debug command, send to other players:
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

/// Returns true if the debug command is handled, otherwise false.
fn process_debug_commands(
    network: Arc<Mutex<NetworkState>>,
    data: Arc<Mutex<WorldServer>>,
    game_data: Arc<Mutex<GameData>>,
    lua: Arc<Mutex<KawariLua>>,
    from_id: ClientId,
    from_actor_id: ObjectId,
    chat_message: &BString,
) -> bool {
    // TODO: Ensure the message has no SEString macros (e.g. auto-translate phrases)?
    let chat_message = chat_message.to_string();

    let parts: Vec<&str> = chat_message.split(' ').collect();

    match parts[0] {
        "!spawnmonster" => {
            if let Some((_, id)) = chat_message.split_once(' ')
                && let Ok(id) = id.parse::<u32>()
            {
                let mut data = data.lock();
                let mut game_data = game_data.lock();

                spawn_custom_bnpc(&mut data, &mut game_data, from_actor_id, id, 405);
            }
            true
        }
        "!mount" => {
            if let Some((_, mount)) = chat_message.split_once(' ') {
                let mount_id = match mount.parse::<u16>() {
                    Ok(id) => id,
                    Err(_) => {
                        let mut gamedata = game_data.lock();
                        gamedata
                            .get_mount_id_from_name(mount.to_string())
                            .unwrap_or(1) // Fallback to a company chocobo otherwise
                    }
                };

                execute_action(
                    network.clone(),
                    data.clone(),
                    game_data.clone(),
                    lua.clone(),
                    from_id,
                    from_actor_id,
                    ActionRequest {
                        action_id: mount_id as u32,
                        action_type: ActionType::Mount,
                        ..Default::default()
                    },
                );
            }

            true
        }
        "!ai_disable" => {
            let mut data = data.lock();
            if let Some(instance) = data.find_actor_instance_mut(from_actor_id) {
                instance.enemy_ai_disabled = true;
            }

            // If it's truly not existent...
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
                ServerNoticeMessage {
                    message: "A.I. disabled...".to_string(),
                    ..Default::default()
                },
            ));

            let mut network = network.lock();
            network.send_to(
                from_id,
                FromServer::PacketSegment(ipc, from_actor_id),
                DestinationNetwork::ZoneClients,
            );

            true
        }
        "!shortcut" => {
            let mut data = data.lock();
            if let Some((_, id)) = chat_message.split_once(' ') {
                let shortcut_poprange_id;
                {
                    let Some(instance) = data.find_actor_instance(from_actor_id) else {
                        return true;
                    };

                    let Some(director) = &instance.directors.first() else {
                        return true;
                    };

                    shortcut_poprange_id =
                        director.get_debug_shortcut(id.parse().unwrap_or_default());
                }

                let mut network = network.lock();
                let mut game_data = game_data.lock();
                // None here means we don't want them to change from their current instance.
                change_zone_warp_to_pop_range(
                    &mut data,
                    &mut network,
                    &mut game_data,
                    None,
                    shortcut_poprange_id,
                    from_actor_id,
                    from_id,
                    WarpType::Normal,
                    0,
                );
            }

            true
        }
        "!strikingdummy" => {
            let mut data = data.lock();
            let mut game_data = game_data.lock();
            spawn_custom_bnpc(&mut data, &mut game_data, from_actor_id, 11744, 541);

            true
        }
        "!fate" => {
            let mut data = data.lock();
            if let Some((_, id)) = chat_message.split_once(' ')
                && let Some(instance) = data.find_actor_instance_mut(from_actor_id)
            {
                // TODO: remove oldest fate as to avoid the maximum limit

                let mut game_data = game_data.lock();
                instance
                    .fates
                    .push(FateInstance::new(id.parse().unwrap(), &mut game_data));
                let mut network = network.lock();
                let fate = instance.fates.last().unwrap().clone();
                inform_fate_spawn_globally(instance, &mut network, &fate);
            }

            true
        }
        "!mapeffect" => {
            let parts: Vec<&str> = chat_message.split(' ').collect();

            let mut data = data.lock();
            if let Some(instance) = data.find_actor_instance_mut(from_actor_id) {
                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::DirectorMapEffect {
                    handler_id: instance.directors.first().unwrap().id,
                    state: parts.get(1).cloned().unwrap_or_default().parse().unwrap(),
                    timeline_id: parts.get(2).cloned().unwrap_or_default().parse().unwrap(),
                    index: parts.get(3).cloned().unwrap_or_default().parse().unwrap(),
                });
                let mut network = network.lock();
                network.send_to(
                    from_id,
                    FromServer::PacketSegment(ipc, from_actor_id),
                    DestinationNetwork::ZoneClients,
                );
            }

            true
        }
        "!ofbg" => {
            let parts: Vec<&str> = chat_message.split(' ').collect();

            let mut data = data.lock();
            let mut network = network.lock();
            if let Some(instance) = data.find_actor_instance_mut(from_actor_id) {
                // TODO: Should we reject doing this if it's attempted from other types of content?
                // Ocean fishing uses "festival" ids of 101 & 102, set when entering the zone, but it's more convenient to set it in this command for the time being
                network.send_to(
                    from_id,
                    FromServer::ActorControlSelf(ActorControlCategory::SetFestival {
                        festival1: 101,
                        festival2: 102,
                        festival3: 0,
                        festival4: 0,
                    }),
                    DestinationNetwork::ZoneClients,
                );
                // The director sends this with a background arg and a "phase" arg when the scenery needs to change. See the IKDSpot sheet for arg1 values (the row number should be increased by 1, so Kugane Coast would be 10, not 9).
                network.send_to(
                    from_id,
                    FromServer::ActorControlSelf(ActorControlCategory::DirectorEvent {
                        handler_id: instance.directors.first().unwrap().id,
                        event: DirectorEvent::Unknown {
                            id: 2,
                            arg1: parts
                                .get(1)
                                .cloned()
                                .unwrap_or_default()
                                .parse()
                                .unwrap_or_default(),
                            arg2: parts
                                .get(2)
                                .cloned()
                                .unwrap_or_default()
                                .parse()
                                .unwrap_or_default(),
                            arg3: 0,
                            arg4: 0,
                        },
                    }),
                    DestinationNetwork::ZoneClients,
                );
            }

            true
        }
        _ => false,
    }
}
