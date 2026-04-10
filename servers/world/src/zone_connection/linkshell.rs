// ! The linkshell systems. Currently, only cross-world shells are supported.
use super::social::fetch_entries;
use crate::{ToServer, ZoneConnection};
use kawari::{
    common::{LogMessageType, ObjectId},
    ipc::chat::{ChatChannel, ChatChannelType},
    ipc::zone::{
        CWLSCommonIdentifiers, CWLSLeaveReason, CWLSMemberListEntry, CWLSPermissionRank,
        CrossworldLinkshell, CrossworldLinkshellEx, CrossworldLinkshellInvite, ServerZoneIpcData,
        ServerZoneIpcSegment,
    },
};

impl ZoneConnection {
    async fn get_linkshells(&mut self) -> Option<Vec<CrossworldLinkshellEx>> {
        let mut db = self.database.lock();
        db.find_linkshells(self.player_data.character.content_id)
    }

    async fn find_linkshell_permissions(
        &mut self,
        for_linkshell_id: u64,
    ) -> Option<CWLSPermissionRank> {
        let mut db = self.database.lock();
        db.find_linkshell_permissions(
            self.player_data.character.content_id as u64,
            for_linkshell_id,
        )
    }

    async fn is_in_linkshell(&mut self, for_linkshell_id: u64) -> bool {
        let mut db = self.database.lock();
        db.is_in_linkshell(
            self.player_data.character.content_id as u64,
            for_linkshell_id,
        )
    }

    /// Update or refresh our ls/cwls info.
    pub async fn init_linkshells(&mut self) {
        let linkshells = self.get_linkshells().await;

        // Don't bother the server if we're not in any linkshells.
        if let Some(linkshells) = linkshells {
            self.handle
                .send(ToServer::SetLinkshells(
                    self.player_data.character.actor_id,
                    linkshells.iter().map(|m| m.ids.linkshell_id).collect(),
                ))
                .await;
        }
    }

