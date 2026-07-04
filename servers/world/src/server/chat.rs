use std::sync::Arc;

use bstr::BString;
use kawari::{
    common::{DEBUG_COMMAND_TRIGGER, ObjectId, STRIKING_DUMMY_NAME_ID},
    ipc::zone::{
        ActionRequest, ActionType, BattleNpcSubKind, CharacterDataFlag, CommonSpawn, ObjectKind,
        ServerNoticeMessage, ServerZoneIpcData, ServerZoneIpcSegment, SpawnNpc, WarpType,
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
            let data = data.lock();

            let mut network = network.lock();

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
        "!dummy" => {
            let mut data = data.lock();
            let actor_id = Instance::generate_actor_id();

            let npc_spawn = {
                let Some(instance) = data.find_actor_instance_mut(from_actor_id) else {
                    return true;
                };

                let Some(actor) = instance.find_actor(from_actor_id) else {
                    return true;
                };

                let NetworkedActor::Player { spawn, .. } = actor else {
                    return true;
                };

                let mut position = spawn.common.position;
                position.0.x += 3.0;

                let mut npc_spawn = if let Some(template) = instance
                    .zone
                    .find_battle_npc_template(11744, STRIKING_DUMMY_NAME_ID)
                {
                    template
                } else {
                    let (model_chara, battalion, customize, rank, equip) = {
                        let mut game_data = game_data.lock();
                        game_data.find_bnpc(11744).unwrap_or_default()
                    };

                    SpawnNpc {
                        character_data_flags: CharacterDataFlag::HOSTILE,
                        character_data_icon: rank,
                        common: CommonSpawn {
                            health_points: 999_999,
                            max_health_points: 999_999,
                            resource_points: 0,
                            max_resource_points: 0,
                            base_id: 11744,
                            name_id: STRIKING_DUMMY_NAME_ID,
                            object_kind: ObjectKind::BattleNpc(BattleNpcSubKind::Enemy),
                            level: 1,
                            battalion,
                            layout_id: actor_id.0,
                            model_chara,
                            position,
                            rotation: spawn.common.rotation,
                            look: customize,
                            ..{
                                let mut game_data = game_data.lock();
                                game_data.get_npc_equip(equip as u32).unwrap_or_default()
                            }
                        },
                        ..Default::default()
                    }
                };

                npc_spawn.character_data_flags = CharacterDataFlag::HOSTILE;
                npc_spawn.common.health_points = 999_999;
                npc_spawn.common.max_health_points = 999_999;
                npc_spawn.common.resource_points = 0;
                npc_spawn.common.max_resource_points = 0;
                // BNpcBase 11744 is the explorer-mode striking dummy, which (unlike the legacy
                // 901 dummy) carries a valid ModelChara so it actually renders and can be targeted.
                npc_spawn.common.base_id = 11744;
                npc_spawn.common.name_id = STRIKING_DUMMY_NAME_ID;
                npc_spawn.common.object_kind = ObjectKind::BattleNpc(BattleNpcSubKind::Enemy);
                npc_spawn.common.level = 1;
                npc_spawn.common.layout_id = actor_id.0;
                npc_spawn.common.position = position;
                npc_spawn.common.rotation = spawn.common.rotation;

                instance.insert_npc(actor_id, npc_spawn.clone());
                npc_spawn
            };

            let Some(instance) = data.find_actor_instance(from_actor_id) else {
                return true;
            };
            let Some(dummy_actor) = instance.find_actor(actor_id) else {
                return true;
            };

            let mut network = network.lock();
            for (_, (handle, state)) in &mut network.clients {
                let Some(client_actor) = instance.find_actor(handle.actor_id) else {
                    continue;
                };

                if !client_actor.in_range_of(dummy_actor) {
                    continue;
                }

                if let Some(msg) =
                    NetworkState::spawn_existing_actor_message(state, actor_id, dummy_actor)
                {
                    let _ = handle.send(msg);
                }
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ServerNoticeMessage(
                ServerNoticeMessage {
                    message: format!(
                        "Spawned striking dummy {} at ({:.1}, {:.1}, {:.1})",
                        actor_id.0,
                        npc_spawn.common.position.0.x,
                        npc_spawn.common.position.0.y,
                        npc_spawn.common.position.0.z
                    ),
                    ..Default::default()
                },
            ));

            network.send_to(
                from_id,
                FromServer::PacketSegment(ipc, from_actor_id),
                DestinationNetwork::ZoneClients,
            );

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

                    let Some(director) = &instance.director else {
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
                    0,
                    0,
                );
            }

            true
        }
        _ => false,
    }
}
