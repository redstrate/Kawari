mod models;
pub use models::{AetherCurrent, Aetheryte, Companion, Content, Quest, Unlock};

mod schema;

use diesel::prelude::*;
use diesel::{Connection, QueryDsl, RunQueryDsl, SqliteConnection};

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use kawari::common::{BasicCharacterData, WORLD_NAME};
use kawari::ipc::zone::GameMasterRank;
use serde::Deserialize;

use crate::GameData;
use crate::{PlayerData, inventory::Inventory};
use kawari::{
    common::{
        EquipDisplayFlag, ObjectId, Position,
        workdefinitions::{CharaMake, ClientSelectData, RemakeMode},
    },
    constants::CLASSJOB_ARRAY_SIZE,
    ipc::lobby::{CharacterDetails, CharacterFlag},
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

fn json_unpack<T: for<'a> Deserialize<'a> + Default>(json_str: &str) -> T {
    serde_json::from_str(json_str).unwrap_or_default()
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

            let chara_make = CharaMake::from_json(&customize.chara_make);

            player_data = PlayerData {
                name: found_character.name.clone(),
                subrace: chara_make.customize.subrace,
                actor_id: ObjectId(found_character.actor_id as u32),
                content_id: found_character.content_id as u64,
                account_id: found_character.service_account_id as u64,
                position: Position {
                    x: volatile.pos_x as f32,
                    y: volatile.pos_y as f32,
                    z: volatile.pos_z as f32,
                },
                rotation: volatile.rotation as f32,
                zone_id: volatile.zone_id as u16,
                inventory: serde_json::from_str(&inventory.contents).unwrap(),
                gm_rank: GameMasterRank::from_repr(found_character.gm_rank as u8).unwrap(),
                classjob_id: classjob.classjob_id as u8,
                classjob_levels: json_unpack(&classjob.classjob_levels),
                classjob_exp: json_unpack(&classjob.classjob_exp),
                rested_exp: classjob.rested_exp,
                unlock,
                content,
                companion,
                aether_current,
                aetheryte,
                display_flags: EquipDisplayFlag::from_bits(volatile.display_flags as u16)
                    .unwrap_or_default(),
                city_state: customize.city_state as u8,
                active_quests: json_unpack(&quest.active),
                quest,
                title: volatile.title as u16,
                ..Default::default()
            };
        }

        // Before we're finished, we need to populate the items in the inventory with additional static information that we don't bother caching in the db.
        Inventory::prepare_player_inventory(&mut player_data.inventory, game_data);

        player_data
    }

    /// Commit the dynamic player data back to the database
    pub fn commit_player_data(&mut self, data: &PlayerData) {
        use chrono::Utc;
        use models::*;

        let volatile = Volatile {
            content_id: data.content_id as i64,
            pos_x: data.position.x as f64,
            pos_y: data.position.y as f64,
            pos_z: data.position.z as f64,
            rotation: data.rotation as f64,
            zone_id: data.zone_id as i32,
            display_flags: data.display_flags.bits() as i32,
            title: data.title as i32,
        };
        volatile
            .save_changes::<Volatile>(&mut self.connection)
            .unwrap();

        let inventory = Inventory {
            content_id: data.content_id as i64,
            contents: serde_json::to_string(&data.inventory).unwrap(),
        };
        inventory
            .save_changes::<Inventory>(&mut self.connection)
            .unwrap();

        let time_played_minutes =
            (Utc::now() - data.login_time).num_minutes() + self.find_playtime(data.content_id);

        #[derive(AsChangeset, Identifiable)]
        #[diesel(table_name = schema::character)]
        #[diesel(primary_key(content_id))]
        struct CharacterChanges {
            pub content_id: i64,
            pub time_played_minutes: i64,
        }
        let characterchanges = CharacterChanges {
            content_id: data.content_id as i64,
            time_played_minutes,
        };

        characterchanges
            .save_changes::<Character>(&mut self.connection)
            .unwrap();

        #[derive(AsChangeset, Identifiable)]
        #[diesel(table_name = schema::classjob)]
        #[diesel(primary_key(content_id))]
        struct ClassJobChanges {
            content_id: i64,
            classjob_id: i32,
            classjob_levels: String,
            classjob_exp: String,
            rested_exp: i32,
        }
        let classjob = ClassJobChanges {
            content_id: data.content_id as i64,
            classjob_id: data.classjob_id as i32,
            classjob_levels: serde_json::to_string(&data.classjob_levels).unwrap(),
            classjob_exp: serde_json::to_string(&data.classjob_exp).unwrap(),
            rested_exp: data.rested_exp,
        };
        classjob
            .save_changes::<ClassJob>(&mut self.connection)
            .unwrap();

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

        let mut quest = data.quest.clone();
        quest.active = serde_json::to_string(&data.active_quests).unwrap();
        quest.save_changes::<Quest>(&mut self.connection).unwrap();
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

            let chara_make = CharaMake::from_json(&customize.chara_make);
            let inventory: crate::inventory::Inventory =
                serde_json::from_str(&inventory.contents).unwrap();

            let select_data = ClientSelectData {
                character_name: character.name.clone(),
                current_class: classjob.classjob_id,
                class_levels: json_unpack(&classjob.classjob_levels),
                race: chara_make.customize.race as i32,
                subrace: chara_make.customize.subrace as i32,
                gender: chara_make.customize.gender as i32,
                birth_month: chara_make.birth_month,
                birth_day: chara_make.birth_day,
                guardian: chara_make.guardian,
                unk8: 0,
                unk9: 0,
                zone_id: volatile.zone_id,
                content_finder_condition: 0,
                customize: chara_make.customize,
                model_main_weapon: inventory.get_main_weapon_id(game_data),
                model_sub_weapon: inventory.get_sub_weapon_id(game_data) as i32,
                model_ids: inventory.get_model_ids(game_data).to_vec(),
                equip_stain: [0; 10].to_vec(),
                glasses: [0; 2].to_vec(),
                remake_mode: RemakeMode::from_repr(customize.remake_mode).unwrap(),
                remake_minutes_remaining: 0,
                voice_id: chara_make.voice_id,
                display_flags: EquipDisplayFlag::from_bits(volatile.display_flags as u16)
                    .unwrap_or_default(),
                unk21: 0,
                world_name: String::new(),
                unk22: 0,
                unk23: 0,
            };

            characters.push(CharacterDetails {
                player_id: character.actor_id as u64, // TODO: not correct
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
        let mut classjob_levels = vec![0i32; CLASSJOB_ARRAY_SIZE];

        {
            let index = game_data
                .get_exp_array_index(chara_make.classjob_id as u16)
                .unwrap();

            classjob_levels[index as usize] = 1; // inital level
        }

        let character = Character {
            content_id: content_id as i64,
            service_account_id: service_account_id as i64,
            actor_id: actor_id as i64,
            gm_rank: 90,
            name: name.to_string(),
            time_played_minutes: 0,
        };
        diesel::insert_into(schema::character::table)
            .values(character)
            .execute(&mut self.connection)
            .unwrap();

        let classjob = ClassJob {
            content_id: content_id as i64,
            classjob_id: chara_make.classjob_id,
            classjob_levels: serde_json::to_string(&classjob_levels).unwrap(),
            classjob_exp: serde_json::to_string(&vec![0u32; CLASSJOB_ARRAY_SIZE]).unwrap(),
            first_classjob: chara_make.classjob_id,
            ..Default::default()
        };
        diesel::insert_into(schema::classjob::table)
            .values(classjob)
            .execute(&mut self.connection)
            .unwrap();

        let customize = Customize {
            content_id: content_id as i64,
            chara_make: chara_make_str.to_string(),
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
}
