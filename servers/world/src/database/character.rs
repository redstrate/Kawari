use diesel::prelude::*;

use super::{Character, WorldDatabase, models, schema};
use crate::{
    CharaMake, ClassLevels, ClientSelectData, GameData, GrandCompanyRanks, PlayerData, RemakeMode,
    inventory::Inventory,
};
use kawari::{
    common::{
        BasicCharacterData, ObjectId, WORLD_NAME, WeaponModelId, determine_initial_homepoint,
    },
    config::get_config,
    ipc::{
        lobby::{CharacterDetails, CharacterFlag},
        zone::{
            GameMasterRank, GrandCompany as IpcGrandCompany, OnlineStatus, ServerZoneIpcData,
            ServerZoneIpcSegment, SocialListUILanguages,
        },
    },
};

impl WorldDatabase {
    /// Returns a row from the Character table, searching either with a content id or a character's name. When sending tells, the ChatConnection is only given a name from the game client, so it needs to pull data in this fashion.
    // TODO: What's a better name for this function?
    pub fn find_character_ids(
        &mut self,
        for_content_id: Option<u64>,
        for_name: Option<String>,
    ) -> Option<Character> {
        use schema::character::dsl::*;
        if let Some(for_content_id) = for_content_id
            && let Ok(data) = character
                .filter(content_id.eq(for_content_id as i64))
                .select(Character::as_select())
                .first(&mut self.connection)
        {
            return Some(data);
        } else if let Some(for_name) = for_name
            && let Ok(data) = character
                .filter(name.eq(for_name))
                .select(Character::as_select())
                .first(&mut self.connection)
        {
            return Some(data);
        }

        None
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
            let grand_company = GrandCompany::belonging_to(&found_character)
                .select(GrandCompany::as_select())
                .first(&mut self.connection)
                .unwrap();
            let glamour = Glamour::belonging_to(&found_character)
                .select(Glamour::as_select())
                .first(&mut self.connection)
                .unwrap_or_default();
            let plate = AdventurerPlate::belonging_to(&found_character)
                .select(AdventurerPlate::as_select())
                .first(&mut self.connection)
                .unwrap_or_default();

            player_data = PlayerData {
                character: found_character,
                classjob,
                subrace: customize.chara_make.customize.tribe as u8,
                volatile,
                inventory: inventory.contents,
                unlock,
                content,
                companion,
                aether_current,
                aetheryte,
                city_state: customize.city_state as u8,
                quest,
                mentor,
                search_info,
                grand_company,
                glamour: glamour.contents,
                plate: plate.contents,
                ..Default::default()
            };
        }

        // Before we're finished, we need to populate the items in the inventory with additional static information that we don't bother caching in the db.
        Inventory::prepare_player_inventory(&mut player_data.inventory, game_data);

