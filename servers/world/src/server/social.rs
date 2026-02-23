use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, ToServer,
    common::PartyUpdateTargets,
    server::{DestinationNetwork, WorldServer, actor::NetworkedActor, network::NetworkState},
};
use kawari::{
    common::{INVALID_OBJECT_ID, ObjectId},
    ipc::zone::{
        OnlineStatus, OnlineStatusMask, PartyMemberEntry, PartyUpdateStatus, PlayerEntry,
        SocialListRequestType, SocialListUIFlags,
    },
};

#[derive(Clone, Debug)]
pub struct PartyMember {
    pub actor_id: ObjectId,
    pub zone_client_id: ClientId,
    pub chat_client_id: ClientId,
    pub content_id: u64,
    pub account_id: u64,
    pub world_id: u16,
    pub name: String,
}

impl Default for PartyMember {
    fn default() -> Self {
        Self {
            actor_id: INVALID_OBJECT_ID,
            zone_client_id: ClientId::default(),
            chat_client_id: ClientId::default(),
            content_id: 0,
            account_id: 0,
            world_id: 0,
            name: String::default(),
        }
    }
}

impl PartyMember {
    pub fn is_valid(&self) -> bool {
        self.actor_id != INVALID_OBJECT_ID
    }

    pub fn is_online(&self) -> bool {
        self.zone_client_id != ClientId::default() && self.chat_client_id != ClientId::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Party {
    pub members: [PartyMember; PartyMemberEntry::NUM_ENTRIES],
    leader_id: ObjectId,
    pub chatchannel_id: u32, // There's no reason to store a full u64/ChatChannel here, as it's created properly in the chat connection!
    pub stratboard_realtime_host: Option<u64>, // Only one player can send a board or host real-time sharing at a time
}

impl Party {
    pub fn get_member_count(&self) -> usize {
        self.members.iter().filter(|x| x.is_valid()).count()
    }

    pub fn get_online_member_count(&self) -> usize {
        self.members
            .iter()
            .filter(|x| x.is_valid() && x.is_online())
            .count()
    }

    pub fn remove_member(&mut self, member_to_remove: ObjectId) {
        for member in self.members.iter_mut() {
            if member.actor_id == member_to_remove {
                *member = PartyMember::default();
                break;
            }
        }
    }

    pub fn set_member_offline(&mut self, offline_member: ObjectId) {
        for member in self.members.iter_mut() {
            if member.actor_id == offline_member {
                member.zone_client_id = ClientId::default();
                member.chat_client_id = ClientId::default();
                break;
            }
        }
    }

    pub fn auto_promote_member(&mut self) -> ObjectId {
        for member in &self.members {
            if member.is_valid() && member.is_online() && member.actor_id != self.leader_id {
                self.leader_id = member.actor_id;
                break;
            }
        }

        self.leader_id
    }

    pub fn get_member_by_content_id(&self, content_id: u64) -> Option<PartyMember> {
        for member in &self.members {
            if member.content_id == content_id {
                return Some(member.clone());
            }
        }
        None
    }
    pub fn get_member_by_actor_id(&self, actor_id: ObjectId) -> Option<PartyMember> {
        for member in &self.members {
            if member.actor_id == actor_id {
                return Some(member.clone());
            }
        }
        None
    }
}

fn build_party_list(party: &Party, data: &WorldServer) -> Vec<PartyMemberEntry> {
    let mut party_list = Vec::<PartyMemberEntry>::new();

    // Online members
    for instance in &data.instances {
        for (id, actor) in &instance.actors {
            let spawn = match actor {
                NetworkedActor::Player { spawn, .. } => spawn,
                _ => continue,
            };
            for member in &party.members {
                if member.actor_id == *id {
                    party_list.push(PartyMemberEntry {
                        account_id: spawn.account_id,
                        content_id: spawn.content_id,
                        name: spawn.common.name.clone(),
                        actor_id: *id,
                        classjob_id: spawn.common.class_job,
                        classjob_level: spawn.common.level,
                        current_hp: spawn.common.hp,
                        max_hp: spawn.common.max_hp,
                        current_mp: spawn.common.mp,
                        max_mp: spawn.common.max_mp,
                        current_zone_id: instance.zone.id,
                        home_world_id: spawn.home_world_id,
                        ..Default::default()
                    });
                    break;
                }
            }
        }
    }

    // Offline members
    for member in &party.members {
        if member.is_valid() && !member.is_online() {
            party_list.push(PartyMemberEntry {
                account_id: member.account_id,
                content_id: member.content_id,
                name: member.name.clone(),
                home_world_id: member.world_id,
                actor_id: ObjectId(0), // It doesn't seem to matter, but retail sets offline members' actor ids to 0.
                ..Default::default()
            })
        }
    }

    party_list
}

/// Helper function to retrieve an actor's party when given only an actor id.
pub fn get_party_id_from_actor_id(network: &NetworkState, actor_id: ObjectId) -> Option<u64> {
    for (id, my_party) in network.parties.iter() {
        for member in &my_party.members {
            if member.actor_id == actor_id {
                return Some(*id);
            }
        }
    }
    None
}

/// Process social list and party-related messages.
pub fn handle_social_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    msg: &ToServer,
) -> bool {
    match msg {
        ToServer::InvitePlayerToParty(from_actor_id, content_id, character_name) => {
            // TODO: Return an error when the target player's already in a party or offline somehow
            let mut network = network.lock();
            let data = data.lock();

            // First pull up some info about the sender, as invite packets require it
            let Some(sender_instance) = data.find_actor_instance(*from_actor_id) else {
                tracing::error!(
                    "ToServer::InvitePlayerToParty: Unable to find the sender! What happened?"
                );
                return true;
            };

            let mut sender_name = "".to_string();
            let mut sender_account_id = 0;
            let mut sender_content_id = 0;

            for (id, actor) in &sender_instance.actors {
                if id == from_actor_id {
                    let Some(spawn) = actor.get_player_spawn() else {
                        panic!("Why are we trying to get the PlayerSpawn of an NPC?");
                    };

                    sender_name = spawn.common.name.clone();
                    sender_account_id = spawn.account_id;
                    sender_content_id = spawn.content_id;
                    break;
                }
            }

            // If the sender wasn't found in the instance we already found them to be in, reality has apparently broken
            assert!(sender_content_id != 0);

            let mut recipient_actor_id = INVALID_OBJECT_ID;

            // Second, look up the recipient by name, since that and their content id are all we're given by the sending client.
            // Since we don't implement multiple worlds, the world id isn't useful for anything here.
            'outer: for instance in &data.instances {
                for (id, actor) in &instance.actors {
                    if let NetworkedActor::Player { spawn, .. } = actor
                        && (spawn.content_id == *content_id || spawn.common.name == *character_name)
                    {
                        recipient_actor_id = *id;
                        break 'outer;
                    }
                }
            }

            let mut already_in_party = false;

            // Next, see if the recipient is already in a party, and let the sender know if they are...
            'outer: for party in network.parties.values() {
                for member in &party.members {
                    if member.actor_id == recipient_actor_id {
                        already_in_party = true;
                        break 'outer;
                    }
                }
            }

            if !already_in_party {
                // Finally, if the recipient is online, fetch their handle from the network and send them the message!
                if recipient_actor_id != INVALID_OBJECT_ID {
                    let mut to_remove = Vec::new();
                    for (id, (handle, _)) in &mut network.clients {
                        if handle.actor_id == recipient_actor_id {
                            let msg = FromServer::PartyInvite(
                                sender_account_id,
                                sender_content_id,
                                sender_name,
                            );
                            if handle.send(msg.clone()).is_err() {
                                to_remove.push(*id);
                            }
                            break;
                        }
                    }
                    network.to_remove.append(&mut to_remove);
                } else {
                    // TODO: Else, if the recipient is offline, inform the sender.
                    tracing::error!(
                        "InvitePlayerToParty: The recipient is offline! What happened?"
                    );
                }
            } else {
                let msg = FromServer::CharacterAlreadyInParty();
                network.send_to_by_actor_id(*from_actor_id, msg, DestinationNetwork::ZoneClients);
            }

            true
        }
        ToServer::InvitationResponse(
            from_id,
            from_account_id,
            from_content_id,
            from_name,
            sender_content_id,
            invite_type,
            response,
        ) => {
            let mut network = network.lock();
            let data = data.lock();

            // Look up the invite sender and tell them the response.
            let mut recipient_actor_id = INVALID_OBJECT_ID;

            // Second, look up the recipient (the original invite sender) by content id, since that is all we're given by the sending client.
            'outer: for instance in &data.instances {
                for (id, actor) in &instance.actors {
                    let Some(spawn) = actor.get_player_spawn() else {
                        continue;
                    };
                    if spawn.content_id == *sender_content_id {
                        recipient_actor_id = *id;
                        break 'outer;
                    }
                }
            }

