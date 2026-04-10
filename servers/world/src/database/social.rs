use super::{WorldDatabase, models, schema};
use crate::{ClassLevels, GameData, PartyMembers, server::PartyMember};
use diesel::prelude::*;
use kawari::{
    common::ClientLanguage,
    config::get_config,
    constants::AVAILABLE_CLASSJOBS,
    ipc::zone::{
        GrandCompany as IpcGrandCompany, OnlineStatus, OnlineStatusMask, PlayerEntry,
        ServerZoneIpcData, SocialListUIFlags, SocialListUILanguages,
    },
};
use std::collections::HashMap;

impl WorldDatabase {
    pub fn commit_parties(&mut self, parties: HashMap<u64, crate::server::Party>) {
        // Delete all existing parties
        diesel::delete(schema::party::dsl::party)
            .execute(&mut self.connection)
            .unwrap();

        for (id, party) in parties {
            let leader = party
                .members
                .iter()
                .find(|x| x.actor_id == party.leader_id)
                .unwrap();

            let party = models::Party {
                id: id as i64,
                leader_content_id: leader.content_id as i64,
                members: PartyMembers(party.members.iter().map(|x| x.content_id as i64).collect()),
            };
            diesel::insert_into(schema::party::dsl::party)
                .values(party)
                .execute(&mut self.connection)
                .unwrap();
        }
    }

    pub fn get_parties(&mut self) -> HashMap<u64, crate::server::Party> {
        let mut parties = HashMap::new();

        use schema::party::dsl::*;
        if let Ok(flat_parties) = party
            .select(models::Party::as_select())
            .load(&mut self.connection)
        {
            for p_party in flat_parties {
                parties.insert(
                    p_party.id as u64,
                    crate::server::Party {
                        members: p_party
                            .members
                            .0
                            .into_iter()
                            .map(|content_id| self.find_party_member(content_id as u64))
                            .collect(),
                        leader_id: self.find_actor_id(p_party.leader_content_id as u64),
                        chatchannel_id: fastrand::u32(..),
                        ..Default::default()
                    },
                );
            }
        }

        parties
    }

    pub fn get_party_entries(
        &mut self,
        game_data: &mut GameData,
        party_id: i64,
    ) -> Vec<PlayerEntry> {
        let found_party;
        {
            use schema::party::dsl::*;

            found_party = party
                .filter(id.eq(party_id))
                .first::<models::Party>(&mut self.connection)
                .unwrap();
        }

        let mut entries = Vec::new();
        for member in &found_party.members.0 {
            entries.push(self.get_player_entry(game_data, *member));
        }

        entries
    }

    pub fn find_party_member(&mut self, for_content_id: u64) -> PartyMember {
        let found_character = self.find_character_ids(Some(for_content_id), None).unwrap();

        let config = get_config();

        PartyMember {
            actor_id: found_character.actor_id,
            content_id: for_content_id,
            world_id: config.world.world_id,
            account_id: found_character.service_account_id as u64,
            name: found_character.name,
            ..Default::default()
        }
    }

    pub fn get_online_player_count(&mut self) -> i64 {
        use schema::volatile::dsl::*;
        volatile
            .select(is_online)
            .filter(is_online.eq(true))
            .count()
            .first::<i64>(&mut self.connection)
            .unwrap_or_default()
    }

    pub fn find_online_players(
        &mut self,
        game_data: &mut GameData,
        for_content_id: i64,
    ) -> Vec<PlayerEntry> {
        let mut online_players = Vec::<PlayerEntry>::new();

        use schema::volatile::dsl::*;
        let online_content_ids: Vec<i64> = volatile
            .filter(is_online.eq(true))
            .select(schema::volatile::dsl::content_id)
            .load(&mut self.connection)
            .unwrap();

        for id in online_content_ids {
            // Don't add ourselves to these results.
            if id == for_content_id {
                continue;
            }

            // Truncate to 200 users maximum, and stop afterward.
            if online_players.len() > 200 {
                online_players.truncate(200);
                break;
            }

            online_players.push(self.get_player_entry(game_data, id));
        }

        online_players
    }

    /// Determine the online status mask, with party/novice/mentor status.
    pub fn determine_online_status_mask(&mut self, for_content_id: i64) -> OnlineStatusMask {
        let mut new_status_mask = OnlineStatusMask::default();

        // Only apply online-related statuses if they're actually online.
        if schema::volatile::dsl::volatile
            .select(schema::volatile::dsl::is_online)
            .filter(schema::volatile::dsl::content_id.eq(for_content_id))
            .first::<bool>(&mut self.connection)
            .unwrap_or_default()
        {
            new_status_mask.set_status(OnlineStatus::Online);

            let parties: Vec<models::Party> = schema::party::dsl::party
                .load(&mut self.connection)
                .unwrap();
            for party in parties {
                if party.members.0.contains(&for_content_id) {
                    if party.leader_content_id == for_content_id {
                        new_status_mask.set_status(OnlineStatus::PartyLeader);
                    }
                    new_status_mask.set_status(OnlineStatus::PartyMember);
                    break;
                }
            }

            // And of course, add the user's chosen status
            new_status_mask.set_status(
                schema::search_info::dsl::search_info
                    .select(schema::search_info::dsl::online_status)
                    .filter(schema::search_info::dsl::content_id.eq(for_content_id))
                    .first::<OnlineStatus>(&mut self.connection)
                    .unwrap(),
            );
        }

        new_status_mask
    }