    // TODO: Where else is this sent, if anywhere?
    pub async fn send_crossworld_linkshells(&mut self, detailed: bool) {
        let linkshells = self.get_linkshells().await;

        if detailed {
            // Send a more detailed report about all of the client's cross-world linkshells. Sent when the client opens the CWLS menu. It contains extra information about when the CWLS was founded.
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellsEx {
                linkshells: if let Some(linkshells) = linkshells {
                    linkshells
                } else {
                    vec![CrossworldLinkshellEx::default(); CrossworldLinkshellEx::COUNT]
                },
            });

            self.send_ipc_self(ipc).await;
        } else {
            // Send a (very slightly) less detailed "overview" of cross-world linkshells on login and possibly elsewhere. Probably used so the client can chat without having to open the actual cwls menu.
            let mut cwlses = vec![CrossworldLinkshell::default(); CrossworldLinkshell::COUNT];

            if let Some(linkshells) = linkshells {
                // Our database returns the extended version, so we need to translate it back for the overview.
                for cwls in cwlses.iter_mut().zip(linkshells.iter()) {
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

    pub async fn invite_to_linkshell(&mut self, target_content_id: u64, linkshell_id: u64) {
        let successful_invite;
        let target_actor_id;
        let target_name;
        let linkshell_name;
        {
            let mut db = self.database.lock();

            let Some(our_perms) = db.find_linkshell_permissions(
                self.player_data.character.content_id as u64,
                linkshell_id,
            ) else {
                return;
            };

            if our_perms < CWLSPermissionRank::Leader {
                tracing::warn!(
                    "{} tried to invite {} to linkshell {linkshell_id} without invite permissions! Rejecting request!",
                    self.player_data.character.content_id as u64,
                    target_content_id
                );
                return;
            }
            successful_invite = db.add_member_to_linkshell(
                linkshell_id as i64,
                target_content_id as i64,
                CWLSPermissionRank::Invitee,
            );

            let Some(target_ids) = db.find_character_ids(Some(target_content_id), None) else {
                return;
            };

            target_actor_id = target_ids.actor_id;
            target_name = target_ids.name;
            linkshell_name = db.find_linkshell_name(linkshell_id);
        }

        // Only send the invite if all of our info gathering was successful.
        if successful_invite
            && target_actor_id != ObjectId::default()
            && let Some(linkshell_name) = linkshell_name
        {
            let ipc = CrossworldLinkshellInvite {
                linkshell_id,
                execute_content_id: self.player_data.character.content_id as u64,
                target_content_id,
                execute_world_id: self.config.world_id,
                target_world_id: self.config.world_id,
                unk1: 1,
                unk2: 1,
                linkshell_name: linkshell_name.clone(),
                execute_name: self.player_data.character.name.clone(),
                target_name: target_name.clone(),
            };

            self.handle
                .send(ToServer::SendLinkshellInvite(target_actor_id, ipc))
                .await;
        } else {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::ShowLinkshellError {
                // TODO: Probably display other errors if some other error occurs while adding the member. For now, add_member_to_linkshell only returns a bool...
                log_message: LogMessageType::PlayerAlreadyInYourCWLS as u16,
                unk: 0,
            });

            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn received_linkshell_invite(&mut self, invite_info: CrossworldLinkshellInvite) {
        let ipc =
            ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellInvite(invite_info));

        self.send_ipc_self(ipc).await;
    }

    pub async fn accepted_linkshell_invite(&mut self, linkshell_id: u64) {
        let linkshell_name;
        {
            // Here we bypass permission checks because there's not really a whole lot we can do about that as an invitee.
            let mut db = self.database.lock();
            db.set_linkshell_rank(
                self.player_data.character.content_id as u64,
                self.player_data.character.content_id as u64,
                linkshell_id,
                CWLSPermissionRank::Member,
            );
            linkshell_name = db.find_linkshell_name(linkshell_id);
        }

        if let Some(linkshell_name) = linkshell_name {
            self.handle
                .send(ToServer::AcceptedLinkshellInvite(
                    self.player_data.character.actor_id,
                    linkshell_id,
                    self.player_data.character.content_id as u64,
                    self.player_data.character.name.clone(),
                    linkshell_name.clone(),
                ))
                .await;
        }
    }

    pub async fn member_joined_linkshell(
        &mut self,
        linkshell_id: u64,
        content_id: u64,
        target_name: String,
        linkshell_name: String,
    ) {
        if content_id != self.player_data.character.content_id as u64 {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellJoined2 {
                linkshell_id,
                content_id,
                home_world_id: self.config.world_id,
                unk1: 1,
                target_name: target_name.clone(),
            });

            self.send_ipc_self(ipc).await;
        } else {
            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellJoinedSelf {
                common_ids: CWLSCommonIdentifiers {
                    linkshell_id,
                    linkshell_chat_id: ChatChannel {
                        channel_type: ChatChannelType::CWLinkshell,
                        world_id: 10008,
                        channel_number: linkshell_id as u32,
                    },
                },
                linkshell_name: linkshell_name.clone(),
            });

            self.send_ipc_self(ipc).await;
        }
    }

    /// Creates a new cross-world linkshell and then informs both the global server & the client about it.
    pub async fn create_crossworld_linkshell(&mut self, name: String) {
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
            self.init_linkshells().await;

            let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::NewCrossworldLinkshell {
                ids: info.ids.clone(),
                unk_timestamp1: info.creation_time,
                unk_timestamp2: info.creation_time,
                common: info.common.clone(),
            });

            self.send_ipc_self(ipc).await;
        }
    }

    pub async fn disband_linkshell(&mut self, linkshell_id: u64) {
        let our_rank;
        let linkshell_name;
        {
            let mut db = self.database.lock();

            let Some(rank) = db.find_linkshell_permissions(
                self.player_data.character.content_id as u64,
                linkshell_id,
            ) else {
                return;
            };

            our_rank = rank;

            let Some(ls_name) = db.find_linkshell_name(linkshell_id) else {
                return;
            };

            linkshell_name = ls_name;

            if our_rank == CWLSPermissionRank::Master {
                db.remove_linkshell(linkshell_id);
            } else {
                tracing::warn!(
                    "Client {} tried to disband linkshell {} with permission_rank {:#?}! Rejecting request!",
                    self.player_data.character.content_id,
                    linkshell_id,
                    our_rank
                );
                return;
            }
        }

        self.handle
            .send(ToServer::DisbandLinkshell(
                linkshell_id,
                linkshell_name.clone(),
            ))
            .await;
    }

    pub async fn crossworld_linkshell_disbanded(
        &mut self,
        linkshell_id: u64,
        linkshell_name: String,
    ) {
        // Inform the client.
        let ipc = ServerZoneIpcSegment::new(ServerZoneIpcData::CrossworldLinkshellDisbanded {
            linkshell_id,
            name: linkshell_name.to_string(),
        });

        self.send_ipc_self(ipc).await;
    }

    pub async fn remove_linkshell_member(
        &mut self,
        linkshell_id: u64,
        target_content_id: u64,
        reason_for_leaving: CWLSLeaveReason,
    ) {
        let target_ids;
        {
            let mut db = self.database.lock();

            // If we're kicking someone, check permissions first.
            let Some(our_perms) = db.find_linkshell_permissions(
                self.player_data.character.content_id as u64,
                linkshell_id,
            ) else {
                return;
            };

            let Some(their_perms) = db.find_linkshell_permissions(target_content_id, linkshell_id)
            else {
                return;
            };

            let mut gamedata = self.gamedata.lock();
            let Some(members) = db.find_linkshell_members(linkshell_id, &mut gamedata) else {
                return;
            };

            // Only allow kicking if our permissions are at least Leader, and theirs are Member or lower.
            if target_content_id != (self.player_data.character.content_id as u64) {
                if our_perms < CWLSPermissionRank::Leader {
                    tracing::warn!(
                        "Player {} tried to kick {} from linkshell {}, but did not have permission to do so! Rejecting request!",
                        self.player_data.character.content_id as u64,
                        target_content_id,
                        linkshell_id
                    );
                    return;
                } else if their_perms > CWLSPermissionRank::Member {
                    tracing::warn!(
                        "Player {} tried to kick {} from linkshell {}, but their target is rank {:#?}! Rejecting request!",
                        self.player_data.character.content_id as u64,
                        target_content_id,
                        linkshell_id,
                        their_perms
                    );
                    return;
                }
            }
            // This might not really be necessary, but it's to guard against a client doing naughty things like bypassing an error which prevents them from resigning as Master when they're the only member left. In that situation, their only option normally is to disband the linkshell, so we reject this invalid request.
            else if target_content_id == (self.player_data.character.content_id as u64)
                && our_perms == CWLSPermissionRank::Master
                && members.len() == 1
            {
                tracing::warn!(
                    "Player {target_content_id} tried to resign as Master from linkshell {linkshell_id}, but they're the only member! This shouldn't happen! Rejecting request!"
                );
                return;
            }

            target_ids = db.find_character_ids(Some(target_content_id), None);
        }
        if let Some(ref target_ids) = target_ids
            && target_ids.actor_id.is_valid()
        {
            self.handle
                .send(ToServer::LeaveLinkshell(
                    target_ids.actor_id,
                    self.player_data.character.content_id as u64,
                    target_content_id,
                    target_ids.name.clone(),
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

            // If we were the Master of this linkshell, we need to tell whoever is online that a new Master has been selected.
            if let Some(possible_successor) = possible_successor {
                let target_name;
                {
                    let mut db = self.database.lock();

                    let Some(found_name) = db.find_character_ids(Some(possible_successor), None)
                    else {
                        return;
                    };

                    target_name = found_name.name;
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
        if let Some(our_rank) = self.find_linkshell_permissions(linkshell_id).await
            && our_rank == CWLSPermissionRank::Master
        {
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
        } else {
            tracing::warn!(
                "Client {} attempted to rename linkshell {} without permissions! Rejecting request!",
                self.player_data.character.content_id,
                linkshell_id
            );
        }
    }

    pub async fn linkshell_renamed(
        &mut self,
        from_content_id: u64,
        from_name: String,
        linkshell_id: u64,
        linkshell_name: String,
    ) {
        if self.is_in_linkshell(linkshell_id).await {
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

            let Some(our_perms) = db.find_linkshell_permissions(our_content_id, linkshell_id)
            else {
                return;
            };

            // Leaders can resign, so they're allowed to change their own rank. A Master can only resign by leaving the linkshell, so that is handled elsewhere.
            if our_content_id == content_id && our_perms == CWLSPermissionRank::Leader {
                db.set_linkshell_rank(
                    our_content_id,
                    our_content_id,
                    linkshell_id,
                    CWLSPermissionRank::Member, // Don't allow privilege escalation if the client is doing naughty things. The only permitted rank when targeting oneself is Member, and that happens when a Leader is resigning.
                );
            }
            // Otherwise, if this player is targeting another, they need Master permissions to do so.
            else if our_content_id != content_id && our_perms == CWLSPermissionRank::Master {
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

            let Some(target_ids) = db.find_character_ids(Some(content_id), None) else {
                return;
            };

            target_name = target_ids.name;
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