            if recipient_actor_id != INVALID_OBJECT_ID {
                let mut to_remove = Vec::new();
                for (id, (handle, _)) in &mut network.clients {
                    // Tell the invite sender about the invite result
                    if handle.actor_id == recipient_actor_id
                        && recipient_actor_id != INVALID_OBJECT_ID
                    {
                        let msg = FromServer::InvitationResult(
                            *from_account_id,
                            *from_content_id,
                            from_name.clone(),
                            *invite_type,
                            *response,
                        );
                        if handle.send(msg.clone()).is_err() {
                            to_remove.push(*id);
                        }
                    }
                    // Tell the client who just responded to the sender's invite to wait for further instructions
                    if *id == *from_id {
                        let msg = FromServer::InvitationReplyResult(
                            *from_content_id,
                            from_name.clone(),
                            *invite_type,
                            *response,
                        );
                        if handle.send(msg.clone()).is_err() {
                            to_remove.push(*id);
                        }
                    }
                }
                network.to_remove.append(&mut to_remove);
            }

            true
        }
        ToServer::RequestSocialList(from_id, from_actor_id, from_party_id, request) => {
            let mut network = network.lock();
            let data = data.lock();
            let mut entries = vec![PlayerEntry::default(); 10];

            match &request.request_type {
                SocialListRequestType::Party => {
                    if *from_party_id != 0 {
                        let leader_actor_id = network.parties[from_party_id].leader_id;
                        let mut index: usize = 0;
                        for member in &network.parties[from_party_id].members {
                            // The internal party list can and will contain invalid entries representing empty slots, so skip them.
                            if !member.is_valid() {
                                continue;
                            }

                            if !member.is_online() {
                                entries[index].content_id = member.content_id;
                                entries[index].name = member.name.clone();
                                entries[index].current_world_id = 65535; // This doesn't seem to matter, but retail does it.
                                entries[index].ui_flags = SocialListUIFlags::ENABLE_CONTEXT_MENU;
                                entries[index].home_world_id = member.world_id;
                                index += 1;
                                continue;
                            }

                            let Some(instance) = data.find_actor_instance(member.actor_id) else {
                                // TOOD: This situation might be panic-worthy? Reality should have broken, or an invalid party member slipped past the earlier check if this trips.
                                tracing::error!(
                                    "Unable to find this actor in any instance, what happened? {} {}",
                                    member.actor_id,
                                    member.name.clone()
                                );
                                continue;
                            };

                            for (id, actor) in &instance.actors {
                                if *id == member.actor_id {
                                    let Some(spawn) = actor.get_player_spawn() else {
                                        panic!(
                                            "Why are we trying to get the PlayerSpawn of an NPC?"
                                        );
                                    };
                                    let mut online_status_mask = OnlineStatusMask::default();
                                    online_status_mask.set_status(OnlineStatus::Online);
                                    online_status_mask.set_status(OnlineStatus::PartyMember);
                                    if member.actor_id == leader_actor_id {
                                        online_status_mask.set_status(OnlineStatus::PartyLeader);
                                    }
                                    entries[index].online_status_mask = online_status_mask;
                                    entries[index].classjob_id = spawn.common.class_job;
                                    entries[index].classjob_level = spawn.common.level;
                                    entries[index].zone_id = instance.zone.id;

                                    entries[index].content_id = spawn.content_id;
                                    entries[index].home_world_id = member.world_id;
                                    entries[index].current_world_id = spawn.current_world_id;
                                    entries[index].name = spawn.common.name.clone();
                                    entries[index].ui_flags =
                                        SocialListUIFlags::ENABLE_CONTEXT_MENU;

                                    index += 1;
                                    break;
                                }
                            }
                        }
                    } else {
                        let Some(instance) = data.find_actor_instance(*from_actor_id) else {
                            return true;
                        };

                        for (id, actor) in &instance.actors {
                            if *id == *from_actor_id {
                                let Some(spawn) = actor.get_player_spawn() else {
                                    panic!("Why are we trying to get the PlayerSpawn of an NPC?");
                                };

                                // TODO: Probably start with a cached status from elsewhere?
                                let mut online_status_mask = OnlineStatusMask::default();
                                online_status_mask.set_status(OnlineStatus::Online);

                                entries[0].content_id = spawn.content_id;
                                entries[0].current_world_id = spawn.home_world_id;
                                entries[0].home_world_id = spawn.home_world_id;
                                entries[0].name = spawn.common.name.clone();
                                entries[0].ui_flags = SocialListUIFlags::ENABLE_CONTEXT_MENU;
                                entries[0].online_status_mask = online_status_mask;
                                entries[0].classjob_id = spawn.common.class_job;
                                entries[0].classjob_level = spawn.common.level;
                                entries[0].zone_id = instance.zone.id;
                                break;
                            }
                        }
                    }
                }
                _ => {
                    tracing::warn!(
                        "SocialListRequestType was {:#?}! This is not yet implemented!",
                        &request.request_type
                    );
                }
            }

            let msg = FromServer::SocialListResponse(request.request_type, request.count, entries);
            network.send_to(*from_id, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::AddPartyMember(party_id, leader_actor_id, new_member_content_id) => {
            let mut network = network.lock();
            let data = data.lock();
            let mut party_id = *party_id;

            // This client is creating a party.
            if party_id == 0 {
                // TODO: We should probably generate these differently so there are no potential collisions.
                party_id = fastrand::u64(..);
                let chatchannel_id = fastrand::u32(..);
                let party = network.parties.entry(party_id).or_default();
                party.chatchannel_id = chatchannel_id;
                party.leader_id = *leader_actor_id;
                party.members[0].actor_id = *leader_actor_id;
            }

            if let Some(party) = network.parties.get(&party_id) {
                let chatchannel_id = network.parties[&party_id].chatchannel_id;
                let mut party = party.members.clone();

                let mut party_list = Vec::<PartyMemberEntry>::new();

                let mut execute_account_id = 0;
                let mut execute_content_id = 0;
                let mut execute_name = String::default();
                let mut target_account_id = 0;
                let mut target_content_id = 0;
                let mut target_name = String::default();

                // TODO: This can probably be simplified/the logic can probably be adjusted, need to think more on this
                for instance in &data.instances {
                    for (id, actor) in &instance.actors {
                        let Some(spawn) = actor.get_player_spawn() else {
                            continue;
                        };

                        if spawn.content_id == *new_member_content_id {
                            // Find the first open member slot.
                            let Some(free_index) =
                                party.iter().position(|x| x.actor_id == INVALID_OBJECT_ID)
                            else {
                                // TODO: See if we can gracefully exit from here without a panic
                                panic!(
                                    "Tried to add a party member to a full party! What happened? {party:#?}"
                                );
                            };
                            party[free_index].actor_id = *id;
                            target_account_id = spawn.account_id;
                            target_content_id = spawn.content_id;
                            target_name = spawn.common.name.clone();
                        }

                        if *id == *leader_actor_id {
                            execute_account_id = spawn.account_id;
                            execute_content_id = spawn.content_id;
                            execute_name = spawn.common.name.clone();
                        }

                        for member in &mut party {
                            if member.actor_id == *id {
                                member.account_id = spawn.account_id;
                                member.content_id = spawn.content_id;
                                member.name = spawn.common.name.clone();

                                party_list.push(PartyMemberEntry {
                                    account_id: spawn.account_id,
                                    content_id: spawn.content_id,
                                    name: spawn.common.name.clone(),
                                    actor_id: *id,
                                    classjob_id: spawn.common.class_job,
                                    classjob_level: spawn.common.level,
                                    current_hp: spawn.common.hp,
                                    max_hp: spawn.common.max_hp,
                                    current_mp: spawn.common.mp,
                                    max_mp: spawn.common.max_mp,
                                    current_zone_id: instance.zone.id,
                                    home_world_id: spawn.home_world_id,
                                    ..Default::default()
                                });
                                break;
                            }
                        }
                    }
                }

                assert!(
                    !party_list.is_empty() && party_list.len() <= PartyMemberEntry::NUM_ENTRIES
                );

                let update_status = PartyUpdateStatus::JoinParty;

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id,
                        execute_content_id,
                        execute_name: execute_name.clone(),
                        target_account_id,
                        target_content_id,
                        target_name: target_name.clone(),
                    },
                    update_status,
                    Some((
                        party_id,
                        chatchannel_id,
                        *leader_actor_id,
                        party_list.clone(),
                    )),
                );

                let mut to_remove = Vec::new();

                // Next, tell everyone in the party someone joined (including the joining player themself).
                // Also cache their client ids to speed up sending future replies.
                for (id, (handle, _)) in &mut network.clients {
                    for member in &mut party {
                        if member.actor_id == handle.actor_id {
                            member.zone_client_id = *id;
                            if handle.send(msg.clone()).is_err() {
                                to_remove.push(*id);
                            }
                        }
                    }
                }

                let msg = FromServer::SetPartyChatChannel(chatchannel_id);

                // Finally, tell their chat connections they're now in a party.
                // Also cache their client ids to speed up sending future replies.
                for (id, (handle, _)) in &mut network.chat_clients {
                    for member in &mut party {
                        if member.actor_id == handle.actor_id {
                            member.chat_client_id = *id;
                            if handle.send(msg.clone()).is_err() {
                                to_remove.push(*id);
                            }
                        }
                    }
                }

                network.to_remove.append(&mut to_remove);
                network.parties.get_mut(&party_id).unwrap().members = party; // Now we can give the clone back after all that nonsense
            } else {
                tracing::error!("AddPartyMember: Party id wasn't in the hashmap! What happened?");
            }

            true
        }
        ToServer::PartyMemberChangedAreas(
            party_id,
            execute_account_id,
            execute_content_id,
            execute_name,
        ) => {
            let mut network = network.lock();
            let data = data.lock();
            let party = network.parties.get_mut(party_id).unwrap();

            let party_list = build_party_list(party, &data);

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    ..Default::default()
                },
                PartyUpdateStatus::MemberChangedZones,
                Some((*party_id, party.chatchannel_id, party.leader_id, party_list)),
            );

            // Finally, tell everyone in the party about the update.
            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::PartyChangeLeader(
            party_id,
            execute_account_id,
            execute_content_id,
            execute_name,
            target_content_id,
            target_name,
        ) => {
            let mut network = network.lock();

            if !network.parties.contains_key(party_id) {
                panic!("Why are we trying to do party operations on an invalid party?");
            }

            let data = data.lock();
            let target_account_id;
            {
                let party = &mut network.parties.get_mut(party_id).unwrap();
                let Some(member) = party.get_member_by_content_id(*target_content_id) else {
                    return true;
                };
                party.leader_id = member.actor_id;
                target_account_id = member.account_id;
            }

            let party = &network.parties.get(party_id).unwrap();

            let party_list = build_party_list(party, &data);

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    target_account_id,
                    target_content_id: *target_content_id,
                    target_name: target_name.clone(),
                },
                PartyUpdateStatus::PromoteLeader,
                Some((*party_id, party.chatchannel_id, party.leader_id, party_list)),
            );

            // Finally, tell everyone in the party about the update.
            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::PartyMemberLeft(
            party_id,
            execute_account_id,
            execute_content_id,
            execute_actor_id,
            execute_name,
        ) => {
            let mut network = network.lock();
            let data = data.lock();
            let party_list;
            let leaving_zone_client_id;
            let leaving_chat_client_id;
            let chatchannel_id;
            let mut leader_id;
            let member_count;
            {
                let Some(party) = network.parties.get_mut(party_id) else {
                    return true;
                };
                chatchannel_id = party.chatchannel_id;

                // Construct the party list we're sending back to the clients in this party.
                leaving_zone_client_id = party
                    .get_member_by_actor_id(*execute_actor_id)
                    .unwrap()
                    .zone_client_id;
                leaving_chat_client_id = party
                    .get_member_by_actor_id(*execute_actor_id)
                    .unwrap()
                    .chat_client_id;

                party.remove_member(*execute_actor_id);
                member_count = party.get_member_count();
                leader_id = party.leader_id;

                // If the leader left the party, and there are still enough members, auto-promote the next available player
                if *execute_actor_id == party.leader_id && member_count >= 2 {
                    leader_id = party.auto_promote_member();
                }

                party_list = build_party_list(party, &data);
            }

            let update_status;
            let party_info;

            if member_count < 2 {
                update_status = PartyUpdateStatus::DisbandingParty;
                party_info = None;
            } else {
                update_status = PartyUpdateStatus::MemberLeftParty;
                party_info = Some((*party_id, chatchannel_id, leader_id, party_list));
            }

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    ..Default::default()
                },
                update_status,
                party_info,
            );

            let leaver_msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    ..Default::default()
                },
                update_status,
                None,
            );

            // Tell everyone in the party about the update.
            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);

            // Tell the leaver that they're not in the party anymore.
            network.send_to(
                leaving_zone_client_id,
                leaver_msg,
                DestinationNetwork::ZoneClients,
            );
            network.send_to(
                leaving_chat_client_id,
                FromServer::SetPartyChatChannel(0),
                DestinationNetwork::ChatClients,
            );

            // Clean up the party on our side, if necessary.
            if member_count < 2 {
                // Tell their chat connections they're no longer in a party.
                network.send_to_party(
                    *party_id,
                    None,
                    FromServer::SetPartyChatChannel(0),
                    DestinationNetwork::ChatClients,
                );
                network.parties.remove(party_id);
            }

            true
        }
        ToServer::PartyDisband(party_id, execute_account_id, execute_content_id, execute_name) => {
            let mut network = network.lock();

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    ..Default::default()
                },
                PartyUpdateStatus::DisbandingParty,
                None,
            );

            // Finally, tell everyone in the party about the update.
            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);

            // Tell their chat connections they're no longer in a party.
            network.send_to_party(
                *party_id,
                None,
                FromServer::SetPartyChatChannel(0),
                DestinationNetwork::ChatClients,
            );

            // We don't need to keep track of this party anymore.
            network.parties.remove(party_id);

            true
        }
        ToServer::PartyMemberKick(
            party_id,
            execute_account_id,
            execute_content_id,
            execute_name,
            target_content_id,
            target_name,
        ) => {
            let mut network = network.lock();
            let data = data.lock();
            let party = network.parties.get_mut(party_id).unwrap();

            let Some(member) = party.get_member_by_content_id(*target_content_id) else {
                return true;
            };
            party.remove_member(member.actor_id);

            // Construct the party list we're sending back to the clients in this party.
            let party_list = build_party_list(party, &data);

            let update_status;
            let party_info;
            let member_count = party.get_member_count();
            if member_count < 2 {
                update_status = PartyUpdateStatus::DisbandingParty;
                party_info = None;
            } else {
                update_status = PartyUpdateStatus::MemberKicked;
                party_info = Some((*party_id, party.chatchannel_id, party.leader_id, party_list));
            }

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    target_account_id: member.account_id,
                    target_content_id: *target_content_id,
                    target_name: target_name.clone(),
                },
                update_status,
                party_info,
            );

            let leaver_msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    ..Default::default()
                },
                update_status,
                None,
            );

            // Tell everyone in the party about the update.
            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);

            // Tell the leaver that they're not in the party anymore, including their chat connection.
            network.send_to(
                member.zone_client_id,
                leaver_msg,
                DestinationNetwork::ZoneClients,
            );
            network.send_to(
                member.chat_client_id,
                FromServer::SetPartyChatChannel(0),
                DestinationNetwork::ChatClients,
            );

            // Clean up the party on our side, if necessary.
            if member_count < 2 {
                // Tell their chat connections they're no longer in a party.
                network.send_to_party(
                    *party_id,
                    None,
                    FromServer::SetPartyChatChannel(0),
                    DestinationNetwork::ChatClients,
                );
                network.parties.remove(party_id);
            }

            true
        }
        ToServer::PartyMemberOffline(
            party_id,
            execute_account_id,
            execute_content_id,
            from_actor_id,
            execute_name,
        ) => {
            let mut network = network.lock();
            let data = data.lock();

            if !network.parties.contains_key(party_id) {
                tracing::error!(
                    "PartyMemberOffline: We were given an invalid party id {}. What happened?",
                    party_id
                );
                return true;
            }

            let party = &mut network.parties.get_mut(party_id).unwrap();
            party.set_member_offline(*from_actor_id);

            if party.get_online_member_count() > 0 {
                let party_list = build_party_list(party, &data);

                // Auto-promote the first available player to leader if the previous leader went offline.
                // In this situation: retail uses PartyLeaderWentOffline as the update status, followed by sending another full MemberWentOffline update,
                // but this is very inefficient and wasteful, so we will not do that (unless we have good reason to).
                // The client still accepts a leader change during MemberWentOffline.
                if party.leader_id == *from_actor_id {
                    party.leader_id = party.auto_promote_member();
                }

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id: *execute_account_id,
                        execute_content_id: *execute_content_id,
                        execute_name: execute_name.clone(),
                        ..Default::default()
                    },
                    PartyUpdateStatus::MemberWentOffline,
                    Some((*party_id, party.chatchannel_id, party.leader_id, party_list)),
                );

                network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
            } else {
                // If nobody in the party is online, disband it.
                // Retail keeps it around for ~2 hours or so if everyone is offline, but there's no point doing that.
                network.parties.remove(party_id);
            }

            true
        }
        ToServer::PartyMemberReturned(execute_actor_id) => {
            let mut network = network.lock();
            let data = data.lock();

            let mut member = PartyMember::default();
            let mut party_id = 0;
            let mut party = Party::default();

            'outer: for (id, my_party) in &mut network.parties.iter() {
                for my_member in &my_party.members {
                    if my_member.actor_id == *execute_actor_id {
                        member = my_member.clone();
                        party_id = *id;
                        party = my_party.clone();
                        break 'outer;
                    }
                }
            }

            let party_list = build_party_list(&party, &data);
            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: member.account_id,
                    execute_content_id: member.content_id,
                    execute_name: member.name.clone(),
                    ..Default::default()
                },
                PartyUpdateStatus::MemberReturned,
                Some((party_id, party.chatchannel_id, party.leader_id, party_list)),
            );

            network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);

            true
        }
        ToServer::ShareStrategyBoard(
            from_actor_id,
            from_content_id,
            party_id,
            client_content_id,
            board_data,
        ) => {
            let mut network = network.lock();

            // TODO: Once we understand the board data, should we perform validation/sanitization to ensure it's not malicious in some way?

            // client_content_id is what the client passed to us. If it's 0, they're either starting a regular share, or beginning a real-time share, which is followed up by a second share moments later that has the content id set. If it's set, the board is not sent to the party again, and we should be expecting to see real-time update opcodes.
            if *client_content_id == 0 {
                // Inform the party about the board.
                let msg = FromServer::StrategyBoardShared(*from_content_id, board_data.clone());
                network.send_to_party(
                    *party_id,
                    Some(*from_actor_id),
                    msg,
                    DestinationNetwork::ZoneClients,
                );
            }

            true
        }
        ToServer::StrategyBoardReceived(party_id, from_content_id, dest_content_id) => {
            let mut network = network.lock();

            // Only send the ack to the board sharer if we're doing the first board share in a sequence (in real-time sharing, the client sends two at the beginning for unknown reasons).
            if let Some(party) = network.parties.get(party_id)
                && let None = party.stratboard_realtime_host
            {
                let msg = FromServer::StrategyBoardSharedAck(*from_content_id);

                // Tell the board sender a party member received it.
                let mut dest_actor_id = INVALID_OBJECT_ID;
                for my_member in &network.parties[party_id].members {
                    if my_member.content_id == *dest_content_id {
                        dest_actor_id = my_member.actor_id;
                        break;
                    }
                }

                network.send_to_by_actor_id(dest_actor_id, msg, DestinationNetwork::ZoneClients);
            }

            true
        }
        ToServer::StrategyBoardRealtimeUpdate(
            from_actor_id,
            from_content_id,
            party_id,
            board_update,
        ) => {
            let mut network = network.lock();
            let msg = FromServer::StrategyBoardRealtimeUpdate(board_update.clone());

            // Until we get realtime updates for the first time in a sharing session, it doesn't matter for our records who the sender is.
            // TODO: We should probably make better use of the content id field in the initial share opcode, but this works just as well for now.
            network.parties.entry(*party_id).and_modify(|v| {
                if v.stratboard_realtime_host.is_none() {
                    v.stratboard_realtime_host = Some(*from_content_id)
                }
            });

            // Tell everyone except the board sharer about the updates.
            network.send_to_party(
                *party_id,
                Some(*from_actor_id),
                msg,
                DestinationNetwork::ZoneClients,
            );

            true
        }
        ToServer::StrategyBoardRealtimeFinished(party_id) => {
            let mut network = network.lock();
            let msg = FromServer::StrategyBoardRealtimeFinished();

            // Tell everyone about the session ending, and reset state so further real-time sessions can be initiated.
            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
            network
                .parties
                .entry(*party_id)
                .and_modify(|v| v.stratboard_realtime_host = None);

            true
        }
        ToServer::ApplyWaymarkPreset(from_id, party_id, waymark_preset) => {
            let mut network = network.lock();
            let msg = FromServer::WaymarkPreset(waymark_preset.clone());

            if *party_id != 0 {
                network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
            } else {
                network.send_to_by_actor_id(*from_id, msg, DestinationNetwork::ZoneClients);
            }

            true
        }
        ToServer::StartCountdown(
            party_id,
            from_id,
            account_id,
            content_id,
            starter_name,
            starter_actor_id,
            duration,
        ) => {
            let mut network = network.lock();
            let msg = FromServer::Countdown(
                *account_id,
                *content_id,
                starter_name.clone(),
                *starter_actor_id,
                *duration,
            );

            if *party_id != 0 {
                network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
            } else {
                network.send_to_by_actor_id(*from_id, msg, DestinationNetwork::ZoneClients);
            }

            true
        }
        _ => false,
    }
}
