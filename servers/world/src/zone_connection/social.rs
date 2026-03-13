//! The party system and other social features.

use crate::{ToServer, ZoneConnection, common::PartyUpdateTargets};
use kawari::{
    common::{ObjectId, ObjectTypeId, timestamp_secs},
    ipc::{
        chat::{ChatChannel, ChatChannelType},
        zone::{
            ActorControlCategory, InviteReply, InviteType, InviteUpdateType, OnlineStatus,
            OnlineStatusMask, PartyMemberEntry, PartyUpdateStatus, PlayerEntry,
            SearchUIGrandCompanies, ServerZoneIpcData, ServerZoneIpcSegment, SocialList,
            SocialListRequestType, SocialListUILanguages, StrategyBoard, StrategyBoardUpdate,
            WaymarkPlacementMode, WaymarkPosition, WaymarkPreset,
        },
    },
};

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
        // only party supported for now
        if invite_type != InviteType::Party {
            return;
        }

        if response == InviteReply::Accepted {
            self.handle
                .send(ToServer::AddPartyMember(
                    self.party_id,
                    self.player_data.character.actor_id,
                    from_content_id,
                ))
                .await;
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
        if let Some((party_id, chatchannel_id, leader_actor_id, mut party_list)) = party_info {
            if self.party_id == 0 {
                self.party_id = party_id;
            }

            let member_count = party_list.len() as u8;

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
                member_count: 0,
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
            unk1: 0,
            unk2: 0,
            unk3: 0,
        });

        self.send_ipc_self(ipc).await;

        // TODO:
        // after party update they send the status effect list
        // after the status effect list they send updateclassinfo

        // Ensure our online status is updated, since that is affected by whether we're in a party etc.
        self.update_online_status().await;
    }

    pub async fn send_social_list(
        &mut self,
        request_type: SocialListRequestType,
        sequence: u8,
        entries: Vec<PlayerEntry>,
    ) {
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SocialList(SocialList {
            // TODO: Fill these in once we support more social list types
            community_id: 0,
            current_index: 0,
            next_index: 0,
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
        let mut new_status_mask = OnlineStatusMask::default();
        new_status_mask.set_status(OnlineStatus::Online);

        if self.party_id != 0 {
            if self.is_party_leader {
                new_status_mask.set_status(OnlineStatus::PartyLeader);
            }
            new_status_mask.set_status(OnlineStatus::PartyMember);
        }

        new_status_mask.set_status(self.player_data.search_info.online_status);

        new_status_mask
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
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::SetOnlineStatus(
            self.get_online_status_mask(),
        ));
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
        _classjobs: u64,
        _minimum_level: u16,
        _maximum_level: u16,
        _grand_company: SearchUIGrandCompanies,
        _languages: SocialListUILanguages,
        _online_status: OnlineStatusMask,
        _areas: [u16; 50],
        _name: String,
    ) {
        // For now, no results were found until we implement it properly.
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::SearchPlayersResult { num_results: 0 });
        self.send_ipc_self(ipc).await;
    }
}
