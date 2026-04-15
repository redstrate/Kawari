//! Other social features, as well as invite sending and replies.
use crate::{ToServer, ZoneConnection};
use kawari::{
    common::{LogMessageType, timestamp_secs},
    ipc::zone::{
        InviteReply, InviteType, InviteUpdateType, OnlineStatus, OnlineStatusMask, PlayerEntry,
        SearchUIClassJobMask, SearchUIGrandCompanies, ServerZoneIpcData, ServerZoneIpcSegment,
        SocialList, SocialListRequestType, SocialListUILanguages,
    },
};

pub fn fetch_entries<T>(
    next_index: &mut u16,
    data: &mut Vec<T>,
    increment_by: usize,
    state: &mut usize,
) -> Vec<T>
where
    T: Clone + std::default::Default,
{
    let mut ret: Vec<T>;
    if data.len() > increment_by {
        *next_index += increment_by as u16;
        ret = data.drain(0..increment_by).collect();
    } else {
        *next_index = 0;
        ret = std::mem::take(data);
        ret.resize(increment_by, T::default());
    }

    if !data.is_empty() {
        *state += increment_by;
    } else {
        *state = 0;
    }

    ret
}

impl ZoneConnection {
    pub async fn send_invite_update(
        &mut self,
        from_account_id: u64,
        from_content_id: u64,
        expiration_timestamp: u32,
        invite_type: InviteType,
        update_kind: Option<InviteUpdateType>,
        from_name: String,
        response: Option<InviteReply>,
    ) {
        if update_kind.is_some() && response.is_some() {
            tracing::error!(
                "Invalid state for send_invite_update: update_type and response cannot both be Some!"
            );
            return;
        }

        let update_type;
        if let Some(response) = response {
            update_type = match response {
                InviteReply::Accepted => InviteUpdateType::InviteAccepted,
                InviteReply::Declined => InviteUpdateType::InviteDeclined,
                InviteReply::Cancelled => InviteUpdateType::InviteCancelled,
            };
        } else if let Some(kind) = update_kind {
            update_type = kind;
        } else {
            tracing::error!(
                "Invalid state for send_invite_update: update_type and response cannot both be None!"
            );
            return;
        }

        let response = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteUpdate {
            sender_content_id: from_content_id,
            sender_account_id: from_account_id,
            expiration_timestamp,
            world_id: self.config.world_id,
            invite_type,
            update_type,
            unk1: 1,
            sender_name: from_name,
        });
        self.send_ipc_self(response).await;
    }

    pub async fn invite_reply(
        &mut self,
        inviter_content_id: u64,
        invite_type: InviteType,
        response: InviteReply,
    ) {
        let inviter_info;
        {
            let mut db = self.database.lock();
            inviter_info = db.find_character_ids(Some(inviter_content_id), None);
        }

        if let Some(inviter_info) = inviter_info {
            self.handle
                .send(ToServer::InvitationResponse(
                    self.player_data.character.actor_id,
                    self.player_data.character.service_account_id as u64,
                    self.player_data.character.content_id as u64,
                    self.player_data.character.name.clone(),
                    inviter_info.actor_id,
                    inviter_content_id,
                    inviter_info.name.clone(),
                    invite_type,
                    response,
                ))
                .await;
        } else {
            tracing::warn!("invite_reply: Unable to find {inviter_content_id}'s character info!")
        }
    }

    /// The player received an invitation response from another player.
    pub async fn received_invitation_response(
        &mut self,
        from_account_id: u64,
        from_content_id: u64,
        from_name: String,
        invite_type: InviteType,
        response: InviteReply,
    ) {
        match invite_type {
            InviteType::Party => {
                if response == InviteReply::Accepted {
                    self.handle
                        .send(ToServer::AddPartyMember(
                            self.party_id,
                            self.player_data.character.actor_id,
                            from_content_id,
                        ))
                        .await;
                }
            }
            InviteType::PendingFriendList => {
                if response == InviteReply::Accepted {
                    let mut database = self.database.lock();
                    database.accept_friend(
                        self.player_data.character.content_id,
                        from_content_id as i64,
                    );
                } else {
                    // We do nothing further than this and the invite update, because the inviter's client doesn't display anything on-screen when the invitee declines a friend request.
                    let mut db = self.database.lock();
                    db.remove_from_friend_list(
                        from_content_id as i64,
                        self.player_data.character.content_id,
                    );
                }
            }
            _ => todo!(), // Linkshells, FCs, and everything else?
        }

        self.send_invite_update(
            from_account_id,
            from_content_id,
            0,
            invite_type,
            None,
            from_name,
            Some(response),
        )
        .await;
    }

    /// The player needs to be informed about the reply they just sent.
    pub async fn send_invite_reply_result(
        &mut self,
        from_content_id: u64,
        from_name: String,
        invite_type: InviteType,
        response: InviteReply,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteReplyResult {
            content_id: from_content_id,
            invite_type,
            response,
            unk1: 1,
            character_name: from_name,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn invite_character_result(
        &mut self,
        content_id: u64,
        message_id: LogMessageType,
        invite_type: InviteType,
        character_name: String,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteCharacterResult {
            content_id,
            message_id,
            world_id: self.config.world_id,
            invite_type,
            unk1: 1,
            character_name,
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn send_social_invite(
        &mut self,
        content_id: u64,
        invite_type: InviteType,
        character_name: String,
    ) {
        let recipient_info;
        {
            let mut db = self.database.lock();
            let character_name = if content_id == 0 {
                Some(character_name.clone())
            } else {
                None
            };
            let content_id = if content_id != 0 {
                Some(content_id)
            } else {
                None
            };
            recipient_info = db.find_character_ids(content_id, character_name);
        }

        let Some(recipient_info) = recipient_info else {
            tracing::warn!(
                "send_social_invite: Unable to find character details for {content_id}!"
            );
            return;
        };

        if invite_type == InviteType::FriendList {
            self.add_to_friend_list(recipient_info.content_id as u64, 32);
        }

        self.handle
            .send(ToServer::InvitePlayerTo(
                self.player_data.character.actor_id,
                self.player_data.character.service_account_id as u64,
                self.player_data.character.content_id as u64,
                self.player_data.character.name.clone(),
                recipient_info.actor_id,
                recipient_info.content_id as u64,
                character_name.clone(),
                invite_type,
            ))
            .await;
    }

    pub async fn received_social_invite(
        &mut self,
        sender_account_id: u64,
        sender_content_id: u64,
        sender_name: String,
        invite_type: InviteType,
    ) {
        let mut expiration_timestamp = timestamp_secs() + 300;
        if invite_type == InviteType::FriendList {
            expiration_timestamp = 0;
            self.add_to_friend_list(sender_content_id, 48);
        }

        self.send_invite_update(
            sender_account_id,
            sender_content_id,
            expiration_timestamp,
            invite_type,
            Some(InviteUpdateType::NewInvite),
            sender_name.clone(),
            None,
        )
        .await;
    }

    pub async fn send_social_list(
        &mut self,
        request_type: SocialListRequestType,
        sequence: u8,
        entries: Option<Vec<PlayerEntry>>,
        community_id: Option<u64>,
    ) {
        let mut next_index;
        let current_index;

        let mut entries = entries.unwrap_or_default();

        fn fetch_entries(next_index: &mut u16, data: &mut Vec<PlayerEntry>) -> Vec<PlayerEntry> {
            if data.len() > PlayerEntry::COUNT {
                *next_index += PlayerEntry::COUNT as u16;
                data.drain(0..PlayerEntry::COUNT).collect()
            } else {
                *next_index = 0;
                let mut ret: Vec<PlayerEntry> = std::mem::take(data);
                ret.resize(PlayerEntry::COUNT, PlayerEntry::default());
                ret
            }
        }

        // TODO: Use the new generic version of fetch_entries above after testing these for regressions. Works fine with cwls lists though.
        match request_type {
            SocialListRequestType::Friends => {
                current_index = self.friend_index as u16;
                next_index = self.friend_index as u16;
                entries = fetch_entries(&mut next_index, &mut self.friend_results);

                if !self.friend_results.is_empty() {
                    self.friend_index += PlayerEntry::COUNT;
                } else {
                    self.friend_index = 0;
                }
            }
            SocialListRequestType::SearchResults => {
                current_index = self.search_index as u16;
                next_index = self.search_index as u16;
                entries = fetch_entries(&mut next_index, &mut self.search_results);

                if !self.search_results.is_empty() {
                    self.search_index += PlayerEntry::COUNT;
                } else {
                    self.search_index = 0;
                }
            }
            SocialListRequestType::Party => {
                current_index = 0;
                next_index = 0;
            }
            _ => todo!(),
        }

        let community_id = community_id.unwrap_or_default();

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SocialList(SocialList {
            community_id,
            next_index,
            current_index,
            request_type,
            sequence,
            entries,
        }));

        self.send_ipc_self(ipc).await;
    }

    /// Determine the online status mask, with party/novice/mentor status.
    pub fn get_online_status_mask(&self) -> OnlineStatusMask {
        let mut database = self.database.lock();
        database.determine_online_status_mask(self.player_data.character.content_id)
    }

    /// Grabs the correct online status, taking into account the priority of each icon.
    pub fn get_actual_online_status(&self) -> OnlineStatus {
        let mask = self.get_online_status_mask();
        let priorities;
        {
            let mut gamedata = self.gamedata.lock();
            priorities = gamedata.online_status_priorities();
        }
        let mut priorities: Vec<(usize, &u8)> = priorities.iter().enumerate().collect();
        priorities.sort_by(|(_, a_priority), (_, b_priority)| {
            a_priority.partial_cmp(b_priority).unwrap()
        }); // So the highest priority (e.g. "AFK" is above "Online") are the first indices

        for (i, _) in priorities {
            let online_status = OnlineStatus::from_repr(i as u8).unwrap();
            if mask.has_status(online_status) {
                return online_status;
            }
        }

        OnlineStatus::Offline
    }

    /// Updates the online status not just on yourself but also informing other players.
    pub async fn update_online_status(&mut self) {
        // TODO: re-review this now that OnlineStatusMask can be calculated independently from any ZoneConnection

        let online_status_mask = self.get_online_status_mask();

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SetOnlineStatus(online_status_mask));
        self.send_ipc_self(ipc).await;

        self.handle
            .send(ToServer::SetOnlineStatus(
                self.player_data.character.actor_id,
                self.get_actual_online_status(),
            ))
            .await;
    }

    /// Searches for online players.
    pub async fn search_players(
        &mut self,
        classjobs: SearchUIClassJobMask,
        minimum_level: u16,
        maximum_level: u16,
        grand_companies: SearchUIGrandCompanies,
        languages: SocialListUILanguages,
        online_status: OnlineStatusMask,
        areas: [u16; 50],
        name: String,
    ) {
        use std::collections::HashSet;

        // First, grab up to 200 online players.
        {
            let mut db = self.database.lock();
            let mut game_data = self.gamedata.lock();
            self.search_results =
                db.find_online_players(&mut game_data, self.player_data.character.content_id);
            self.search_index = 0;
        }

        // The Online Status portion of the client's search, broken into individual OnlineStatuses.
        let search_onlinestatus_masks = online_status.mask();

        // The classjob portion of the client's search, broken into individual SearchUIClassJobs.
        let search_classjobs = classjobs.mask();

        // Reorganize the areas so that none are zeroes. Zero indicates this entry isn't being searched for.
        let areas: Vec<_> = areas.iter().filter(|&zone| *zone != 0).collect();

        // TODO: Classjob filtering, it's weird. Need to look at it closer
        // Filter the results based on the client's preferences.
        for player in self.search_results.clone() {
            // Remove this player if they don't have a similar name.
            if name != String::default() {
                // Check if we're searching by last name. The client sends the last name query with a space at the beginning.
                let by_last_name = name.chars().nth(0).unwrap() == ' ';

                // Next, correct the search query string to remove any spaces.
                let search_name = name.trim().to_owned();

                // Split the player's full name into first and last halves.
                let name_split: Vec<&str> = player.name.split(' ').collect();

                let my_name = if by_last_name {
                    name_split[1]
                } else {
                    name_split[0]
                };

                // If this player's name doesn't have a match, they're not relevant to this search.
                if !my_name.contains(&search_name) {
                    self.search_results
                        .retain(|p| p.content_id != player.content_id);
                    continue;
                }
            }

            // Remove this player if they don't meet the classjob search criteria.
            if !search_classjobs.is_empty() {
                let search_classjobs: HashSet<&u8> = HashSet::from_iter(search_classjobs.iter());

                // Since this type is likely not used anywhere else, we have to convert the player's classjob_id to something we can work with, unlike OnlineStatusMask.
                let mut player_classjobs = SearchUIClassJobMask::default();

                // Once we have their classjob_id as a SearchUIClassJob, set it in the imaginary mask and then make a `HashSet` out of it.
                player_classjobs.set_classjob(player.classjob_id - 1); // The id minus 1 because classjob ids start at 1, not 0, so we need to account for this.
                let player_classjobs = player_classjobs.mask();
                let player_classjobs = HashSet::from_iter(player_classjobs.iter());

                // Finally, compare our two HashSets and check for a lack of intersections. If so, remove this player.
                if search_classjobs.intersection(&player_classjobs).count() == 0 {
                    self.search_results
                        .retain(|p| p.content_id != player.content_id);
                    continue;
                }
            }

            // Remove this player if they don't fall into this level range.
            if player.classjob_level < minimum_level as u8
                || player.classjob_level > maximum_level as u8
            {
                self.search_results
                    .retain(|p| p.content_id != player.content_id);
                continue;
            }

            // Remove this player if they don't have at least one or more of these OnlineStatuses, but also allow OnlineStatus::Online a free pass (if the client searches for no OnlineStatuses, that's the only one sent).
            if !search_onlinestatus_masks.is_empty()
                && search_onlinestatus_masks[0] != OnlineStatus::Online
            {
                // Build `HashSet`s out of the two OnlineStatusMasks, and check if there are any matches.
                let search_onlinestatus_masks: HashSet<&OnlineStatus> =
                    HashSet::from_iter(search_onlinestatus_masks.iter());
                let player_masks = player.online_status_mask.mask();
                let player_masks: HashSet<&OnlineStatus> = HashSet::from_iter(player_masks.iter());

                // If there are no overlapping OnlineStatuses, this player isn't relevant.
                if search_onlinestatus_masks
                    .intersection(&player_masks)
                    .count()
                    == 0
                {
                    self.search_results
                        .retain(|p| p.content_id != player.content_id);
                    continue;
                }
            }

            // Remove this player if they aren't a member of any of the specified companies (it's not a strict search).
            if grand_companies != SearchUIGrandCompanies::NONE {
                let player_gc = SearchUIGrandCompanies::from(&player.grand_company);

                if !grand_companies.intersects(player_gc) {
                    self.search_results
                        .retain(|p| p.content_id != player.content_id);
                    continue;
                }
            }

            // Remove this player if their Social UI languages don't match what the client is looking for (but allow for any matches, it's not a strict search).
            // Client languages are not considered in this check, only Social UI languages.
            if !languages.intersects(player.social_ui_languages) {
                self.search_results
                    .retain(|p| p.content_id != player.content_id);
                continue;
            }

            // If all other search conditions succeed, filter by area. This one is last instead of 4th, because there currently isn't a good condition to check against. The location check *always* happens if name, classjob and level all pass. You can't search for no areas, essentially.
            let mut game_data = self.gamedata.lock();

            // If the player is in an invalid zone somehow, or isn't in a region being searched for, they're not relevant.
            let Some(player_region) = game_data.get_territory_placenamezone_data(player.zone_id)
            else {
                tracing::error!(
                    "search_players: Player was likely in an invalid zone, zone id is {}, and we weren't able to get PlaceNameZone data. Skipping them for this search condition.",
                    player.zone_id
                );
                continue;
            };

            if !areas.contains(&&player_region) {
                self.search_results
                    .retain(|p| p.content_id != player.content_id);
            }
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SearchPlayersResult {
            num_results: self.search_results.len() as u32,
        });
        self.send_ipc_self(ipc).await;
    }
}
