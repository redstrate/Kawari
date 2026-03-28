//! The party system and other social features.

use crate::{ToServer, ZoneConnection, common::PartyUpdateTargets};
use kawari::{
    common::{LogMessageType, ObjectId, ObjectTypeId, timestamp_secs},
    ipc::{
        chat::{ChatChannel, ChatChannelType},
        zone::{
            ActorControlCategory, CWLSLeaveReason, CWLSMemberListEntry, CWLSPermissionRank,
            CrossworldLinkshell, CrossworldLinkshellEx, InviteReply, InviteType, InviteUpdateType,
            OnlineStatus, OnlineStatusMask, PartyMemberEntry, PartyUpdateStatus, PlayerEntry,
            SearchUIClassJobMask, SearchUIGrandCompanies, ServerZoneIpcData, ServerZoneIpcSegment,
            SocialList, SocialListRequestType, SocialListUILanguages, StrategyBoard,
            StrategyBoardUpdate, WaymarkPlacementMode, WaymarkPosition, WaymarkPreset,
        },
    },
};

fn fetch_entries<T>(
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
    pub async fn received_party_invite(
        &mut self,
        sender_account_id: u64,
        sender_content_id: u64,
        sender_name: String,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteUpdate {
            sender_account_id,
            sender_content_id,
            expiration_timestamp: timestamp_secs() + 300, // usually the packet's timestamp + 300, TODO: we might want to keep a timer going somewhere to inform the original sender if it expires due to timeout, does retail do that?
            world_id: self.config.world_id,
            invite_type: InviteType::Party,
            update_type: InviteUpdateType::NewInvite,
            unk1: 1,
            sender_name,
        });
        self.send_ipc_self(ipc).await;
    }

    pub async fn send_invite_update(
        &mut self,
        from_account_id: u64,
        from_content_id: u64,
        from_name: String,
        invite_type: InviteType,
        response: InviteReply,
    ) {
        let update_type = match response {
            InviteReply::Accepted => InviteUpdateType::InviteAccepted,
            InviteReply::Declined => InviteUpdateType::InviteDeclined,
            InviteReply::Cancelled => InviteUpdateType::InviteCancelled,
        };

        let response = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteUpdate {
            sender_content_id: from_content_id,
            sender_account_id: from_account_id,
            expiration_timestamp: 0,
            world_id: self.config.world_id,
            invite_type,
            update_type,
            unk1: 1,
            sender_name: from_name,
        });
        self.send_ipc_self(response).await;
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
        if response == InviteReply::Accepted {
            match invite_type {
                InviteType::Party => {
                    self.handle
                        .send(ToServer::AddPartyMember(
                            self.party_id,
                            self.player_data.character.actor_id,
                            from_content_id,
                        ))
                        .await;
                }
                InviteType::PendingFriendList => {
                    let mut database = self.database.lock();
                    database.accept_friend(
                        self.player_data.character.content_id,
                        from_content_id as i64,
                    );
                }
                _ => todo!(), // Linkshells, FCs, and everything else?
            }
        }

        self.send_invite_update(
            from_account_id,
            from_content_id,
            from_name,
            invite_type,
            response,
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

    // A party event happened, so we need to inform our client.
    pub async fn send_party_update(
        &mut self,
        targets: PartyUpdateTargets,
        update_status: PartyUpdateStatus,
        party_info: Option<(u64, u32, ObjectId, Vec<PartyMemberEntry>)>,
    ) {
        let mut member_count = 0;
        if let Some((party_id, chatchannel_id, leader_actor_id, mut party_list)) = party_info {
            if self.party_id == 0 {
                self.party_id = party_id;
            }

            member_count = party_list.len() as u8;

            let Some(leader_index) = party_list
                .iter()
                .position(|x: &PartyMemberEntry| x.actor_id == leader_actor_id)
            else {
                tracing::error!(
                    "Unable to determine party leader! What happened? {} {} {} {:#?}",
                    party_id,
                    chatchannel_id,
                    leader_actor_id,
                    party_list
                );
                return;
            };

            // We edit the party list to hide information of players not in our zone.
            for member in party_list.iter_mut() {
                if (member.actor_id != self.player_data.character.actor_id
                    && member.current_zone_id != self.player_data.volatile.zone_id as u16)
                    || (update_status == PartyUpdateStatus::MemberWentOffline
                        && member.content_id == targets.execute_content_id)
                {
                    member.actor_id = ObjectId(0);
                    member.classjob_id = 0;
                    member.classjob_level = 0;
                    member.current_hp = 0;
                    member.max_hp = 0;
                    member.current_mp = 0;
                    member.max_mp = 0;
                    // Don't want to sync positions of offline people.
                    member.sync_positions = 0;
                    member.unk2 = 0;
                }
            }

            // Ensure we have only the correct amount of entries. Possibly redundant with binrw, but it doesn't hurt to be safe.
            party_list.resize(PartyMemberEntry::NUM_ENTRIES, PartyMemberEntry::default());

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PartyList {
                members: party_list,
                member_count,
                leader_index: leader_index as u8,
                party_id: self.party_id,
                party_chatchannel: ChatChannel {
                    channel_number: chatchannel_id,
                    channel_type: ChatChannelType::Party,
                    world_id: self.config.world_id,
                },
            });

            self.send_ipc_self(ipc).await;

            self.is_party_leader = self.player_data.character.actor_id == leader_actor_id;
        } else {
            // If there's no data, then we're the one who left.
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PartyList {
                members: vec![PartyMemberEntry::default(); PartyMemberEntry::NUM_ENTRIES],
                member_count,
                leader_index: 0,
                party_id: 0,
                party_chatchannel: ChatChannel {
                    channel_number: 0,
                    channel_type: ChatChannelType::Party,
                    world_id: self.config.world_id,
                },
            });
            self.send_ipc_self(ipc).await;

            self.party_id = 0;
            self.is_party_leader = false;
        }

        // TODO:
        // after partylist they send playerstats, but we'll skip it for now
        // after stats they send a second redundant ac SetStatusIcon and UpdateOnlineStatusMask

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::PartyUpdate {
            execute_account_id: targets.execute_account_id,
            target_account_id: targets.target_account_id,
            execute_content_id: targets.execute_content_id,
            target_content_id: targets.target_content_id,
            update_status,
            execute_name: targets.execute_name,
            target_name: targets.target_name,
            unk1: 1,
            unk2: if update_status == PartyUpdateStatus::ReadyCheckResponse {
                0
            } else {
                2 // TODO: figure out what the other values of unk2 mean
            },
            unk3: member_count,
        });

        self.send_ipc_self(ipc).await;

        // TODO:
        // after party update they send the status effect list
        // after the status effect list they send updateclassinfo

        // Ensure our online status is updated, since that is affected by whether we're in a party etc.
        self.update_online_status().await;
    }

    pub fn party_member_entries(&self) -> Vec<PlayerEntry> {
        let mut entries = Vec::new();

        let mut database = self.database.lock();
        let mut game_data = self.gamedata.lock();
        if self.party_id != 0 {
            entries = database.get_party_entries(&mut game_data, self.party_id as i64);
        } else {
            entries.push(
                database.get_player_entry(&mut game_data, self.player_data.character.content_id),
            );
        }

        entries
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

    pub fn is_in_party(&self) -> bool {
        self.party_id != 0
    }

    pub async fn received_strategy_board(&mut self, content_id: u64, board_data: StrategyBoard) {
        // TODO: Figure out what all these mean!
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::BeginStrategyBoardSession {
            unk1: 0x01010100,
            unk2: 0x04010101,
            unk3: 0x00010101,
        });

        self.send_ipc_self(ipc).await;

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StrategyBoard {
            content_id,
            board_data,
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn strategy_board_ack(&mut self, content_id: u64) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StrategyBoardReceivedAck {
            content_id,
            unk: 1,
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn strategy_board_updated(&mut self, update_data: StrategyBoardUpdate) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::StrategyBoardUpdate(update_data));

        self.send_ipc_self(ipc).await;
    }

    pub async fn strategy_board_realtime_finished(&mut self) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::EndStrategyBoardSession { unk: [0; 16] });

        self.send_ipc_self(ipc).await;
    }

    /// Someone in the party updated a single waymark.
    pub async fn waymark_updated(
        &mut self,
        id: u8,
        placement_mode: WaymarkPlacementMode,
        pos: WaymarkPosition,
        zone_id: i32,
    ) {
        // Ignore updates that aren't relevant to us, so that people in different zones can have their own waymarks going on.
        if zone_id == self.player_data.volatile.zone_id {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::WaymarkUpdate {
                id,
                placement_mode,
                pos,
            });

            self.send_ipc_self(ipc).await;
        }
    }

    /// Someone in the party loaded a waymark preset, or cleared all waymarks.
    pub async fn waymark_preset(&mut self, data: WaymarkPreset, zone_id: i32) {
        // Ignore updates that aren't relevant to us, so that people in different zones can have their own waymark presets going on.
        if zone_id == self.player_data.volatile.zone_id {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::FieldMarkerPreset(data));
            self.send_ipc_self(ipc).await;
        }
    }

    /// Someone in the party started a countdown.
    pub async fn start_countdown(
        &mut self,
        account_id: u64,
        content_id: u64,
        starter_name: String,
        starter_actor_id: ObjectId,
        duration: u16,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::Countdown {
            account_id,
            content_id,
            starter_actor_id,
            unk: 0x5B,
            duration,
            starter_name,
        });
        self.send_ipc_self(ipc).await;
    }

    /// Someone in the party marked a target with a sign.
    pub async fn target_sign_toggled(
        &mut self,
        sign_id: u32,
        from_actor_id: ObjectId,
        target_actor: ObjectTypeId,
    ) {
        self.actor_control_target(
            from_actor_id,
            target_actor,
            ActorControlCategory::ToggleSign {
                sign_id,
                from_actor_id,
            },
        )
        .await;
    }

    /// Determine the online status mask, with party/novice/mentor status.
    fn get_online_status_mask(&self) -> OnlineStatusMask {
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

    pub async fn invite_character_result(
        &mut self,
        content_id: u64,
        message_id: LogMessageType,
        world_id: u16,
        invite_type: InviteType,
        character_name: String,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteCharacterResult {
            content_id,
            message_id,
            world_id,
            invite_type,
            unk1: 1,
            character_name,
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn refresh_friend_list(&mut self) {
        // Only refresh if we ran out of results from a prior run.
        if self.friend_results.is_empty() {
            let mut db = self.database.lock();
            let mut game_data = self.gamedata.lock();
            self.friend_results =
                db.find_friend_list(&mut game_data, self.player_data.character.content_id);

            self.friend_index = 0;
        }
    }

    pub fn add_to_friend_list(&mut self, friend_content_id: u64) {
        let mut db = self.database.lock();
        db.add_to_friend_list(
            friend_content_id as i64,
            self.player_data.character.content_id,
        );
    }

    pub async fn received_friend_invite(
        &mut self,
        sender_account_id: u64,
        sender_content_id: u64,
        sender_name: String,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::InviteUpdate {
            sender_account_id,
            sender_content_id,
            expiration_timestamp: 0,
            world_id: self.config.world_id,
            invite_type: InviteType::FriendList,
            update_type: InviteUpdateType::NewInvite,
            unk1: 1,
            sender_name,
        });
        self.send_ipc_self(ipc).await;

        self.add_to_friend_list(sender_content_id);
    }

    /// Update or refresh our ls/cwls info.
    pub async fn init_linkshells(&mut self) {
        {
            let mut db = self.database.lock();
            self.cwls_memberships = db.find_linkshells(self.player_data.character.content_id);
        }
        // TODO: local shells

        // Don't bother the server if we're not in any linkshells.
        if let Some(cwls_memberships) = &self.cwls_memberships {
            self.handle
                .send(ToServer::SetLinkshells(
                    self.player_data.character.actor_id,
                    Some(
                        cwls_memberships
                            .iter()
                            .map(|m| (m.ids.linkshell_id, m.common.rank))
                            .collect(),
                    ),
                    None,
                    true,
                ))
                .await;
        }
    }

    // TODO: Extend to support locals too
    pub async fn set_linkshell_chatchannels(&mut self, cwls_channels: Vec<u32>) {
        if let Some(cwls_memberships) = &mut self.cwls_memberships {
            for channel_info in cwls_memberships.iter_mut().zip(cwls_channels) {
                channel_info.0.ids.linkshell_chat_id.channel_number = channel_info.1;

                // Unfortunately, we can't let the chat connection decide these without pointlessly bothering the global server state
                channel_info.0.ids.linkshell_chat_id.world_id = 10008;
                channel_info.0.ids.linkshell_chat_id.channel_type = ChatChannelType::CWLinkshell;
            }
        }
    }

    // TODO: Where else is this sent, if anywhere?
    pub async fn send_crossworld_linkshells(&mut self, detailed: bool) {
        if detailed {
            // Send a more detailed report about all of the client's cross-world linkshells. Sent when the client opens the CWLS menu. It contains extra information about when the CWLS was founded.
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellsEx {
                linkshells: if let Some(cwls_memberships) = &self.cwls_memberships {
                    cwls_memberships.clone()
                } else {
                    vec![CrossworldLinkshellEx::default(); CrossworldLinkshellEx::COUNT]
                },
            });

            self.send_ipc_self(ipc).await;
        } else {
            // Send a (very slightly) less detailed "overview" of cross-world linkshells on login and possibly elsewhere. Probably used so the client can chat without having to open the actual cwls menu.
            let mut cwlses = vec![CrossworldLinkshell::default(); CrossworldLinkshell::COUNT];

            if let Some(cwls_memberships) = &self.cwls_memberships {
                // Our cache stores the extended version, so we need to translate it back.
                for cwls in cwlses.iter_mut().zip(cwls_memberships.iter()) {
                    cwls.0.common.name = cwls.1.common.name.clone();
                    cwls.0.ids.linkshell_id = cwls.1.ids.linkshell_id;
                    cwls.0.common.rank = cwls.1.common.rank;
                    cwls.0.ids.linkshell_chat_id = cwls.1.ids.linkshell_chat_id;
                }
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshells {
                linkshells: cwlses,
            });
            self.send_ipc_self(ipc).await;
        }
    }

    // TODO: Likely extend this for local LSes too
    pub async fn send_cwlinkshell_members(&mut self, linkshell_id: u64, sequence: u16) {
        // Only refresh and reset state if our list is empty.
        if self.cwls_results.is_empty() {
            let mut db = self.database.lock();
            let mut gamedata = self.gamedata.lock();
            if let Some(cwls_results) = db.find_linkshell_members(linkshell_id, &mut gamedata) {
                self.cwls_results = cwls_results;
            } else {
                // If we somehow are told about an empty linkshell, ensure we can at least provide a blank member list so the client doesn't experience oddities beyond that.
                self.cwls_results = vec![CWLSMemberListEntry::default(); 8];
            }
            self.cwls_index = 0;
        }

        let current_index = self.cwls_index as u16;
        let mut next_index = self.cwls_index as u16;

        let members = fetch_entries(
            &mut next_index,
            &mut self.cwls_results,
            CWLSMemberListEntry::COUNT,
            &mut self.cwls_index,
        );

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellMemberList {
            next_index,
            current_index,
            linkshell_id,
            sequence,
            members,
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn check_cwlinkshell_name_availability(&mut self, name: String) {
        let result;
        {
            let mut db = self.database.lock();
            result = db.linkshell_name_available(name.clone());
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CWLinkshellNameAvailability {
            result,
            name,
            unk1: 1,
        });

        self.send_ipc_self(ipc).await;
    }

    // TODO: For all of these linkshell-related functions, make a helper function to handle updating cwls_memberships and sending ToServer::SetLinkshells as much as possible

    /// Creates a new cross-world linkshell and then informs both the global server & the client about it.
    pub async fn create_crossworld_linkshell(&mut self, name: String) {
        let mut cwlses = vec![(0, CWLSPermissionRank::Invitee); CrossworldLinkshellEx::COUNT];
        {
            let info;
            {
                let mut db = self.database.lock();
                info = db.create_linkshell(
                    None,
                    self.player_data.character.content_id,
                    name.clone(),
                    true,
                );
            }

            // If LS creation is successful, prepare some info for both the client and the global server state.
            if let Some(info) = info {
                if let Some(cwls_memberships) = &mut self.cwls_memberships {
                    let mut found_empty_slot = false;
                    for (index, linkshell) in cwls_memberships.iter_mut().enumerate() {
                        // Fill the first empty slot on our side with the new linkshell's info.
                        if !found_empty_slot && linkshell.ids.linkshell_id == 0 {
                            *linkshell = info.clone();
                            found_empty_slot = true;
                        }

                        // Fill in the global server's copy of the info.
                        cwlses[index] = (linkshell.ids.linkshell_id, linkshell.common.rank);
                    }
                } else {
                    // Otherwise, even if we didn't have any linkshells before, we do now.
                    let mut new_memberships =
                        vec![CrossworldLinkshellEx::default(); CrossworldLinkshellEx::COUNT];
                    new_memberships[0] = info.clone();
                    cwlses[0] = (info.ids.linkshell_id, info.common.rank);
                    self.cwls_memberships = Some(new_memberships);
                }

                self.handle
                    .send(ToServer::SetLinkshells(
                        self.player_data.character.actor_id,
                        Some(cwlses),
                        None, // TODO: local linkshells
                        false,
                    ))
                    .await;

                let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::NewCrossworldLinkshell {
                    ids: info.ids.clone(),
                    unk_timestamp1: info.creation_time,
                    unk_timestamp2: info.creation_time,
                    common: info.common.clone(),
                });

                self.send_ipc_self(ipc).await;
            }
        }
    }

    // TODO: I think this and crossworld_linkshell_disbanded can be combined, need to consider it
    pub async fn disband_linkshell(&mut self, linkshell_id: u64) {
        if let Some(cwls_memberships) = &self.cwls_memberships {
            for linkshell in cwls_memberships {
                if linkshell.ids.linkshell_id == linkshell_id {
                    if linkshell.common.rank == CWLSPermissionRank::Master {
                        {
                            let mut db = self.database.lock();
                            db.remove_linkshell(linkshell_id);
                        }
                        self.handle
                            .send(ToServer::DisbandLinkshell(linkshell_id))
                            .await;
                    } else {
                        tracing::warn!(
                            "Client {} tried to disband linkshell {} with permission_rank {:#?}! Rejecting request!",
                            self.player_data.character.content_id,
                            linkshell_id,
                            linkshell.common.rank
                        );
                    }

                    break;
                }
            }
        }
    }

    pub async fn crossworld_linkshell_disbanded(&mut self, linkshell_id: u64) {
        let mut cwlses = vec![(0, CWLSPermissionRank::Invitee); 8];
        if let Some(cwls_memberships) = &mut self.cwls_memberships {
            let mut disbanded_name = String::default();
            for (index, cwls) in cwls_memberships.iter_mut().enumerate() {
                if cwls.ids.linkshell_id == linkshell_id {
                    disbanded_name = cwls.common.name.clone();
                    *cwls = CrossworldLinkshellEx::default();
                }

                cwlses[index] = (cwls.ids.linkshell_id, cwls.common.rank);
            }

            // If we're no longer in any cross-world Linkshells
            if cwlses.iter().filter(|&linkshell| linkshell.0 == 0).count() == 8 {
                self.cwls_memberships = None;
            }

            // Inform the client.
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellDisbanded {
                linkshell_id,
                name: disbanded_name.to_string(),
            });

            self.send_ipc_self(ipc).await;
        }

        // Tell the global state about our remaining memberships.
        self.handle
            .send(ToServer::SetLinkshells(
                self.player_data.character.actor_id,
                Some(cwlses),
                None,
                false,
            ))
            .await;
    }

    pub async fn remove_linkshell_member(
        &mut self,
        linkshell_id: u64,
        target_content_id: u64,
        reason_for_leaving: CWLSLeaveReason,
    ) {
        let target_actor_id;
        let target_name;
        {
            let mut db = self.database.lock();

            // If we're kicking someone, check permissions first.
            let has_permission = db.has_linkshell_permissions(
                self.player_data.character.content_id as u64,
                linkshell_id,
                CWLSPermissionRank::Leader,
            );
            if target_content_id != (self.player_data.character.content_id as u64)
                && !has_permission
            {
                tracing::warn!(
                    "Player {} tried to kick {} from linkshell {}, but did not have permission to do so! Rejecting request!",
                    self.player_data.character.content_id as u64,
                    target_content_id,
                    linkshell_id
                );
                return;
            }

            target_actor_id = db.find_actor_id(target_content_id);
            target_name = db.find_character_name(target_content_id);
        }

        if let Some(target_name) = target_name
            && target_actor_id.is_valid()
        {
            self.handle
                .send(ToServer::LeaveLinkshell(
                    target_actor_id,
                    self.player_data.character.content_id as u64,
                    target_content_id,
                    target_name,
                    reason_for_leaving,
                    linkshell_id,
                ))
                .await;
        }
    }

    pub async fn member_left_linkshell(
        &mut self,
        execute_content_id: u64,
        target_content_id: u64,
        target_name: String,
        reason_for_leaving: CWLSLeaveReason,
        linkshell_id: u64,
    ) {
        // If we're the one leaving, then remove ourself from the LS.
        if target_content_id == (self.player_data.character.content_id as u64) {
            let possible_successor;
            {
                let mut db = self.database.lock();
                possible_successor = db.remove_member_from_linkshell(
                    self.player_data.character.content_id,
                    linkshell_id,
                );
            }

            let mut cwlses = vec![(0, CWLSPermissionRank::Invitee); 8];
            if let Some(cwls_memberships) = &mut self.cwls_memberships {
                for (index, cwls) in cwls_memberships.iter_mut().enumerate() {
                    if cwls.ids.linkshell_id == linkshell_id {
                        *cwls = CrossworldLinkshellEx::default();
                        break;
                    }

                    cwlses[index] = (cwls.ids.linkshell_id, cwls.common.rank);
                }
            }

            // Tell the global state about our remaining memberships.
            self.handle
                .send(ToServer::SetLinkshells(
                    self.player_data.character.actor_id,
                    Some(cwlses),
                    None,
                    false,
                ))
                .await;
            // If we were the Master of this linkshell, we need to tell whoever is online that a new Master has been selected.
            if let Some(possible_successor) = possible_successor {
                let target_name;
                {
                    let mut db = self.database.lock();

                    let Some(found_name) = db.find_character_name(possible_successor) else {
                        return;
                    };

                    target_name = found_name;
                }
                self.handle
                    .send(ToServer::SetLinkshellRank(
                        linkshell_id,
                        self.player_data.character.content_id as u64,
                        possible_successor,
                        CWLSPermissionRank::Master,
                        target_name.clone(),
                    ))
                    .await;
            }
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellMemberLeft {
            linkshell_id,
            execute_content_id,
            target_content_id,
            target_homeworld_id: self.config.world_id,
            unk1: 1,
            reason_for_leaving,
            character_name: target_name.clone(),
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn rename_linkshell(&mut self, linkshell_id: u64, name: String) {
        if let Some(cwls_memberships) = &mut self.cwls_memberships {
            let mut has_permission = false;
            for cwls in cwls_memberships.iter_mut() {
                if cwls.ids.linkshell_id == linkshell_id
                    && cwls.common.rank == CWLSPermissionRank::Master
                {
                    has_permission = true;
                    break;
                }
            }

            if !has_permission {
                tracing::warn!(
                    "Client {} attempted to rename linkshell {} without permissions! Rejecting request!",
                    self.player_data.character.content_id,
                    linkshell_id
                );
                return;
            }

            // Otherwise, rename this linkshell.
            let result;
            {
                let mut db = self.database.lock();
                // is_crossworld_ls doesn't matter, we're renaming here
                result = db.create_linkshell(
                    Some(linkshell_id),
                    self.player_data.character.content_id,
                    name.clone(),
                    false,
                );
            }

            // Then tell the global server state so it can inform online members and have them display a notification in-game.
            if result.is_some() {
                self.handle
                    .send(ToServer::RenameLinkshell(
                        self.player_data.character.content_id as u64,
                        self.player_data.character.name.clone(),
                        linkshell_id,
                        name,
                    ))
                    .await;
            }
        }
    }

    pub async fn linkshell_renamed(
        &mut self,
        from_content_id: u64,
        from_name: String,
        linkshell_id: u64,
        linkshell_name: String,
    ) {
        // TODO: get rid of cwls_memberships, it's causing a bunch of headaches
        if let Some(cwls_memberships) = &mut self.cwls_memberships {
            for cwls in cwls_memberships.iter_mut() {
                if cwls.ids.linkshell_id == linkshell_id {
                    cwls.common.name = linkshell_name.clone();
                    break;
                }
            }

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellRenamed {
                linkshell_id,
                content_id: from_content_id,
                home_world_id: self.config.world_id,
                unk1: 1,
                unk2: 0,
                character_name: from_name.clone(),
                new_linkshell_name: linkshell_name.clone(),
            });

            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn set_linkshell_rank(
        &mut self,
        linkshell_id: u64,
        content_id: u64,
        new_rank: CWLSPermissionRank,
    ) {
        let our_content_id = self.player_data.character.content_id as u64;

        // First ensure our client has the proper permissions to do this. And, if they do, set whoever's rank and inform everyone.
        {
            let mut db = self.database.lock();

            let has_leader_perms = db.has_linkshell_permissions(
                our_content_id,
                linkshell_id,
                CWLSPermissionRank::Leader,
            );
            let has_master_perms = db.has_linkshell_permissions(
                our_content_id,
                linkshell_id,
                CWLSPermissionRank::Leader,
            );

            // Leaders can resign, so they're allowed to change their own rank. A Master can only resign by leaving the linkshell, so that is handled elsewhere.
            if our_content_id == content_id && has_leader_perms && !has_master_perms {
                db.set_linkshell_rank(
                    our_content_id,
                    our_content_id,
                    linkshell_id,
                    CWLSPermissionRank::Member, // Don't allow privilege escalation if the client is doing naughty things. The only permitted rank when targeting oneself is Member, and that happens when a Leader is resigning.
                );
            }
            // Otherwise, if this player is targeting another, they need Master permissions to do so.
            else if our_content_id != content_id && has_master_perms {
                db.set_linkshell_rank(our_content_id, content_id, linkshell_id, new_rank);
            } else {
                tracing::warn!(
                    "Client {content_id} attempted to set the rank for another member in linkshell {linkshell_id}, but doesn't have permission! Rejecting request!"
                );
                return;
            }
        }

        let target_name;
        {
            let mut db = self.database.lock();

            let Some(found_name) = db.find_character_name(content_id) else {
                return;
            };

            target_name = found_name;
        }

        self.handle
            .send(ToServer::SetLinkshellRank(
                linkshell_id,
                our_content_id,
                content_id,
                new_rank,
                target_name.clone(),
            ))
            .await;
    }

    pub async fn linkshell_rank_set(
        &mut self,
        linkshell_id: u64,
        execute_content_id: u64,
        target_content_id: u64,
        permission_rank: CWLSPermissionRank,
        target_name: String,
    ) {
        if target_content_id == (self.player_data.character.content_id as u64)
            && let Some(cwls_memberships) = &mut self.cwls_memberships
        {
            for cwls in cwls_memberships {
                if cwls.ids.linkshell_id == linkshell_id {
                    cwls.common.rank = permission_rank;
                    break;
                }
            }
        }

        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellMemberRank {
            linkshell_id,
            execute_content_id,
            target_content_id,
            home_world_id: self.config.world_id,
            unk1: 1,
            permission_rank,
            target_name: target_name.clone(),
        });

        self.send_ipc_self(ipc).await;
    }
}
