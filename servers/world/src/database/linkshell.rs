use diesel::prelude::*;
use std::collections::HashMap;

use super::{WorldDatabase, models, schema, unixepoch};
use crate::GameData;
use kawari::{
    common::ObjectId,
    config::get_config,
    ipc::chat::{ChatChannel, ChatChannelType},
    ipc::zone::{
        CWLS_MAX_MEMBERS, CWLSCommon, CWLSCommonIdentifiers, CWLSMemberListEntry,
        CWLSNameAvailability, CWLSPermissionRank, CrossworldLinkshellEx, LWLS_MAX_MEMBERS,
        OnlineStatusMask,
    },
};

impl WorldDatabase {
    /// Returns a HashMap of linkshells for the global server state. NOTE: It does not fill in the members or chatchannel ids, and this is intentional! The global server waits for members to log in and inform it that they belong to a given set of linkshells.
    pub fn find_all_linkshells(&mut self) -> HashMap<u64, Vec<ObjectId>> {
        use schema::linkshells::dsl::*;

        let mut found_linkshells = HashMap::new();

        if let Ok(flat_linkshells) = linkshells
            .select(models::Linkshells::as_select())
            .load(&mut self.connection)
        {
            for linkshell in flat_linkshells {
                found_linkshells.insert(linkshell.id as u64, Vec::new());
            }
        }

        found_linkshells
    }

    /// Returns a list of linkshells that the given content id is a member of.
    pub fn find_linkshells(&mut self, for_content_id: i64) -> Option<Vec<CrossworldLinkshellEx>> {
        let memberships: Vec<_>;
        {
            use schema::linkshell_members::dsl::*;

            memberships = linkshell_members
                .filter(content_id.eq(for_content_id))
                .load::<models::LinkshellMembers>(&mut self.connection)
                .unwrap_or_default();
        }

        let mut shell_info = Vec::new();
        {
            use schema::linkshells::dsl::*;

            for membership in &memberships {
                if let Ok(info) = linkshells
                    .filter(id.eq(membership.linkshell_id))
                    .select(models::Linkshells::as_select())
                    .first(&mut self.connection)
                {
                    shell_info.push(info);
                }
            }
        }

        assert!(memberships.len() == shell_info.len());

        if !memberships.is_empty() && !shell_info.is_empty() {
            let mut ret = vec![CrossworldLinkshellEx::default(); CrossworldLinkshellEx::COUNT];

            for (index, shell) in ret.iter_mut().enumerate() {
                if index >= memberships.len() {
                    break;
                }

                shell.common.name = shell_info[index].name.clone();
                let rank = CWLSPermissionRank::from_repr(memberships[index].rank as u8);
                shell.common.rank = if let Some(rank) = rank {
                    rank
                } else {
                    CWLSPermissionRank::Invitee
                };
                shell.ids.linkshell_id = shell_info[index].id as u64;
                shell.ids.linkshell_chat_id = ChatChannel {
                    world_id: 10008,
                    channel_type: ChatChannelType::CWLinkshell,
                    channel_number: shell_info[index].id as u32,
                };
                shell.creation_time = shell_info[index].creation_time as u32;
            }

            return Some(ret);
        }

        None
    }

    pub fn find_linkshell_name(&mut self, for_linkshell_id: u64) -> Option<String> {
        use schema::linkshells::dsl::*;

        if let Ok(ls_name) = linkshells
            .select(name)
            .filter(id.eq(for_linkshell_id as i64))
            .first::<String>(&mut self.connection)
        {
            return Some(ls_name);
        }

        tracing::warn!("Unable to find linkshell name for {for_linkshell_id}!?");
        None
    }

    /// Removes all of a linkshell's members, and then removes the linkshell.
    pub fn remove_linkshell(&mut self, for_linkshell_id: u64) {
        use schema::linkshell_members::dsl::*;
        use schema::linkshells::dsl::*;

        let for_linkshell_id = for_linkshell_id as i64;
        if let Ok(linkshell) = linkshells
            .select(models::Linkshells::as_select())
            .filter(schema::linkshells::id.eq(for_linkshell_id))
            .load(&mut self.connection)
            && !linkshell.is_empty()
        {
            if let Ok(_) = diesel::delete(
                linkshell_members
                    .filter(schema::linkshell_members::linkshell_id.eq(for_linkshell_id)),
            )
            .execute(&mut self.connection)
                && let Ok(_) =
                    diesel::delete(linkshells.filter(schema::linkshells::id.eq(for_linkshell_id)))
                        .execute(&mut self.connection)
            {
                tracing::info!("Linkshell {for_linkshell_id} deleted!");
            }
        } else {
            tracing::warn!(
                "Got a request to delete non-existent linkshell {for_linkshell_id}, what happened?",
            );
        }
    }

