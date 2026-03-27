mod models;
use std::collections::HashMap;

use kawari::config::get_config;
use kawari::ipc::zone::{
    GameMasterRank, OnlineStatus, ServerZoneIpcData, SocialListUIFlags, SocialListUILanguages,
};
pub use models::{
    AetherCurrent, Aetheryte, Character, ClassJob, Companion, Content, Friends, LinkshellMembers,
    Mentor, Quest, SearchInfo, Unlock, Volatile,
};

mod schema;

use diesel::prelude::*;
use diesel::{Connection, QueryDsl, RunQueryDsl, SqliteConnection};

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use kawari::common::{BasicCharacterData, ClientLanguage, WORLD_NAME, determine_initial_homepoint};

use crate::database::models::Party;
use crate::server::PartyMember;
use crate::{CharaMake, ClassLevels, ClientSelectData, GameData, PartyMembers, RemakeMode};
use crate::{PlayerData, inventory::Inventory};
use kawari::{
    common::ObjectId,
    constants::AVAILABLE_CLASSJOBS,
    ipc::lobby::{CharacterDetails, CharacterFlag},
    ipc::zone::{
        CWLSCommon, CWLSCommonIdentifiers, CWLSMemberListEntry, CWLSNameAvailability,
        CWLSPermissionRank, CrossworldLinkshellEx, OnlineStatusMask, PlayerEntry,
    },
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub struct WorldDatabase {
    connection: SqliteConnection,
}

impl Default for WorldDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl WorldDatabase {
    pub fn new() -> Self {
        let mut connection =
            SqliteConnection::establish("world.db").expect("Failed to open database!");

        connection.run_pending_migrations(MIGRATIONS).unwrap();

        Self { connection }
    }

    pub fn find_player_data(
        &mut self,
        for_actor_id: ObjectId,
        game_data: &mut GameData,
    ) -> PlayerData {
        let found_character;
        {
            use models::*;
            use schema::character::dsl::*;

            found_character = character
                .filter(actor_id.eq(for_actor_id.0 as i64))
                .select(Character::as_select())
                .first(&mut self.connection)
                .unwrap();
        }

        let mut player_data;
        {
            use models::*;

            let volatile = Volatile::belonging_to(&found_character)
                .select(Volatile::as_select())
                .first(&mut self.connection)
                .unwrap();
            let inventory = Inventory::belonging_to(&found_character)
                .select(Inventory::as_select())
                .first(&mut self.connection)
                .unwrap();
            let classjob = ClassJob::belonging_to(&found_character)
                .select(ClassJob::as_select())
                .first(&mut self.connection)
                .unwrap();
            let unlock = Unlock::belonging_to(&found_character)
                .select(Unlock::as_select())
                .first(&mut self.connection)
                .unwrap();
            let customize = Customize::belonging_to(&found_character)
                .select(Customize::as_select())
                .first(&mut self.connection)
                .unwrap();
            let quest = Quest::belonging_to(&found_character)
                .select(Quest::as_select())
                .first(&mut self.connection)
                .unwrap();
            let content = Content::belonging_to(&found_character)
                .select(Content::as_select())
                .first(&mut self.connection)
                .unwrap();
            let companion = Companion::belonging_to(&found_character)
                .select(Companion::as_select())
                .first(&mut self.connection)
                .unwrap();
            let aether_current = AetherCurrent::belonging_to(&found_character)
                .select(AetherCurrent::as_select())
                .first(&mut self.connection)
                .unwrap();
            let aetheryte = Aetheryte::belonging_to(&found_character)
                .select(Aetheryte::as_select())
                .first(&mut self.connection)
                .unwrap();
            let mentor = Mentor::belonging_to(&found_character)
                .select(Mentor::as_select())
                .first(&mut self.connection)
                .unwrap();
            let search_info = SearchInfo::belonging_to(&found_character)
                .select(SearchInfo::as_select())
                .first(&mut self.connection)
                .unwrap();

            player_data = PlayerData {
                character: found_character,
                classjob,
                subrace: customize.chara_make.customize.subrace,
                volatile,
                inventory: serde_json::from_str(&inventory.contents).unwrap(),
                unlock,
                content,
                companion,
                aether_current,
                aetheryte,
                city_state: customize.city_state as u8,
                quest,
                mentor,
                search_info,
                ..Default::default()
            };
        }

        // Before we're finished, we need to populate the items in the inventory with additional static information that we don't bother caching in the db.
        Inventory::prepare_player_inventory(&mut player_data.inventory, game_data);

        player_data
    }

    pub fn commit_classjob(&mut self, data: &PlayerData) {
        use models::*;

        data.classjob
            .save_changes::<ClassJob>(&mut self.connection)
            .unwrap();
    }

    pub fn commit_volatile(&mut self, data: &PlayerData) {
        use models::*;

        data.volatile
            .save_changes::<Volatile>(&mut self.connection)
            .unwrap();
    }

    pub fn commit_search_info(&mut self, data: &PlayerData) {
        use models::*;

        data.search_info
            .save_changes::<SearchInfo>(&mut self.connection)
            .unwrap();
    }

    /// Commit the dynamic player data back to the database
    pub fn commit_player_data(&mut self, data: &PlayerData) {
        use models::*;

        self.commit_volatile(data);

        let inventory = Inventory {
            content_id: data.character.content_id,
            contents: serde_json::to_string(&data.inventory).unwrap(),
        };
        inventory
            .save_changes::<Inventory>(&mut self.connection)
            .unwrap();

        data.character
            .save_changes::<Character>(&mut self.connection)
            .unwrap();
        self.commit_classjob(data);
        data.unlock
            .save_changes::<Unlock>(&mut self.connection)
            .unwrap();
        data.content
            .save_changes::<Content>(&mut self.connection)
            .unwrap();
        data.aetheryte
            .save_changes::<Aetheryte>(&mut self.connection)
            .unwrap();
        data.aether_current
            .save_changes::<AetherCurrent>(&mut self.connection)
            .unwrap();
        data.companion
            .save_changes::<Companion>(&mut self.connection)
            .unwrap();
        data.quest
            .save_changes::<Quest>(&mut self.connection)
            .unwrap();
        data.mentor
            .save_changes::<Mentor>(&mut self.connection)
            .unwrap();
        self.commit_search_info(data);
    }

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

            let party = Party {
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

    pub fn find_party_member(&mut self, for_content_id: u64) -> PartyMember {
        use schema::character::dsl::*;

        let found_character = character
            .filter(content_id.eq(for_content_id as i64))
            .select(models::Character::as_select())
            .first::<Character>(&mut self.connection)
            .unwrap();

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

    pub fn find_actor_id(&mut self, for_content_id: u64) -> ObjectId {
        use schema::character::dsl::*;

        match character
            .filter(content_id.eq(for_content_id as i64))
            .select(actor_id)
            .first::<i64>(&mut self.connection)
        {
            Ok(my_actor_id) => ObjectId(my_actor_id as u32),
            Err(err) => {
                tracing::warn!(
                    "Unable to find {for_content_id}'s actor id in the database due to the following error {err:#?}!"
                );
                ObjectId::default()
            }
        }
    }

    pub fn find_character_name(&mut self, for_content_id: u64) -> Option<String> {
        use schema::character::dsl::*;

        match character
            .filter(content_id.eq(for_content_id as i64))
            .select(name)
            .first::<String>(&mut self.connection)
        {
            Ok(my_name) => Some(my_name),
            Err(err) => {
                tracing::warn!(
                    "Unable to find {for_content_id}'s name in the database due to the following error: {err:#?}!"
                );
                None
            }
        }
    }

    pub fn get_character_list(
        &mut self,
        for_service_account_id: u64,
        world_id: u16,
        game_data: &mut GameData,
    ) -> Vec<CharacterDetails> {
        use models::*;

        // Find the content ids associated with the service account:
        let found_characters;
        {
            use schema::character::dsl::*;

            found_characters = character
                .filter(service_account_id.eq(for_service_account_id as i64))
                .select(Character::as_select())
                .load(&mut self.connection)
                .unwrap_or_default();
        }

        // Put together the list:
        let mut characters = Vec::new();
        for (index, character) in found_characters.iter().enumerate() {
            let volatile = Volatile::belonging_to(&character)
                .select(Volatile::as_select())
                .first(&mut self.connection)
                .unwrap();
            let inventory = Inventory::belonging_to(&character)
                .select(Inventory::as_select())
                .first(&mut self.connection)
                .unwrap();
            let classjob = ClassJob::belonging_to(&character)
                .select(ClassJob::as_select())
                .first(&mut self.connection)
                .unwrap();
            let customize = Customize::belonging_to(&character)
                .select(Customize::as_select())
                .first(&mut self.connection)
                .unwrap();

            let inventory: crate::inventory::Inventory =
                serde_json::from_str(&inventory.contents).unwrap();

            let select_data = ClientSelectData {
                character_name: character.name.clone(),
                current_class: classjob.current_class,
                class_levels: classjob.levels.0.iter().map(|x| *x as i32).collect(),
                race: customize.chara_make.customize.race as i32,
                subrace: customize.chara_make.customize.subrace as i32,
                gender: customize.chara_make.customize.gender as i32,
                birth_month: customize.chara_make.birth_month,
                birth_day: customize.chara_make.birth_day,
                guardian: customize.chara_make.guardian,
                unk8: 0,
                unk9: 0,
                zone_id: volatile.zone_id,
                content_finder_condition: 0,
                customize: customize.chara_make.customize,
                model_main_weapon: inventory.get_main_weapon_id(game_data),
                model_sub_weapon: inventory.get_sub_weapon_id(game_data) as i32,
                model_ids: inventory.get_model_ids(game_data).to_vec(),
                equip_stain: [0; 10].to_vec(),
                glasses: [0; 2].to_vec(),
                remake_mode: RemakeMode::from_repr(customize.remake_mode).unwrap(),
                remake_minutes_remaining: 0,
                voice_id: customize.chara_make.voice_id,
                display_flags: volatile.display_flags,
                unk21: 0,
                world_name: String::new(),
                unk22: 0,
                unk23: 0,
            };

            characters.push(CharacterDetails {
                player_id: character.actor_id.0 as u64, // TODO: not correct
                content_id: character.content_id as u64,
                index: index as u8,
                flags: CharacterFlag::NONE,
                unk1: [255; 6],
                origin_server_id: world_id,
                current_server_id: world_id,
                character_name: character.name.clone(),
                origin_server_name: WORLD_NAME.to_string(),
                current_server_name: WORLD_NAME.to_string(),
                character_detail_json: select_data.to_json(),
                unk2: [255; 16],
                unk3: [4; 5],
            });
        }

        characters
    }

    fn generate_content_id() -> u32 {
        fastrand::u32(..)
    }

    fn generate_actor_id() -> ObjectId {
        ObjectId(fastrand::u32(..))
    }

    /// Gives (content_id, actor_id)
    pub fn create_player_data(
        &mut self,
        service_account_id: u64,
        name: &str,
        chara_make_str: &str,
        city_state: u8,
        zone_id: u16,
        inventory: Inventory,
        game_data: &mut GameData,
    ) -> (u64, ObjectId) {
        use models::*;

        let content_id = Self::generate_content_id();
        let actor_id = Self::generate_actor_id();

        // fill out the initial classjob
        let chara_make = CharaMake::from_json(chara_make_str);
        let mut classjob_levels = ClassLevels::default();

        {
            let index = game_data
                .get_exp_array_index(chara_make.classjob_id as u16)
                .expect("Failed to find EXP array index?!");

            classjob_levels.0[index as usize] = 1; // inital level
        }

        let character = Character {
            content_id: content_id as i64,
            service_account_id: service_account_id as i64,
            actor_id,
            gm_rank: GameMasterRank::Debug,
            name: name.to_string(),
            time_played_minutes: 0,
        };
        diesel::insert_into(schema::character::table)
            .values(character)
            .execute(&mut self.connection)
            .unwrap();

        let classjob = ClassJob {
            content_id: content_id as i64,
            current_class: chara_make.classjob_id,
            levels: classjob_levels.clone(),
            first_class: chara_make.classjob_id,
            ..Default::default()
        };
        diesel::insert_into(schema::classjob::table)
            .values(classjob)
            .execute(&mut self.connection)
            .unwrap();

        let customize = Customize {
            content_id: content_id as i64,
            chara_make,
            city_state: city_state as i32,
            ..Default::default()
        };
        diesel::insert_into(schema::customize::table)
            .values(customize)
            .execute(&mut self.connection)
            .unwrap();

        let quest = Quest {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::quest::table)
            .values(quest)
            .execute(&mut self.connection)
            .unwrap();

        let aetheryte = Aetheryte {
            content_id: content_id as i64,
            homepoint: determine_initial_homepoint(city_state) as i32,
            ..Default::default()
        };
        diesel::insert_into(schema::aetheryte::table)
            .values(aetheryte)
            .execute(&mut self.connection)
            .unwrap();

        let volatile = Volatile {
            content_id: content_id as i64,
            zone_id: zone_id as i32,
            ..Default::default()
        };
        diesel::insert_into(schema::volatile::table)
            .values(volatile)
            .execute(&mut self.connection)
            .unwrap();

        let inventory = Inventory {
            content_id: content_id as i64,
            contents: serde_json::to_string(&inventory).unwrap(),
        };
        diesel::insert_into(schema::inventory::table)
            .values(inventory)
            .execute(&mut self.connection)
            .unwrap();

        let aether_current = AetherCurrent {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::aether_current::table)
            .values(aether_current)
            .execute(&mut self.connection)
            .unwrap();

        let companion = Companion {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::companion::table)
            .values(companion)
            .execute(&mut self.connection)
            .unwrap();

        let content = Content {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::content::table)
            .values(content)
            .execute(&mut self.connection)
            .unwrap();

        let unlock = Unlock {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::unlock::table)
            .values(unlock)
            .execute(&mut self.connection)
            .unwrap();

        let mentor = Mentor {
            content_id: content_id as i64,
            is_novice: 1, // All players are novice by default
            ..Default::default()
        };
        diesel::insert_into(schema::mentor::table)
            .values(mentor)
            .execute(&mut self.connection)
            .unwrap();

        let search_info = SearchInfo {
            content_id: content_id as i64,
            online_status: OnlineStatus::NewAdventurer, // because you're a novice :-)
            selected_languages: SocialListUILanguages::ENGLISH,
            ..Default::default()
        };
        diesel::insert_into(schema::search_info::table)
            .values(search_info)
            .execute(&mut self.connection)
            .unwrap();

        (content_id as u64, actor_id)
    }

    /// Checks if `name` is in the character data table
    pub fn check_is_name_free(&mut self, for_name: &str) -> bool {
        use schema::character::dsl::*;

        character
            .filter(name.eq(for_name))
            .count()
            .first::<i64>(&mut self.connection)
            .unwrap_or_default()
            == 0
    }

    /// Deletes a character and all associated data
    pub fn delete_character(&mut self, for_content_id: u64) {
        {
            use schema::unlock::dsl::*;
            diesel::delete(unlock.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::content::dsl::*;
            diesel::delete(content.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::companion::dsl::*;
            diesel::delete(companion.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::aether_current::dsl::*;
            diesel::delete(aether_current.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::inventory::dsl::*;
            diesel::delete(inventory.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::volatile::dsl::*;
            diesel::delete(volatile.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::aetheryte::dsl::*;
            diesel::delete(aetheryte.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::quest::dsl::*;
            diesel::delete(quest.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::customize::dsl::*;
            diesel::delete(customize.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::classjob::dsl::*;
            diesel::delete(classjob.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::mentor::dsl::*;
            diesel::delete(mentor.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::search_info::dsl::*;
            diesel::delete(search_info.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::friends::dsl::*;
            // Delete linked friends first.
            diesel::delete(friends.filter(friend_content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();

            // Next, delete the user's friend list.
            diesel::delete(friends.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        // Since linkshell management is a little more complex than just deleting all rows with this content id, we do it the slightly slower way. We want orphaned linkshells with zero members to auto-disband.
        // TODO: Implement the ToServer protocol for CustomIpcConnection so we can notify the global server about this character's departures from their linkshells
        if let Some(linkshells) = self.find_linkshells(for_content_id as i64) {
            for linkshell_entry in linkshells {
                if linkshell_entry.is_empty() {
                    continue;
                }

                self.remove_member_from_linkshell(
                    for_content_id as i64,
                    linkshell_entry.ids.linkshell_id,
                );
            }
        }

        // NOTE: The character table should always be last!
        {
            use schema::character::dsl::*;
            diesel::delete(character.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }
    }

    /// Sets the remake mode for a character
    pub fn set_remake_mode(&mut self, for_content_id: u64, mode: RemakeMode) {
        use schema::customize::dsl::*;

        diesel::update(customize.filter(content_id.eq(for_content_id as i64)))
            .set(remake_mode.eq(mode as i32))
            .execute(&mut self.connection)
            .unwrap();
    }

    /// Sets the chara make JSON for a character
    pub fn set_chara_make(&mut self, for_content_id: u64, chara_make_json: &str) {
        use schema::customize::dsl::*;

        diesel::update(customize.filter(content_id.eq(for_content_id as i64)))
            .set(chara_make.eq(chara_make_json))
            .execute(&mut self.connection)
            .unwrap();
    }

    /// Gets the chara make for a character
    pub fn get_chara_make(&mut self, for_content_id: u64) -> CharaMake {
        use schema::customize::dsl::*;

        CharaMake::from_json(
            &customize
                .filter(content_id.eq(for_content_id as i64))
                .select(chara_make)
                .first::<String>(&mut self.connection)
                .unwrap(),
        )
    }

    /// Gets the city state for a character
    pub fn get_city_state(&mut self, for_content_id: u64) -> u8 {
        use schema::customize::dsl::*;

        customize
            .filter(content_id.eq(for_content_id as i64))
            .select(city_state)
            .first::<i32>(&mut self.connection)
            .unwrap() as u8
    }

    /// Deletes all character associated with the service account.
    pub fn delete_characters(&mut self, for_service_account_id: u64) {
        use schema::character::dsl::*;
        let content_ids: Vec<i64> = character
            .filter(service_account_id.eq(for_service_account_id as i64))
            .select(content_id)
            .load(&mut self.connection)
            .unwrap();
        for id in content_ids {
            self.delete_character(id as u64);
        }
    }

    /// Returns surface-level information about all of the characters in the database.
    pub fn request_full_character_list(&mut self) -> String {
        use schema::character::dsl::*;

        if let Ok(users) = character
            .select(models::Character::as_select())
            .load(&mut self.connection)
        {
            let characters: Vec<BasicCharacterData> = users
                .iter()
                .map(|x| BasicCharacterData {
                    content_id: x.content_id as u64,
                    name: x.name.clone(),
                })
                .collect();

            serde_json::to_string(&characters).unwrap_or_default()
        } else {
            String::default()
        }
    }

    /// returns
    pub fn find_service_account(&mut self, for_content_id: u64) -> u64 {
        use schema::character::dsl::*;

        character
            .filter(content_id.eq(for_content_id as i64))
            .select(service_account_id)
            .first::<i64>(&mut self.connection)
            .unwrap_or_default() as u64
    }

    pub fn find_playtime(&mut self, for_content_id: u64) -> i64 {
        use schema::character::dsl::*;

        character
            .filter(content_id.eq(for_content_id as i64))
            .select(time_played_minutes)
            .first::<i64>(&mut self.connection)
            .unwrap_or_default()
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

    fn get_friend_content_ids(&mut self, for_content_id: i64) -> Vec<i64> {
        use schema::friends::dsl::*;
        friends
            .filter(content_id.eq(for_content_id))
            .select(friend_content_id)
            .load(&mut self.connection)
            .unwrap_or_default()
    }

    fn friend_request_is_pending(&mut self, for_content_id: i64) -> bool {
        use schema::friends::dsl::*;
        friends
            .filter(friend_content_id.eq(for_content_id))
            .select(is_pending)
            .first::<bool>(&mut self.connection)
            .unwrap_or_default()
    }

    pub fn accept_friend(&mut self, for_content_id: i64, for_friend_content_id: i64) {
        use schema::friends::dsl::*;

        diesel::update(friends)
            .filter(content_id.eq(for_content_id))
            .filter(friend_content_id.eq(for_friend_content_id))
            .set(is_pending.eq(false))
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn find_friend_list(
        &mut self,
        game_data: &mut GameData,
        for_content_id: i64,
    ) -> Vec<PlayerEntry> {
        let mut friend_entries = Vec::<PlayerEntry>::new();

        let friend_content_ids = self.get_friend_content_ids(for_content_id);

        // If they have no friends, just return an empty list that the zone connection can reuse.
        if friend_content_ids.is_empty() {
            return vec![PlayerEntry::default(); 10];
        }

        for id in friend_content_ids {
            friend_entries.push(self.get_player_entry(game_data, id));
        }
        friend_entries
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

            let parties: Vec<Party> = schema::party::dsl::party
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

        ServerZoneIpcData::OtherSearchInfo {
            content_id: for_content_id as u64,
            unk1: [0; 26],
            world_id: config.world.world_id,
            comment,
            unk2: [0; 160],
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
                if self.friend_request_is_pending(for_content_id) {
                    48
                } else {
                    0
                }, // TODO: this is a bitfield in CS we should support
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
            ..Default::default()
        }
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
                .first::<Party>(&mut self.connection)
                .unwrap();
        }

        let mut entries = Vec::new();
        for member in &found_party.members.0 {
            entries.push(self.get_player_entry(game_data, *member));
        }

        entries
    }

    pub fn add_to_friend_list(&mut self, fwen_content_id: i64, my_content_id: i64) {
        if my_content_id == fwen_content_id {
            tracing::error!(
                "Player with content id {my_content_id} attempted to add themselves to their friend list. Ignoring request."
            );
            return;
        }

        use schema::friends::dsl::*;
        let time = diesel::select(datetime())
            .get_result::<String>(&mut self.connection)
            .unwrap();

        let friend = Friends {
            id: fastrand::i64(..),
            content_id: my_content_id,
            friend_content_id: fwen_content_id,
            group_icon: 0,
            invite_time: time,
            is_pending: true,
        };

        diesel::insert_into(friends)
            .values(friend)
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn remove_from_friend_list(&mut self, fwen_content_id: i64, my_content_id: i64) {
        use schema::friends::dsl::*;

        diesel::delete(
            friends
                .filter(content_id.eq(my_content_id))
                .filter(friend_content_id.eq(fwen_content_id)),
        )
        .execute(&mut self.connection)
        .unwrap();
    }

    /// Returns a HashMap of linkshells for the global server state. NOTE: It does not fill in the members or chatchannel ids, and this is intentional! The global server waits for members to log in and inform it that they belong to a given set linkshells, and the chatchannel ids are decided by the global server itself.
    pub fn find_all_linkshells(&mut self) -> HashMap<u64, crate::server::Linkshell> {
        use schema::linkshells::dsl::*;

        let mut found_linkshells = HashMap::new();

        if let Ok(flat_linkshells) = linkshells
            .select(models::Linkshells::as_select())
            .load(&mut self.connection)
        {
            for linkshell in flat_linkshells {
                found_linkshells.insert(linkshell.id as u64, crate::server::Linkshell::default());
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
                shell.creation_time = shell_info[index].creation_time as u32;
            }

            return Some(ret);
        }

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
        let was_master = self.has_linkshell_permissions(
            for_content_id as u64,
            for_linkshell_id,
            CWLSPermissionRank::Master,
        );

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
            } else if was_master {
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

    pub fn add_member_to_linkshell(
        &mut self,
        for_linkshell_id: i64,
        for_content_id: i64,
        their_rank: CWLSPermissionRank,
        their_invite_time: i64,
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
            let new_member = LinkshellMembers {
                id: next_id,
                content_id: for_content_id,
                linkshell_id: for_linkshell_id,
                invite_time: their_invite_time,
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

    /// Returns true if this player's rank is Leader or Master.
    pub fn has_linkshell_permissions(
        &mut self,
        for_content_id: u64,
        for_linkshell_id: u64,
        required_rank: CWLSPermissionRank,
    ) -> bool {
        use schema::linkshell_members::dsl::*;

        if self.is_in_linkshell(for_content_id, for_linkshell_id)
            && let Ok(my_rank) = linkshell_members
                .select(rank)
                .filter(content_id.eq(for_content_id as i64))
                .filter(linkshell_id.eq(for_linkshell_id as i64))
                .first::<i32>(&mut self.connection)
            && let Some(my_rank) = CWLSPermissionRank::from_repr(my_rank as u8)
        {
            // Master has Leader's permissions and more, so >= is fine.
            return my_rank >= required_rank;
        }

        false
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
                    if self.add_member_to_linkshell(
                        next_id,
                        from_content_id,
                        rank,
                        ls_creation_time,
                    ) {
                        return Some(CrossworldLinkshellEx {
                            ids: CWLSCommonIdentifiers {
                                linkshell_id: next_id as u64,
                                ..Default::default()
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

    pub fn do_cleanup_tasks(&mut self) {
        // Ensure the most volatile aspects of the db are reset to a clean state.
        // We expect these to be "offline" as the initial state elsewhere for things like the online player count and friend lists to function correctly.
        {
            use schema::volatile::dsl::*;

            diesel::update(volatile)
                .set(is_online.eq(false))
                .execute(&mut self.connection)
                .unwrap();
        }

        // Clean up orphaned linkshells with no members that were missed somehow. This should theoretically not happen without manual database edits.
        {
            use schema::linkshell_members::dsl::*;

            for (orphaned_linkshell_id, _) in self.find_all_linkshells() {
                if let Ok(members) = linkshell_members
                    .select(models::LinkshellMembers::as_select())
                    .filter(linkshell_id.eq(orphaned_linkshell_id as i64))
                    .load(&mut self.connection)
                    && members.is_empty()
                {
                    tracing::info!(
                        "Found orphaned linkshell {orphaned_linkshell_id} with zero members, cleaning it up now."
                    );
                    self.remove_linkshell(orphaned_linkshell_id);
                }
            }

            // TODO: Auto-promote new owners in linkshells that don't have owners, which should theoretically not happen without manual database edits.
        }
    }
}

#[declare_sql_function]
extern "SQL" {
    fn datetime() -> diesel::sql_types::Text;
}

#[declare_sql_function]
extern "SQL" {
    fn unixepoch() -> diesel::sql_types::BigInt;
}