        player_data
    }

    /// Saves the classjob and inventory tables to the database. This is always done in lockstep as your current classjob and equipped inventory are closely related.
    pub fn commit_classjob_and_inventory(&mut self, data: &PlayerData) {
        use models::*;

        data.classjob
            .save_changes::<ClassJob>(&mut self.connection)
            .unwrap();

        let inventory = Inventory {
            content_id: data.character.content_id,
            contents: data.inventory.clone(),
        };
        inventory
            .save_changes::<Inventory>(&mut self.connection)
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

    pub fn commit_grand_companies(&mut self, data: &PlayerData) {
        use models::*;

        data.grand_company
            .save_changes::<GrandCompany>(&mut self.connection)
            .unwrap();
    }

    pub fn commit_glamour(&mut self, data: &PlayerData) {
        use models::*;
        use schema::glamour::dsl::content_id;

        let glamour = Glamour {
            content_id: data.character.content_id,
            contents: data.glamour.clone(),
        };
        // Upsert instead of `save_changes` (an UPDATE) so characters created before the
        // `glamour` table existed — and therefore have no row yet — get one on first commit
        // rather than panicking with `NotFound`.
        diesel::insert_into(schema::glamour::table)
            .values(&glamour)
            .on_conflict(content_id)
            .do_update()
            .set(&glamour)
            .execute(&mut self.connection)
            .unwrap();
    }

    pub fn commit_plate(&mut self, data: &PlayerData) {
        use models::*;
        use schema::adventurer_plate::dsl::content_id;

        let plate = AdventurerPlate {
            content_id: data.character.content_id,
            contents: data.plate.clone(),
        };
        // Upsert instead of `save_changes` (an UPDATE) so characters created before the
        // `adventurer_plate` table existed — and therefore have no row yet — get one on first
        // commit rather than panicking with `NotFound`.
        diesel::insert_into(schema::adventurer_plate::table)
            .values(&plate)
            .on_conflict(content_id)
            .do_update()
            .set(&plate)
            .execute(&mut self.connection)
            .unwrap();
    }

    /// Flags a character's adventurer plate as reset by a Fantasia (the snapshot portrait no
    /// longer matches the re-customized character). Sets `flags & 1` on the stored design
    /// without otherwise clearing the plate; no-op if the character has no plate row or has
    /// never saved a plate.
    pub fn mark_plate_reset_by_fantasia(&mut self, for_content_id: u64) {
        use models::*;
        use schema::adventurer_plate::dsl::content_id;

        let mut plate = match schema::adventurer_plate::table
            .filter(content_id.eq(for_content_id as i64))
            .select(AdventurerPlate::as_select())
            .first(&mut self.connection)
        {
            Ok(plate) => plate,
            // No row yet (character predates the table or was never loaded) — nothing to reset.
            Err(_) => return,
        };

        plate.contents.mark_reset_by_fantasia();

        plate
            .save_changes::<AdventurerPlate>(&mut self.connection)
            .unwrap();
    }

    /// Commit the dynamic player data back to the database
    pub fn commit_player_data(&mut self, data: &PlayerData) {
        use models::*;

        self.commit_volatile(data);
        data.character
            .save_changes::<Character>(&mut self.connection)
            .unwrap();
        self.commit_classjob_and_inventory(data);
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
        data.grand_company
            .save_changes::<GrandCompany>(&mut self.connection)
            .unwrap();
        self.commit_glamour(data);
        self.commit_plate(data);
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

            let select_data = ClientSelectData {
                character_name: character.name.clone(),
                current_class: classjob.current_class,
                class_levels: classjob.levels.0.iter().map(|x| *x as i32).collect(),
                race: customize.chara_make.customize.race as i32,
                subrace: customize.chara_make.customize.tribe as i32,
                gender: customize.chara_make.customize.gender as i32,
                birth_month: customize.chara_make.birth_month,
                birth_day: customize.chara_make.birth_day,
                guardian: customize.chara_make.guardian,
                unk8: 0,
                unk9: 0,
                zone_id: volatile.zone_id,
                content_finder_condition: 0,
                customize: customize.chara_make.customize,
                model_main_weapon: inventory.contents.get_main_weapon_id(game_data).into(),
                model_sub_weapon: <WeaponModelId as Into<u64>>::into(
                    inventory.contents.get_sub_weapon_id(game_data),
                ) as i32,
                model_ids: inventory
                    .contents
                    .legacy_model_ids(game_data)
                    .map(|x| x.into())
                    .to_vec(),
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
                flags: CharacterFlag::empty(),
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
            contents: inventory,
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
        let grand_company = GrandCompany {
            content_id: content_id as i64,
            active_company: IpcGrandCompany::None,
            company_ranks: GrandCompanyRanks::default(),
        };
        diesel::insert_into(schema::grand_company::table)
            .values(grand_company)
            .execute(&mut self.connection)
            .unwrap();

        let glamour = Glamour {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::glamour::table)
            .values(glamour)
            .execute(&mut self.connection)
            .unwrap();

        let plate = AdventurerPlate {
            content_id: content_id as i64,
            ..Default::default()
        };
        diesel::insert_into(schema::adventurer_plate::table)
            .values(plate)
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

        {
            use schema::grand_company::dsl::*;
            diesel::delete(grand_company.filter(content_id.eq(for_content_id as i64)))
                .execute(&mut self.connection)
                .unwrap();
        }

        {
            use schema::adventurer_plate::dsl::*;
            diesel::delete(adventurer_plate.filter(content_id.eq(for_content_id as i64)))
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

        self.remove_all_letters(for_content_id);

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

    pub fn find_playtime(&mut self, for_content_id: u64) -> i64 {
        use schema::character::dsl::*;

        character
            .filter(content_id.eq(for_content_id as i64))
            .select(time_played_minutes)
            .first::<i64>(&mut self.connection)
            .unwrap_or_default()
    }

    /// Builds the adventurer plate response for `for_actor_id`, as requested by the player with
    /// `viewer_content_id`.
    ///
    /// Returns a full `AdventurerPlate` when the plate is visible to the viewer, or a
    /// `RequestAdventurerPlateError` (a LogMessage the client shows before closing the plate
    /// window) when the target has no plate or has restricted its visibility. A player can always
    /// see their own plate, even before saving one for the first time.
    pub fn lookup_adventurer_plate(
        &mut self,
        for_actor_id: ObjectId,
        viewer_content_id: u64,
    ) -> ServerZoneIpcSegment {
        use models::*;

        let for_character;
        {
            use schema::character::dsl::*;

            for_character = character
                .filter(actor_id.eq(for_actor_id))
                .select(Character::as_select())
                .first(&mut self.connection)
                .unwrap();
        }

        let plate = AdventurerPlate::belonging_to(&for_character)
            .select(AdventurerPlate::as_select())
            .first(&mut self.connection)
            .unwrap_or_default()
            .contents;

        let is_self = for_character.content_id as u64 == viewer_content_id;

        // Decide whether the viewer is allowed to see this plate. The owner always can.
        if !is_self {
            if !plate.has_plate {
                return Self::adventurer_plate_error(LOG_MESSAGE_PLATE_NOT_SET);
            }
            let design = plate.design();
            // `flags & 2` == visible to no one (only yourself).
            let only_self = design.flags & 2 != 0;
            // `privacy_flags & 1` == friends only.
            let friends_only = design.privacy_flags & 1 != 0;
            let is_friend =
                self.are_friends(viewer_content_id, for_character.content_id as u64);
            if only_self || (friends_only && !is_friend) {
                return Self::adventurer_plate_error(LOG_MESSAGE_PLATE_NOT_PUBLIC);
            }
        }

        // Visible: echo the persisted design snapshot, merging in the live header fields that are
        // not part of the submitted design block (name, comment, grand company, world, etc.).
        let mut design = plate.design();

        let grand_company = GrandCompany::belonging_to(&for_character)
            .select(GrandCompany::as_select())
            .first(&mut self.connection)
            .unwrap();
        let search_info = SearchInfo::belonging_to(&for_character)
            .select(SearchInfo::as_select())
            .first(&mut self.connection)
            .unwrap();
        let classjob = ClassJob::belonging_to(&for_character)
            .select(ClassJob::as_select())
            .first(&mut self.connection)
            .unwrap();

        // When the owner opens a plate they have never saved, the stored design is only a set of
        // default style values with no character snapshot (job/customize/gear are all zero). A
        // plate with `class_job_id == 0` and empty customize/gear cannot be rendered by the
        // client, so the plate window silently fails to open. Fill the snapshot fields from the
        // character's live data so the first-time editor shows the current appearance — this
        // mirrors how retail initializes a fresh plate.
        if is_self && !plate.has_plate {
            let inventory = Inventory::belonging_to(&for_character)
                .select(Inventory::as_select())
                .first(&mut self.connection)
                .unwrap();
            let customize = Customize::belonging_to(&for_character)
                .select(Customize::as_select())
                .first(&mut self.connection)
                .unwrap();
            let equipped = &inventory.contents.equipped;

            design.class_job_id = classjob.current_class as u8;
            design.customize = customize.chara_make.customize;
            design.stain_ids1 = [
                equipped.main_hand.stains[0],
                equipped.off_hand.stains[0],
                equipped.head.stains[0],
                equipped.body.stains[0],
                equipped.hands.stains[0],
                equipped.legs.stains[0],
                equipped.feet.stains[0],
                equipped.ears.stains[0],
                equipped.neck.stains[0],
                equipped.wrists.stains[0],
                equipped.left_ring.stains[0],
                equipped.right_ring.stains[0],
            ];
            design.stain_ids2 = [
                equipped.main_hand.stains[1],
                equipped.off_hand.stains[1],
                equipped.head.stains[1],
                equipped.body.stains[1],
                equipped.hands.stains[1],
                equipped.legs.stains[1],
                equipped.feet.stains[1],
                equipped.ears.stains[1],
                equipped.neck.stains[1],
                equipped.wrists.stains[1],
                equipped.left_ring.stains[1],
                equipped.right_ring.stains[1],
            ];
            // Use the apparent (glamoured) id so glamoured gear shows its glamour on the plate,
            // matching what the player sees on their character.
            design.item_ids = [
                equipped.main_hand.apparent_id(),
                equipped.off_hand.apparent_id(),
                equipped.head.apparent_id(),
                equipped.body.apparent_id(),
                equipped.hands.apparent_id(),
                equipped.legs.apparent_id(),
                equipped.feet.apparent_id(),
                equipped.ears.apparent_id(),
                equipped.neck.apparent_id(),
                equipped.wrists.apparent_id(),
                equipped.right_ring.apparent_id(),
                equipped.left_ring.apparent_id(),
            ];
        }

        // Ranks are 1-indexed by the active company; a character with no grand company
        // has `active_company == None` (0), so guard against underflowing the array index.
        let grand_company_rank = if grand_company.active_company != IpcGrandCompany::None {
            grand_company.company_ranks.0[grand_company.active_company as usize - 1]
        } else {
            0
        };

        let config = get_config();
        ServerZoneIpcSegment::new(ServerZoneIpcData::AdventurerPlate {
            unk1: 0,
            unk2: 0,
            unk3: 0,
            unk4: 0,
            content_id: for_character.content_id as u64,
            actor_id: for_actor_id,
            unk5: 2,
            world_id: config.world.world_id,
            favored_class_level: 100,
            favored_class: classjob.current_class as u8,
            unk7: 1,
            grand_company: grand_company.active_company,
            grand_company_rank,
            design,
            comment: search_info.comment.clone(),
            name: for_character.name.clone(),
        })
    }

    /// Builds the "plate request error" response (not set / not visible / unavailable) carrying
    /// the given LogMessage row id.
    fn adventurer_plate_error(log_message_id: u32) -> ServerZoneIpcSegment {
        ServerZoneIpcSegment::new(ServerZoneIpcData::RequestAdventurerPlateError {
            log_message_id,
            unk1: 0,
            padding: [0; 16],
        })
    }

    /// Returns whether two characters are mutual (non-pending) friends.
    fn are_friends(&mut self, content_id_a: u64, content_id_b: u64) -> bool {
        use schema::friends::dsl::*;

        friends
            .filter(content_id.eq(content_id_a as i64))
            .filter(friend_content_id.eq(content_id_b as i64))
            .filter(is_pending.eq(0))
            .count()
            .first::<i64>(&mut self.connection)
            .unwrap_or(0)
            > 0
    }
}

/// LogMessage row id shown when a plate has never been set up.
const LOG_MESSAGE_PLATE_NOT_SET: u32 = 5856;
/// LogMessage row id shown when a plate exists but is not visible to the viewer.
const LOG_MESSAGE_PLATE_NOT_PUBLIC: u32 = 5858;
