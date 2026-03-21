use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::{
    ClientId, FromServer, ToServer,
    common::PartyUpdateTargets,
    server::{
        DestinationNetwork, WorldServer, actor::NetworkedActor, network::NetworkState,
        set_character_mode,
    },
};
use kawari::{
    common::{CharacterMode, ObjectId, ObjectTypeId, Position},
    ipc::zone::{
        ActorControlCategory, PartyMemberEntry, PartyMemberPositions, PartyUpdateStatus,
        ReadyCheckReply, ServerZoneIpcData, ServerZoneIpcSegment, WaymarkPlacementMode,
        WaymarkPosition, WaymarkPositions, WaymarkPreset,
    },
};

#[derive(Clone, Debug, Default)]
pub struct PartyMember {
    pub actor_id: ObjectId,
    pub zone_client_id: ClientId,
    pub chat_client_id: ClientId,
    pub content_id: u64,
    pub account_id: u64,
    pub world_id: u16,
    pub name: String,
    pub position: Position,
    /// If this party member is riding pillion, we need to store who the driver is.
    pub pillion_driver_id: ObjectId,
    /// If a ready check is underway, we need to store this member's response.
    pub ready_check_reply: ReadyCheckReply,
}

impl PartyMember {
    // TODO: See if this is still actually needed since we should only be storing active party members now, not any with INVALID_OBJECT_ID.
    pub fn is_valid(&self) -> bool {
        self.actor_id.is_valid()
    }

    pub fn is_online(&self) -> bool {
        self.zone_client_id != ClientId::default() && self.chat_client_id != ClientId::default()
    }
}

// The current amount of target signs available for the player's party to use.
pub const NUM_TARGET_SIGNS: usize = 17;

#[derive(Clone, Debug, Default)]
pub struct Party {
    pub members: Vec<PartyMember>,
    pub leader_id: ObjectId,
    pub chatchannel_id: u32, // There's no reason to store a full u64/ChatChannel here, as it's created properly in the chat connection!
    pub stratboard_realtime_host: Option<u64>, // Only one player can send a board or host real-time sharing at a time
    pub target_signs: [ObjectTypeId; NUM_TARGET_SIGNS], // NOTE: We deviate from retail here, which seems to have per-instance lists of marked targets, and instead just have one per party for simplicity.
    pub waymarks: HashMap<i32, WaymarkPositions>, // TODO: If/when we ever get unique instance identifiers, use those instead of the zone id.
    pub readycheck_host: Option<ObjectId>, // Only one ready check can be undertaken at a time.
}

impl Party {
    pub fn get_member_count(&self) -> usize {
        // TODO: As noted above, we can probably just use .len() now, but in the interim I'll keep it as it was.
        self.members.iter().filter(|x| x.is_valid()).count()
    }

    pub fn get_online_member_count(&self) -> usize {
        self.members
            .iter()
            .filter(|x| x.is_valid() && x.is_online())
            .count()
    }