    pub fn remove_member_from_linkshell(
        &mut self,
        for_content_id: i64,
        for_linkshell_id: u64,
    ) -> Option<u64> {
        if !self.is_in_linkshell(for_content_id as u64, for_linkshell_id) {
            return None;
        }

        let their_rank =
            self.find_linkshell_permissions(for_content_id as u64, for_linkshell_id)?;

        let for_linkshell_id = for_linkshell_id as i64;

        use schema::linkshell_members::dsl::*;
        if diesel::delete(
            linkshell_members
                .filter(content_id.eq(for_content_id))
                .filter(linkshell_id.eq(for_linkshell_id)),
        )
        .execute(&mut self.connection)
        .is_ok()
        {
            tracing::info!("Player {for_content_id} removed from linkshell {for_linkshell_id}!");
            if let Ok(members) = linkshell_members
                .select(models::LinkshellMembers::as_select())
                .filter(linkshell_id.eq(for_linkshell_id))
                .load(&mut self.connection)
                && members.is_empty()
            {
                tracing::info!(
                    "Linkshell {for_linkshell_id} has no members left! Auto-disbanding now."
                );
                self.remove_linkshell(for_linkshell_id as u64);
            } else if their_rank == CWLSPermissionRank::Master {
                // Else, if the leaving member was the owner, promote the oldest member, so as not to leave the LS orphaned.
                if let Ok(oldest_member) = linkshell_members
                    .select(content_id)
                    .filter(linkshell_id.eq(for_linkshell_id))
                    .order(invite_time.asc())
                    .first::<i64>(&mut self.connection)
                    && let Ok(_) = diesel::update(linkshell_members)
                        .filter(content_id.eq(oldest_member))
                        .filter(linkshell_id.eq(for_linkshell_id))
                        .set(rank.eq(CWLSPermissionRank::Master as i32))
                        .execute(&mut self.connection)
                {
                    tracing::info!(
                        "Due to leaving, the previous Master {for_content_id} of linkshell {for_linkshell_id} has designated {oldest_member} as the new Master!"
                    );
                    return Some(oldest_member as u64);
                }
            }
        }

        None
    }

    /// Returns a list of all members in the given linkshell.
    // TODO: We can likely just reuse this for local LSes too and "downscale" info they don't need in the zone connection
    pub fn find_linkshell_members(
        &mut self,
        for_linkshell_id: u64,
        game_data: &mut GameData,
    ) -> Option<Vec<CWLSMemberListEntry>> {
        use schema::linkshell_members::dsl::*;

        let mut members = Vec::new();
        let config = get_config();

        if let Ok(lsmembers) = linkshell_members
            .select(models::LinkshellMembers::as_select())
            .load(&mut self.connection)
        {
            let for_linkshell_id = for_linkshell_id as i64;
            for member in lsmembers {
                if member.linkshell_id == for_linkshell_id {
                    let player_info = self.get_player_entry(game_data, member.content_id);
                    let is_online = player_info.online_status_mask != OnlineStatusMask::default();
                    // If something goes wrong converting their rank, set it to least privileges as a precaution.
                    let member_rank =
                        if let Some(db_rank) = CWLSPermissionRank::from_repr(member.rank as u8) {
                            db_rank
                        } else {
                            CWLSPermissionRank::Invitee
                        };
                    members.push(CWLSMemberListEntry {
                        content_id: member.content_id as u64,
                        unk_timestamp: member.invite_time as u32,
                        home_world_id: config.world.world_id,
                        current_world_id: config.world.world_id,
                        name: player_info.name.clone(),
                        is_online,
                        zone_id: if is_online { player_info.zone_id } else { 0 },
                        rank: member_rank,
                        unk2: 1,
                        ..Default::default()
                    });
                }
            }
        }

        if members.is_empty() {
            return None;
        }

        Some(members)
    }

    pub fn is_linkshell_crossworld(&mut self, for_linkshell_id: u64) -> Option<bool> {
        use schema::linkshells::dsl::*;

        if let Ok(crossworld) = linkshells
            .select(is_crossworld)
            .filter(id.eq(for_linkshell_id as i64))
            .first::<bool>(&mut self.connection)
        {
            return Some(crossworld);
        }

        None
    }

