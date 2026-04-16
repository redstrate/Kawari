// ! The party system, including the strategy board, waymarks and target signs. Ready checks are handled in the global server state.
use crate::{ZoneConnection, common::PartyUpdateTargets};
use kawari::{
    common::{ObjectId, ObjectTypeId},
    ipc::chat::{ChatChannel, ChatChannelType},
    ipc::zone::{
        ActorControlCategory, PartyMemberEntry, PartyUpdateStatus, PlayerEntry, ServerZoneIpcData,
        ServerZoneIpcSegment, StrategyBoard, StrategyBoardUpdate, WaymarkPlacementMode,
        WaymarkPosition, WaymarkPreset,
    },
};
impl ZoneConnection {
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
                    member.health_points = 0;
                    member.max_health_points = 0;
                    member.resource_points = 0;
                    member.max_resource_points = 0;
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
}
