use std::sync::Arc;

use bstr::BString;
use kawari::{
    common::{DEBUG_COMMAND_TRIGGER, ObjectId},
    ipc::zone::{
        ActionKind, ActionRequest, BattleNpcSubKind, CharacterDataFlag, CommonSpawn, ObjectKind,
        ServerNoticeMessage, ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc,
    },
};
use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, GameData, ToServer,
    lua::KawariLua,
    server::{
        WorldServer,
        action::execute_action,
        actor::NetworkedActor,
        instance::Instance,
        network::{DestinationNetwork, NetworkState},
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
    chat_message: &BString, // TODO: Replace this with an SEString
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

                let actor_id = Instance::generate_actor_id();
                let npc_spawn;
                {
                    let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                        return true;
                    };

                    let Some(actor) = instance.find_actor(from_actor_id) else {
                        return true;
                    };

                    let NetworkedActor::Player { spawn, .. } = actor else {
                        return true;
                    };

                    let model_chara;
                    {
                        let mut game_data = game_data.lock();
                        (model_chara, _, _, _, _) = game_data.find_bnpc(id).unwrap();
                    }

                    npc_spawn = SpawnNpc {
                        character_data_flags: CharacterDataFlag::HOSTILE,
                        common: CommonSpawn {
                            health_points: 1500,
                            max_health_points: 1500,
                            resource_points: 100,
                            max_resource_points: 100,
                            base_id: id,
                            name_id: 405,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            level: 1,
                            battalion: 4,
                            model_chara,
                            position: spawn.common.position,
                            ..Default::default()
                        },
                        ..Default::default()
                    };

                    instance.insert_npc(actor_id, npc_spawn.clone());
                }
            }
            true
        }
        "!spawnclone" => {
            let mut data = data.lock();

            let actor_id = Instance::generate_actor_id();
            let npc_spawn;
            {
                let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                    return true;
                };

                let Some(actor) = instance.find_actor(from_actor_id) else {
                    return true;
                };

                let NetworkedActor::Player { spawn, .. } = actor else {
                    return true;
                };

                npc_spawn = SpawnNpc {
                    common: spawn.common.clone(),
                    ..Default::default()
                };

                instance.insert_npc(actor_id, npc_spawn.clone());
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
                        action_key: mount_id as u32,
                        exec_proc: 0,
                        action_kind: ActionKind::Mount,
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
        _ => false,
    }
}