    pub fn is_linkshell_full(&mut self, for_linkshell_id: u64) -> Option<bool> {
        if let Some(crossworld) = self.is_linkshell_crossworld(for_linkshell_id) {
            use schema::linkshell_members::dsl::*;
            if let Ok(members) = linkshell_members
                .select(models::LinkshellMembers::as_select())
                .filter(linkshell_id.eq(for_linkshell_id as i64))
                .load(&mut self.connection)
            {
                if (crossworld && members.len() >= CWLS_MAX_MEMBERS)
                    || (!crossworld && members.len() >= LWLS_MAX_MEMBERS)
                {
                    return Some(true);
                } else {
                    return Some(false);
                }
            }
        }
        None
    }

    // TODO: Change return type from a bool to a LogMessageType so the ZoneConnection can tell the client what happened.
    pub fn add_member_to_linkshell(
        &mut self,
        for_linkshell_id: i64,
        for_content_id: i64,
        their_rank: CWLSPermissionRank,
    ) -> bool {
        use schema::linkshell_members::dsl::*;

        let already_member = linkshell_members
            .select(content_id)
            .filter(content_id.eq(for_content_id))
            .filter(linkshell_id.eq(for_linkshell_id))
            .first::<i64>(&mut self.connection);

        // If they're not in this linkshell, add them.
        if already_member.is_err() {
            let next_id = if let Ok(highest) = linkshell_members
                .select(id)
                .order(id.desc())
                .first::<i64>(&mut self.connection)
            {
                highest + 1
            } else {
                1 // Start from a safe default if there are no members.
            };

            let ls_invite_time = diesel::select(unixepoch())
                .get_result::<i64>(&mut self.connection)
                .unwrap_or_default();

            let new_member = models::LinkshellMembers {
                id: next_id,
                content_id: for_content_id,
                linkshell_id: for_linkshell_id,
                invite_time: ls_invite_time,
                rank: their_rank as i32,
            };

            let result = diesel::insert_into(linkshell_members)
                .values(new_member)
                .execute(&mut self.connection);

            match result {
                Ok(_) => {
                    return true;
                }

                Err(err) => {
                    tracing::warn!(
                        "Failed to add member to linkshell {for_linkshell_id:#?} due to the following error: {err:#?}"
                    );
                    return false;
                }
            }
        } else {
            tracing::warn!(
                "This character {for_content_id} is already in this linkshell {for_content_id}!"
            );
        }

        false
    }

    /// Returns availability information for a desired linkshell name.
    pub fn linkshell_name_available(&mut self, desired_name: String) -> CWLSNameAvailability {
        // Linkshell names must be: between 3 and 20 characters in length, may contain punctuation, not contain double spaces/underscores, not contain a space at the start or end of the name, and the name may not consist solely of punctuation.
        // TODO: Should we bother enforcing the other rules if a player somehow bypassed the client-side limitations?
        use schema::linkshells::dsl::*;

        if desired_name.len() >= 3 && desired_name.len() <= 20 {
            let already_exists = linkshells
                .select(name)
                .filter(name.eq(desired_name.clone()))
                .first::<String>(&mut self.connection);

            if already_exists.is_err() {
                return CWLSNameAvailability::Available;
            }
        }
        CWLSNameAvailability::NotAvailable
    }

    /// Returns this player's linkshell membership status.
    pub fn is_in_linkshell(&mut self, for_content_id: u64, for_linkshell_id: u64) -> bool {
        use schema::linkshell_members::dsl::*;

        linkshell_members
            .select(content_id)
            .filter(content_id.eq(for_content_id as i64))
            .filter(linkshell_id.eq(for_linkshell_id as i64))
            .first::<i64>(&mut self.connection)
            .is_ok()
    }

    /// Returns this player's rank in the given linkshell.
    pub fn find_linkshell_permissions(
        &mut self,
        for_content_id: u64,
        for_linkshell_id: u64,
    ) -> Option<CWLSPermissionRank> {
        use schema::linkshell_members::dsl::*;

        if self.is_in_linkshell(for_content_id, for_linkshell_id)
            && let Ok(my_rank) = linkshell_members
                .select(rank)
                .filter(content_id.eq(for_content_id as i64))
                .filter(linkshell_id.eq(for_linkshell_id as i64))
                .first::<i32>(&mut self.connection)
            && let Some(my_rank) = CWLSPermissionRank::from_repr(my_rank as u8)
        {
            return Some(my_rank);
        }

        None
    }

