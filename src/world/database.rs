use std::sync::Mutex;

use rusqlite::Connection;

use crate::{
    WORLD_ID, WORLD_NAME, ZONE_ID,
    lobby::{CharaMake, ClientSelectData, ipc::CharacterDetails},
};

use super::PlayerData;

pub struct WorldDatabase {
    connection: Mutex<Connection>,
}

pub struct CharacterData {
    pub name: String,
    pub chara_make: CharaMake, // probably not the ideal way to store this?
}

impl WorldDatabase {
    pub fn new() -> Self {
        let connection = Connection::open("world.db").expect("Failed to open database!");

        // Create characters table
        {
            let query = "CREATE TABLE IF NOT EXISTS characters (content_id INTEGER PRIMARY KEY, service_account_id INTEGER, actor_id INTEGER);";
            connection.execute(query, ()).unwrap();
        }

        // Create characters data table
        {
            let query = "CREATE TABLE IF NOT EXISTS character_data (content_id INTEGER PRIMARY KEY, name STRING, chara_make STRING);";
            connection.execute(query, ()).unwrap();
        }

        Self {
            connection: Mutex::new(connection),
        }
    }

    pub fn find_player_data(&self, actor_id: u32) -> PlayerData {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT content_id, service_account_id FROM characters WHERE actor_id = ?1")
            .unwrap();
        let (content_id, account_id) = stmt
            .query_row((actor_id,), |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        PlayerData {
            actor_id,
            content_id,
            account_id,
        }
    }

    // TODO: from/to sql int

    pub fn find_actor_id(&self, content_id: u64) -> u32 {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT actor_id FROM characters WHERE content_id = ?1")
            .unwrap();

        stmt.query_row((content_id,), |row| row.get(0)).unwrap()
    }

    pub fn get_character_list(&self, service_account_id: u32) -> Vec<CharacterDetails> {
        let connection = self.connection.lock().unwrap();

        let content_actor_ids: Vec<(u32, u32)>;

        // find the content ids associated with the service account
        {
            let mut stmt = connection
                .prepare(
                    "SELECT content_id, actor_id FROM characters WHERE service_account_id = ?1",
                )
                .unwrap();

            content_actor_ids = stmt
                .query_map((service_account_id,), |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .map(|x| x.unwrap())
                .collect();
        }

        let mut characters = Vec::new();

        for (index, (content_id, actor_id)) in content_actor_ids.iter().enumerate() {
            dbg!(content_id);

            let mut stmt = connection
                .prepare("SELECT name, chara_make FROM character_data WHERE content_id = ?1")
                .unwrap();

            let (name, chara_make): (String, String) = stmt
                .query_row((content_id,), |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap();

            let chara_make = CharaMake::from_json(&chara_make);

            let select_data = ClientSelectData {
                game_name_unk: "Final Fantasy".to_string(),
                current_class: 2,
                class_levels: [5; 30],
                race: chara_make.customize.race as i32,
                subrace: chara_make.customize.subrace as i32,
                gender: chara_make.customize.gender as i32,
                birth_month: chara_make.birth_month,
                birth_day: chara_make.birth_day,
                guardian: chara_make.guardian,
                unk8: 0,
                unk9: 0,
                zone_id: ZONE_ID as i32,
                unk11: 0,
                customize: chara_make.customize,
                unk12: 0,
                unk13: 0,
                unk14: [0; 10],
                unk15: 0,
                unk16: 0,
                legacy_character: 0,
                unk18: 0,
                unk19: 0,
                unk20: 0,
                unk21: String::new(),
                unk22: 0,
                unk23: 0,
            };

            characters.push(CharacterDetails {
                actor_id: *actor_id,
                content_id: *content_id as u64,
                index: index as u32,
                unk1: [0; 16],
                origin_server_id: WORLD_ID,
                current_server_id: WORLD_ID,
                character_name: name.clone(),
                origin_server_name: WORLD_NAME.to_string(),
                current_server_name: WORLD_NAME.to_string(),
                character_detail_json: select_data.to_json(),
                unk2: [0; 20],
            });
        }

        characters
    }

    fn generate_content_id() -> u32 {
        rand::random()
    }

    fn generate_actor_id() -> u32 {
        rand::random()
    }

    /// Gives (content_id, actor_id)
    pub fn create_player_data(&self, name: &str, chara_make: &str) -> (u64, u32) {
        let content_id = Self::generate_content_id();
        let actor_id = Self::generate_actor_id();

        let connection = self.connection.lock().unwrap();

        // insert ids
        connection
            .execute(
                "INSERT INTO characters VALUES (?1, ?2, ?3);",
                (content_id, 0x1, actor_id),
            )
            .unwrap();

        // insert char data
        connection
            .execute(
                "INSERT INTO character_data VALUES (?1, ?2, ?3);",
                (content_id, name, chara_make),
            )
            .unwrap();

        (content_id as u64, actor_id)
    }

    /// Checks if `name` is in the character data table
    pub fn check_is_name_free(&self, name: &str) -> bool {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT content_id FROM character_data WHERE name = ?1")
            .unwrap();

        !stmt.exists((name,)).unwrap()
    }

    pub fn find_chara_make(&self, content_id: u64) -> CharacterData {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT name, chara_make FROM character_data WHERE content_id = ?1")
            .unwrap();
        let (name, chara_make_json): (String, String) = stmt
            .query_row((content_id,), |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        CharacterData {
            name,
            chara_make: CharaMake::from_json(&chara_make_json),
        }
    }
}