    pub fn get_search_info(
        &mut self,
        game_data: &mut GameData,
        for_content_id: i64,
    ) -> ServerZoneIpcData {
        let config = get_config();

        let comment = schema::search_info::dsl::search_info
            .select(schema::search_info::dsl::comment)
            .filter(schema::search_info::dsl::content_id.eq(for_content_id))
            .first::<String>(&mut self.connection)
            .unwrap_or_default();

        let levels = schema::classjob::dsl::classjob
            .select(schema::classjob::dsl::levels)
            .filter(schema::classjob::dsl::content_id.eq(for_content_id))
            .first::<ClassLevels>(&mut self.connection)
            .unwrap();

        let mut classjob_levels = [(0u16, 0u16); AVAILABLE_CLASSJOBS];
        for (i, (index, level)) in classjob_levels.iter_mut().enumerate() {
            *index = i as u16 + 1;

            let exp_index = game_data.classjob_exp_indexes[i + 1];
            if exp_index != -1 {
                *level = levels.0[exp_index as usize];
            }
        }

        let grand_company_rank = if let Ok(gc_info) = schema::grand_company::dsl::grand_company
            .select(models::GrandCompany::as_select())
            .filter(schema::grand_company::dsl::content_id.eq(for_content_id))
            .first::<models::GrandCompany>(&mut self.connection)
            && gc_info.active_company != IpcGrandCompany::None
        {
            gc_info.company_ranks.0[gc_info.active_company as usize - 1]
        } else {
            0
        };

        ServerZoneIpcData::OtherSearchInfo {
            content_id: for_content_id as u64,
            unk1: [0; 26],
            world_id: config.world.world_id,
            comment,
            unk2: [0; 157],
            grand_company_rank,
            unk3: [0; 2],
            classjob_levels,
        }
    }

    pub fn get_player_entry(
        &mut self,
        game_data: &mut GameData,
        for_content_id: i64,
    ) -> PlayerEntry {
        let online;
        let online_status_mask;
        let zone_id;
        let client_language;
        let social_ui_languages;
        let has_search_comment;
        let classjob_id;
        let classjob_level;
        let grand_company;
        {
            online_status_mask = self.determine_online_status_mask(for_content_id);

            online = online_status_mask.has_status(OnlineStatus::Online);
            client_language = schema::volatile::dsl::volatile
                .select(schema::volatile::dsl::client_language)
                .filter(schema::volatile::dsl::content_id.eq(for_content_id))
                .first::<ClientLanguage>(&mut self.connection)
                .unwrap();
            zone_id = if online {
                schema::volatile::dsl::volatile
                    .select(schema::volatile::dsl::zone_id)
                    .filter(schema::volatile::dsl::content_id.eq(for_content_id))
                    .first::<i32>(&mut self.connection)
                    .unwrap_or_default() as u16
            } else {
                0
            };

            social_ui_languages = schema::search_info::dsl::search_info
                .select(schema::search_info::dsl::selected_languages)
                .filter(schema::search_info::dsl::content_id.eq(for_content_id))
                .first::<SocialListUILanguages>(&mut self.connection)
                .unwrap();
            has_search_comment = !schema::search_info::dsl::search_info
                .select(schema::search_info::dsl::comment)
                .filter(schema::search_info::dsl::content_id.eq(for_content_id))
                .first::<String>(&mut self.connection)
                .unwrap_or_default()
                .is_empty();

            classjob_id = if online {
                schema::classjob::dsl::classjob
                    .select(schema::classjob::dsl::current_class)
                    .filter(schema::classjob::dsl::content_id.eq(for_content_id))
                    .first::<i32>(&mut self.connection)
                    .unwrap() as u8
            } else {
                0
            };

            classjob_level = if online {
                let index = game_data.classjob_exp_indexes[classjob_id as usize];
                schema::classjob::dsl::classjob
                    .select(schema::classjob::dsl::levels)
                    .filter(schema::classjob::dsl::content_id.eq(for_content_id))
                    .first::<ClassLevels>(&mut self.connection)
                    .unwrap()
                    .0[index as usize] as u8
            } else {
                0
            };

            grand_company = if online {
                schema::grand_company::dsl::grand_company
                    .select(schema::grand_company::dsl::active_company)
                    .filter(schema::grand_company::dsl::content_id.eq(for_content_id))
                    .first::<IpcGrandCompany>(&mut self.connection)
                    .unwrap_or_default()
            } else {
                IpcGrandCompany::None
            };
        }

        let character_name;
        {
            use schema::character::dsl::*;

            character_name = character
                .select(name)
                .filter(content_id.eq(for_content_id))
                .first::<String>(&mut self.connection)
                .unwrap();
        }

        let config = get_config();

        PlayerEntry {
            content_id: for_content_id as u64,
            current_world_id: if online { config.world.world_id } else { 0 },
            ui_flags: SocialListUIFlags::ENABLE_CONTEXT_MENU,
            unk2: [
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                self.friend_request_pending_status(for_content_id), // TODO: this is a bitfield in CS we should support
                0,
            ],
            zone_id,
            client_language,
            social_ui_languages,
            has_search_comment,
            online_status_mask,
            classjob_id,
            classjob_level,
            home_world_id: config.world.world_id,
            name: character_name,
            grand_company,
            ..Default::default()
        }
    }
}