    /// Sets this member's rank in the LS.
    pub fn set_linkshell_rank(
        &mut self,
        from_content_id: u64,
        for_content_id: u64,
        for_linkshell_id: u64,
        new_rank: CWLSPermissionRank,
    ) {
        use schema::linkshell_members::dsl::*;

        if self.is_in_linkshell(from_content_id, for_linkshell_id)
            && self.is_in_linkshell(for_content_id, for_linkshell_id)
        {
            match diesel::update(linkshell_members)
                .filter(content_id.eq(for_content_id as i64))
                .filter(linkshell_id.eq(for_linkshell_id as i64))
                .set(rank.eq(new_rank as i32))
                .execute(&mut self.connection)
            {
                Ok(_) => {
                    // If the Master is designating a new Master, demote the old Master to Member
                    if new_rank == CWLSPermissionRank::Master {
                        match diesel::update(linkshell_members)
                            .filter(content_id.eq(from_content_id as i64))
                            .filter(linkshell_id.eq(for_linkshell_id as i64))
                            .set(rank.eq(CWLSPermissionRank::Member as i32))
                            .execute(&mut self.connection)
                        {
                            Ok(_) => {
                                tracing::info!(
                                    "The previous Master {from_content_id} of linkshell {for_linkshell_id} has designated {for_content_id} as the new Master!"
                                );
                            }
                            Err(err) => tracing::warn!(
                                "The previous Master {from_content_id} could not be demoted due to the following error: {err:#?}!"
                            ),
                        }
                    } else {
                        tracing::info!(
                            "{for_content_id}'s rank in linkshell {for_linkshell_id} is now {new_rank:#?}!"
                        );
                    }
                }
                Err(err) => tracing::warn!(
                    "Unable to set rank for member {for_content_id} in linkshell {for_linkshell_id} because of the following error: {err:#?}!"
                ),
            }
        }
    }

    pub fn create_linkshell(
        &mut self,
        for_linkshell_id: Option<u64>,
        from_content_id: i64,
        ls_name: String,
        is_crossworld_ls: bool,
    ) -> Option<CrossworldLinkshellEx> {
        use schema::linkshells::dsl::*;

        let name_available = self.linkshell_name_available(ls_name.clone());

        // Only allow creation if this LS doesn't exist already. Probably a bit redundant with how the order of events goes, but never hurts.
        if name_available == CWLSNameAvailability::Available && for_linkshell_id.is_none() {
            let ls_creation_time = diesel::select(unixepoch())
                .get_result::<i64>(&mut self.connection)
                .unwrap_or_default();

            let next_id = if let Ok(highest) = linkshells
                .select(id)
                .order(id.desc())
                .first::<i64>(&mut self.connection)
            {
                highest + 1
            } else {
                1 // Start from a safe default if there are no linkshells at all on the server.
            };

            let linkshell = models::Linkshells {
                id: next_id,
                name: ls_name.clone(),
                creation_time: ls_creation_time,
                is_crossworld: is_crossworld_ls,
            };

            let result = diesel::insert_into(linkshells)
                .values(linkshell)
                .execute(&mut self.connection);

            match result {
                Ok(_) => {
                    let rank = CWLSPermissionRank::Master;
                    if self.add_member_to_linkshell(next_id, from_content_id, rank) {
                        return Some(CrossworldLinkshellEx {
                            ids: CWLSCommonIdentifiers {
                                linkshell_id: next_id as u64,
                                linkshell_chat_id: ChatChannel {
                                    world_id: 10008,
                                    channel_type: ChatChannelType::CWLinkshell,
                                    channel_number: next_id as u32,
                                },
                            },
                            creation_time: ls_creation_time as u32,
                            common: CWLSCommon {
                                rank,
                                name: ls_name.clone(),
                            },
                        });
                    }
                }
                Err(err) => tracing::error!(
                    "Failed to create the linkshell because of the following error: {err:#?}"
                ),
            }
        } else if name_available == CWLSNameAvailability::Available
            && let Some(for_linkshell_id) = for_linkshell_id
        {
            let for_linkshell_id = for_linkshell_id as i64;

            match diesel::update(linkshells)
                .filter(id.eq(for_linkshell_id))
                .set(name.eq(ls_name.clone()))
                .execute(&mut self.connection)
            {
                Ok(_) => {
                    tracing::info!("Linkshell {for_linkshell_id} renamed to {ls_name}!");

                    return Some(CrossworldLinkshellEx::default());
                }
                Err(err) => tracing::warn!(
                    "Unable to rename linkshell {for_linkshell_id} because of the following error: {err:#?}!"
                ),
            }
        }

        None
    }
}
