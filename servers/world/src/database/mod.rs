mod models;
use std::collections::HashMap;

use kawari::config::get_config;
use kawari::ipc::zone::{
    GameMasterRank, OnlineStatus, ServerZoneIpcData, SocialListUIFlags, SocialListUILanguages,
};
pub use models::{
    AetherCurrent, Aetheryte, Character, ClassJob, Companion, Content, Friends, Mentor, Quest,
    SearchInfo, Unlock, Volatile,
};

mod schema;

use diesel::prelude::*;
use diesel::{Connection, QueryDsl, RunQueryDsl, SqliteConnection};

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use kawari::common::{BasicCharacterData, ClientLanguage, WORLD_NAME, determine_initial_homepoint};

use crate::database::models::Party;
use crate::{CharaMake, ClassLevels, ClientSelectData, GameData, PartyMembers, RemakeMode};
use crate::{PlayerData, inventory::Inventory};
use kawari::{
    common::ObjectId,
    constants::AVAILABLE_CLASSJOBS,
    ipc::lobby::{CharacterDetails, CharacterFlag},
    ipc::zone::{OnlineStatusMask, PlayerEntry},
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

    pub fn find_actor_id(&mut self, for_content_id: u64) -> u32 {
        use schema::character::dsl::*;

        character
            .filter(content_id.eq(for_content_id as i64))
            .select(actor_id)
            .first::<i64>(&mut self.connection)
            .unwrap_or_default() as u32
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

    fn generate_actor_id() -> u32 {
        fastrand::u32(..)
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
    ) -> (u64, u32) {
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
            actor_id: ObjectId(actor_id),
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

        if schema::volatile::dsl::volatile
            .select(schema::volatile::dsl::is_online)
            .filter(schema::volatile::dsl::content_id.eq(for_content_id))
            .first::<bool>(&mut self.connection)
            .unwrap_or_default()
        {
            new_status_mask.set_status(OnlineStatus::Online);
        }

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

        // And of course, add the user's chosen status'
        new_status_mask.set_status(
            schema::search_info::dsl::search_info
                .select(schema::search_info::dsl::online_status)
                .filter(schema::search_info::dsl::content_id.eq(for_content_id))
                .first::<OnlineStatus>(&mut self.connection)
                .unwrap(),
        );

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

            classjob_id = schema::classjob::dsl::classjob
                .select(schema::classjob::dsl::current_class)
                .filter(schema::classjob::dsl::content_id.eq(for_content_id))
                .first::<i32>(&mut self.connection)
                .unwrap() as u8;

            let index = game_data.classjob_exp_indexes[classjob_id as usize];

            classjob_level = schema::classjob::dsl::classjob
                .select(schema::classjob::dsl::levels)
                .filter(schema::classjob::dsl::content_id.eq(for_content_id))
                .first::<ClassLevels>(&mut self.connection)
                .unwrap()
                .0[index as usize] as u8;
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
            current_world_id: config.world.world_id,
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

    pub fn do_cleanup_tasks(&mut self) {
        use schema::volatile::dsl::*;

        // Ensure the most volatile aspects of the db are reset to a clean state.
        // We expect these to be "offline" as the initial state elsewhere for things like the online player count and friend lists to function correctly.
        diesel::update(volatile)
            .set(is_online.eq(false))
            .execute(&mut self.connection)
            .unwrap();
    }
}

#[declare_sql_function]
extern "SQL" {
    fn datetime() -> diesel::sql_types::Text;
}