    pub fn remove_member(&mut self, member_to_remove: ObjectId) {
        self.members.retain(|x| x.actor_id != member_to_remove);
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

    pub fn get_member_by_actor_id_mut(&mut self, actor_id: ObjectId) -> Option<&mut PartyMember> {
        for (index, member) in self.members.iter().enumerate() {
            if member.actor_id == actor_id {
                return Some(&mut self.members[index]);
            }
        }
        None
    }
}

fn build_party_list(party: &Party, data: &WorldServer) -> Vec<PartyMemberEntry> {
    let mut party_list = Vec::<PartyMemberEntry>::new();

    // NOTE: The client expects active party members to be at the beginning of the list, and for invalid party members (read: they have INVALID_OBJECT_ID as their actor id) to be at the end of the list! Failure to do this can cause very strange behaviour!

    // Online members
    for member in &party.members {
        if member.is_online() {
            'instance: for instance in &data.instances {
                for (id, actor) in &instance.actors {
                    let spawn = match actor {
                        NetworkedActor::Player { spawn, .. } => spawn,
                        _ => continue,
                    };

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
                            sync_positions: 1,
                            unk2: 1,
                            ..Default::default()
                        });
                        break 'instance; // break out of the instance loop to move on to the next member
                    }
                }
            }
        } else {
            // Offline members
            party_list.push(PartyMemberEntry {
                account_id: member.account_id,
                content_id: member.content_id,
                name: member.name.clone(),
                home_world_id: member.world_id,
                actor_id: ObjectId(0), // It doesn't seem to matter, but retail sets offline members' actor ids to 0. This is not the same as an invalid member with INVALID_OBJECT_ID!
                ..Default::default()
            });
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

/// Helper function to send the party's currently marked targets to a specific actor that changed areas or returned from being offline.
fn send_party_target_signs(network: &mut NetworkState, party_id: u64, execute_actor_id: ObjectId) {
    let target_signs = match network.parties.get(&party_id) {
        Some(p) => p.target_signs,
        None => {
            tracing::error!(
                "send_party_target_signs was called with an invalid party id {party_id}! What happened? We won't be sending the markers for this player."
            );
            return;
        }
    };

    for (sign_id, target_to_mark) in target_signs.iter().enumerate() {
        // Don't need to send info for signs that don't have a valid target.
        if target_to_mark.object_id != ObjectId::default() {
            // When informing a client about existing markers, the server sets the sender as the blank actor id instead of the original player that marked the target.
            let msg =
                FromServer::TargetSignToggled(sign_id as u32, ObjectId::default(), *target_to_mark);
            network.send_to_by_actor_id(execute_actor_id, msg, DestinationNetwork::ZoneClients);
        }
    }
}

/// Helper function to send existing waymarks to party members who changed areas or returned from being offline.
fn send_party_waymarks(
    network: &mut NetworkState,
    party_id: u64,
    execute_actor_id: ObjectId,
    zone_id: i32,
) {
    let position_data = network
        .parties
        .get_mut(&party_id)
        .unwrap()
        .waymarks
        .entry(zone_id)
        .or_default();
    let preset = WaymarkPreset::from(*position_data);
    let msg = FromServer::WaymarkPreset(preset, zone_id);
    network.send_to_by_actor_id(execute_actor_id, msg, DestinationNetwork::ZoneClients);
}

/// Helper function to update a party's zone's entire waymarks list.
pub fn update_party_waymarks(
    network: &mut NetworkState,
    data: &WorldServer,
    execute_actor_id: ObjectId,
    waymark_preset: WaymarkPreset,
) {
    // While we could easily pass in the zone id and party id via parameters for function calls in this same source file, client triggers also call into this function, so for the sake of simplicity, we just do the lookups here instead.
    if let Some(instance) = data.find_actor_instance(execute_actor_id) {
        let zone_id = instance.zone.id as i32;
        let msg = FromServer::WaymarkPreset(waymark_preset, zone_id);
        if let Some(party_id) = get_party_id_from_actor_id(network, execute_actor_id) {
            network.send_to_party(party_id, None, msg, DestinationNetwork::ZoneClients);

            let party = network.parties.get_mut(&party_id).unwrap();
            party
                .waymarks
                .entry(zone_id)
                .and_modify(|p| *p = waymark_preset.into())
                .or_insert(waymark_preset.into());
        } else {
            network.send_to_by_actor_id(execute_actor_id, msg, DestinationNetwork::ZoneClients);
        }
    } else {
        tracing::error!(
            "update_party_waymark: Unable to find {}'s instance, what happened?",
            execute_actor_id
        );
    }
}

/// Helper function for CTs ClearWaymark and PlaceWaymark.
pub fn update_party_waymark(
    network: &mut NetworkState,
    data: &WorldServer,
    from_actor_id: ObjectId,
    waymark_id: u32,
    waymark: Option<WaymarkPosition>,
) {
    let zone_id;
    if let Some(instance) = data.find_actor_instance(from_actor_id) {
        zone_id = instance.zone.id as i32;
    } else {
        tracing::error!(
            "update_party_waymark: Unable to find {}'s instance, what happened?",
            from_actor_id
        );
        return;
    }

    // Next, update the party's waymark data, if relevant.
    if let Some(party_id) = get_party_id_from_actor_id(network, from_actor_id) {
        let party = network.parties.get_mut(&party_id).unwrap();
        let waymarks = party.waymarks.entry(zone_id).or_default();
        waymarks[waymark_id as usize] = waymark;
    }

    let placement_mode;
    let position_data;
    match waymark {
        Some(pos_data) => {
            placement_mode = WaymarkPlacementMode::Placed;
            position_data = pos_data;
        }
        None => {
            placement_mode = WaymarkPlacementMode::Removed;
            position_data = WaymarkPosition::default();
        }
    }

    let msg = FromServer::WaymarkUpdated(waymark_id as u8, placement_mode, position_data, zone_id);
    network.send_to_party_or_self(from_actor_id, msg);
}

fn get_pillion_driver_position(party: &Party, index: usize) -> Option<Position> {
    for member in &party.members {
        if member.actor_id == party.members[index].pillion_driver_id {
            return Some(member.position);
        }
    }

    None
}

/// Helper function used to send periodic updates for where party members are.
/// NOTE: This affects things like player dots on the minimap, as well as riding pillion on mounts. Adjust at your own risk!
pub fn send_party_positions(network: &mut NetworkState) {
    // TODO: Can this outer loop be done without cloning?
    for (party_id, party) in &network.parties.clone() {
        if party.get_online_member_count() < 1 {
            tracing::error!(
                "Encountered a party with zero online members, id {party_id}. How did this happen, when we auto-disband such parties?"
            );
            continue;
        }

        let mut member_positions = PartyMemberPositions::default();

        // TODO: Can this also be done without cloning?
        for (index, member) in party.members.clone().iter().enumerate() {
            if member.is_online() {
                member_positions.positions[index].valid = 1;
                // If the party member is riding pillion, their position is broadcasted as the *driver*'s! Otherwise just use that member's current known position.
                if let Some(driver_position) = get_pillion_driver_position(party, index) {
                    member_positions.positions[index].pos = driver_position;
                } else {
                    member_positions.positions[index].pos = member.position;
                }
            }
        }

        let msg = FromServer::PartyMemberPositionsUpdate(member_positions);
        network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
    }
}

/// Helper function to update our copy of the party member's position.
pub fn update_party_position(
    network: &mut NetworkState,
    data: &mut WorldServer,
    party_id: u64,
    actor_id: ObjectId,
    position: Position,
) {
    if let Some(party) = network.parties.get_mut(&party_id)
        && let Some(my_member) = party.get_member_by_actor_id_mut(actor_id)
    {
        my_member.position = position;

        for member in &mut party.members {
            // If this member is our passenger
            if member.pillion_driver_id == actor_id {
                let Some(instance) = data.find_actor_instance_mut(member.actor_id) else {
                    return;
                };

                // Get their position and update it to ours.
                let Some(NetworkedActor::Player { spawn, .. }) =
                    instance.find_actor_mut(member.actor_id)
                else {
                    return;
                };

                member.position = position;
                spawn.common.position = position;
            }
        }
    }
}

/// Process social list and party-related messages.
pub fn handle_social_messages(
    data: Arc<Mutex<WorldServer>>,
    network: Arc<Mutex<NetworkState>>,
    msg: &ToServer,
) -> bool {
    match msg {
        ToServer::InvitePlayerToFriendList(from_actor_id, content_id, character_name) => {
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

            let mut recipient_actor_id = ObjectId::default();

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

            assert!(recipient_actor_id != ObjectId::default());

            let msg = FromServer::FriendInvite(sender_account_id, sender_content_id, sender_name);
            network.send_to_by_actor_id(recipient_actor_id, msg, DestinationNetwork::ZoneClients);

            true
        }
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

            let mut recipient_actor_id = ObjectId::default();

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
                if recipient_actor_id.is_valid() {
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
            let mut recipient_actor_id = ObjectId::default();

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

            if recipient_actor_id.is_valid() {
                let mut to_remove = Vec::new();
                for (id, (handle, _)) in &mut network.clients {
                    // Tell the invite sender about the invite result
                    if handle.actor_id == recipient_actor_id && recipient_actor_id.is_valid() {
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
        ToServer::AddPartyMember(party_id, leader_actor_id, new_member_content_id) => {
            let mut network = network.lock();
            let data = data.lock();
            let mut party_id = *party_id;

            // Grab the leader's info before doing anything else.
            let Some(leader_instance) = data.find_actor_instance(*leader_actor_id) else {
                return true;
            };

            let Some(leader_actor) = leader_instance.find_actor(*leader_actor_id) else {
                return true;
            };

            let Some(leader_spawn) = leader_actor.get_player_spawn() else {
                return true;
            };

            // This client is creating a party.
            if party_id == 0 {
                // TODO: We should probably generate these differently so there are no potential collisions.
                // NOTE: we store i64 in the database, hence why its chosen here.
                party_id = fastrand::i64(..) as u64;
                let chatchannel_id = fastrand::u32(..);
                let mut party = Party {
                    chatchannel_id,
                    leader_id: *leader_actor_id,
                    ..Default::default()
                };

                // Add the initial leader as the first member
                party.members.push(PartyMember {
                    actor_id: *leader_actor_id,
                    content_id: leader_spawn.content_id,
                    account_id: leader_spawn.account_id,
                    world_id: leader_spawn.home_world_id,
                    name: leader_spawn.common.name.clone(),
                    position: leader_spawn.common.position,
                    ..Default::default()
                });

                // We have to cache the leader's stuff earlier than the others so build_party_list can function properly here, as it checks if people are online first. This results in a redundant re-caching for the leader, but for the time being it's harmless and a non-issue.
                for (id, (handle, _)) in network.clients.clone() {
                    if handle.actor_id == *leader_actor_id {
                        party.members[0].zone_client_id = id;
                        break;
                    }
                }

                for (id, (handle, _)) in network.chat_clients.clone() {
                    if handle.actor_id == *leader_actor_id {
                        party.members[0].chat_client_id = id;
                        break;
                    }
                }

                network.parties.entry(party_id).or_insert(party);
                network.commit_parties = true;
            }

            if let Some(party) = network.parties.get(&party_id) {
                if party.members.len() >= PartyMemberEntry::NUM_ENTRIES {
                    tracing::error!(
                        "Tried to add a party member to a full party! What happened? {party:#?}"
                    );
                    return true;
                };

                let chatchannel_id = network.parties[&party_id].chatchannel_id;

                // Push existing party members into the list first.
                let mut party_list = build_party_list(party, &data);

                // Next, shadow for shorter typing, and take a clone of the party members, so we can edit them and give them back later.
                let mut party = party.members.clone();

                let mut target_content_id = 0;
                let mut target_account_id = 0;
                let mut target_name = String::default();

                // Add the new member to the party, and put them into the PartyList.
                'outer: for instance in &data.instances {
                    for (id, actor) in &instance.actors {
                        let Some(spawn) = actor.get_player_spawn() else {
                            continue;
                        };

                        if spawn.content_id == *new_member_content_id {
                            party.push(PartyMember {
                                actor_id: *id,
                                content_id: spawn.content_id,
                                account_id: spawn.account_id,
                                world_id: spawn.home_world_id,
                                name: spawn.common.name.clone(),
                                position: spawn.common.position,
                                ..Default::default()
                            });

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
                                sync_positions: 1,
                                unk2: 1,
                                ..Default::default()
                            });

                            target_content_id = spawn.content_id;
                            target_account_id = spawn.account_id;
                            target_name = spawn.common.name.clone();

                            break 'outer;
                        }
                    }
                }

                assert!(
                    !party_list.is_empty() && party_list.len() <= PartyMemberEntry::NUM_ENTRIES
                );

                assert!(
                    target_content_id != 0
                        && target_account_id != 0
                        && target_name != String::default()
                );

                let update_status = PartyUpdateStatus::JoinParty;

                let msg = FromServer::PartyUpdate(
                    PartyUpdateTargets {
                        execute_account_id: leader_spawn.account_id,
                        execute_content_id: leader_spawn.content_id,
                        execute_name: leader_spawn.common.name.clone(),
                        target_account_id,
                        target_content_id,
                        target_name,
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
                network.commit_parties = true;
            } else {
                tracing::error!("AddPartyMember: Party id wasn't in the hashmap! What happened?");
            }

            true
        }
        ToServer::PartyMemberChangedAreas(
            party_id,
            execute_account_id,
            execute_content_id,
            execute_actor_id,
            execute_name,
            zone_id,
        ) => {
            let mut network = network.lock();
            let data = data.lock();
            let party = network.parties.get(party_id).unwrap();

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

            // Next, inform the player about the party's target markers/signs, and waymarks.
            send_party_target_signs(&mut network, *party_id, *execute_actor_id);
            send_party_waymarks(&mut network, *party_id, *execute_actor_id, *zone_id);

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

            network.commit_parties = true;

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
            network.commit_parties = true;

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

            network.commit_parties = true;

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
                network.commit_parties = true;
            }

            true
        }
        ToServer::PartyMemberReturned(execute_actor_id, zone_id) => {
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

            // Next, inform the player about the party's target markers/signs, and waymarks.
            send_party_target_signs(&mut network, party_id, *execute_actor_id);
            send_party_waymarks(&mut network, party_id, *execute_actor_id, *zone_id);

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
                let mut dest_actor_id = ObjectId::default();
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
        ToServer::ApplyWaymarkPreset(from_id, party_id, waymark_preset, zone_id) => {
            let mut network = network.lock();
            let msg = FromServer::WaymarkPreset(*waymark_preset, *zone_id);

            if *party_id != 0 {
                network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
                let data = data.lock();
                update_party_waymarks(&mut network, &data, *from_id, *waymark_preset);
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
        ToServer::RidePillionRequest(
            from_actor_id,
            party_id,
            target_actor_id,
            target_seat_index,
        ) => {
            let mut network = network.lock();

            let Some(party_id) = party_id else {
                return true;
            };

            let Some(party) = network.parties.get_mut(party_id) else {
                return true;
            };

            for member in &mut party.members {
                // Store who the driver is so we later know how to update positions, etc.
                if member.actor_id == *from_actor_id {
                    member.pillion_driver_id = *target_actor_id;
                    break;
                }
            }

            let mut data = data.lock();
            let Some(instance) = data.find_actor_instance_mut(*from_actor_id) else {
                return true;
            };

            let mount_id;
            {
                // For now, it should be safe to assume the driver is in the same instance if the sending client is requesting to ride pillion.
                let Some(driver_actor) = instance.find_actor(*target_actor_id) else {
                    return true;
                };

                let common = driver_actor.get_common_spawn();
                mount_id = common.current_mount;
            }

            // TODO: Logic to move the player to an unoccupied seat when the desired seat is taken

            // Begin riding pillion
            network.send_ac_in_range_inclusive_instance(
                instance,
                *from_actor_id,
                ActorControlCategory::RidePillion {
                    target_actor_id: *target_actor_id,
                    target_seat_index: *target_seat_index,
                },
            );
            // Also hide the weapon
            network.send_ac_in_range_inclusive_instance(
                instance,
                *from_actor_id,
                ActorControlCategory::ToggleWeapon {
                    shown: false,
                    unk_flag: 1,
                },
            );

            // Inform the driver that someone new is riding
            network.send_to_by_actor_id(
                *target_actor_id,
                FromServer::ActorControlSelf(ActorControlCategory::PillionDriverRelatedUnk {
                    target_seat_index: *target_seat_index,
                    from_actor_id: *from_actor_id,
                }),
                DestinationNetwork::ZoneClients,
            );

            set_character_mode(
                instance,
                &mut network,
                *from_actor_id,
                CharacterMode::RidingPillion,
                1 + *target_seat_index as u8,
            );

            // Inform the passenger that they are riding
            network.send_to_by_actor_id(
                *from_actor_id,
                FromServer::ActorControlSelf(ActorControlCategory::PillionPassengerRelatedUnk {
                    unk: 12,
                }),
                DestinationNetwork::ZoneClients,
            );

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Mount {
                id: mount_id,
                unk1: [0; 14],
            });
            network.send_in_range_inclusive_instance(
                *target_actor_id,
                instance,
                FromServer::PacketSegment(ipc, *from_actor_id),
                DestinationNetwork::ZoneClients,
            );

            true
        }
        ToServer::ReadyCheckInitiated(
            party_id,
            from_actor_id,
            execute_account_id,
            execute_content_id,
            execute_name,
        ) => {
            let Some(party_id) = party_id else {
                return true;
            };

            let mut network = network.lock();
            let Some(party) = network.parties.get_mut(party_id) else {
                return true;
            };

            // Don't allow multiple ready checks at once.
            if party.readycheck_host.is_some() {
                return true;
            }

            // Set the initial ready check state.
            party.readycheck_host = Some(*from_actor_id);

            for member in &mut party.members {
                if member.actor_id == *from_actor_id {
                    member.ready_check_reply = ReadyCheckReply::Yes; // The ready check initiator's vote is always set to Yes.
                } else {
                    member.ready_check_reply = ReadyCheckReply::Unanswered;
                }
            }

            let data = data.lock();
            let party_list = build_party_list(party, &data);

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    ..Default::default()
                },
                PartyUpdateStatus::ReadyCheckInitiated,
                Some((*party_id, party.chatchannel_id, party.leader_id, party_list)),
            );

            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
            true
        }
        ToServer::ReadyCheckResponse(
            party_id,
            from_actor_id,
            execute_account_id,
            execute_content_id,
            execute_name,
            response,
        ) => {
            let Some(party_id) = party_id else {
                return true;
            };

            let mut network = network.lock();
            let Some(party) = network.parties.get_mut(party_id) else {
                return true;
            };

            // Don't proceed if no ready check is underway.
            let Some(readycheck_host) = party.readycheck_host else {
                return true;
            };

            let mut starter_index = usize::MAX;

            // Record both the party member's vote and store the ready check starter's member index so we can use it directly after this.
            for (index, member) in party.members.iter_mut().enumerate() {
                if member.actor_id == *from_actor_id {
                    member.ready_check_reply = *response;
                }
                // We don't cache the index of the starter because it's possible someone could leave the party during the entire voting process, which would produce undefined results.
                if member.actor_id == readycheck_host {
                    starter_index = index;
                }
            }

            assert!(starter_index != usize::MAX);

            // The actual ready check calculation. See party_misc.rs or the PartyUpdate struct for a more detailed explanation.
            // The short version is that we treat an 8 byte u64 as a pseudo-array to place member votes into, and send that back to the party members in the target_content_id field of PartyUpdate.
            // The initial accumulator state has to include the initiator's vote along with whoever responds first, since the actual initiation does not. Upon initiation, before anyone has responded, the target_content_id field is sent as 0 in PartyUpdateStatus::ReadyCheckInitiated.
            let mut accumulator =
                (party.members[starter_index].ready_check_reply as u64) << (8 * starter_index);
            for (index, member) in party.members.iter().enumerate() {
                if index == starter_index {
                    continue;
                }

                accumulator |= (member.ready_check_reply as u64) << (8 * index);
            }

            // Next, tally up how many members have voted, and if everyone has voted, either manually or being forced to auto-vote by timeout, indicate that there is no longer a ready check host, and send the final results.
            let voters_count = party
                .members
                .iter()
                .filter(|m| m.ready_check_reply != ReadyCheckReply::Unanswered)
                .count();

            if voters_count == party.members.len() {
                party.readycheck_host = None;
            }

            let data = data.lock();
            let party_list = build_party_list(party, &data);

            let msg = FromServer::PartyUpdate(
                PartyUpdateTargets {
                    execute_account_id: *execute_account_id,
                    execute_content_id: *execute_content_id,
                    execute_name: execute_name.clone(),
                    target_content_id: accumulator,
                    ..Default::default()
                },
                PartyUpdateStatus::ReadyCheckResponse,
                Some((*party_id, party.chatchannel_id, party.leader_id, party_list)),
            );

            network.send_to_party(*party_id, None, msg, DestinationNetwork::ZoneClients);
            true
        }
        _ => false,
    }
}
