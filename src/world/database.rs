use std::{io::Read, sync::Mutex};

use rusqlite::Connection;
use serde::Deserialize;

use crate::{
    common::{CustomizeData, Position},
    lobby::{
        CharaMake, ClientSelectData, RemakeMode,
        ipc::{CharacterDetails, CharacterFlag},
    },
};

use super::{Inventory, PlayerData};

pub struct WorldDatabase {
    connection: Mutex<Connection>,
}

pub struct CharacterData {
    pub name: String,
    pub chara_make: CharaMake, // probably not the ideal way to store this?
    pub city_state: u8,
    pub position: Position,
    pub zone_id: u16,
}

impl Default for WorldDatabase {
    fn default() -> Self {
        Self::new()
    }
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
            let query = "CREATE TABLE IF NOT EXISTS character_data (content_id INTEGER PRIMARY KEY, name STRING, chara_make STRING, city_state INTEGER, zone_id INTEGER, pos_x REAL, pos_y REAL, pos_z REAL, rotation REAL, inventory STRING);";
            connection.execute(query, ()).unwrap();
        }

        let this = Self {
            connection: Mutex::new(connection),
        };

        // Import any backups
        // NOTE: This won't make sense when service accounts are a real thing, so the functionality will probably be moved
        {
            if let Ok(paths) = std::fs::read_dir("./backups") {
                for path in paths {
                    let path = path.unwrap().path();
                    if path.extension().unwrap() == "zip" {
                        this.import_character(path.to_str().unwrap());
                    }
                }
            }
        }

        this
    }

    fn import_character(&self, path: &str) {
        tracing::info!("Importing character backup {path}...");

        let file = std::fs::File::open(path).unwrap();

        let mut archive = zip::ZipArchive::new(file).unwrap();

        #[derive(Deserialize)]
        struct CharacterJson {
            name: String,
        }

        let character: CharacterJson;
        {
            let mut character_file = archive.by_name("character.json").unwrap();

            let mut json_string = String::new();
            character_file.read_to_string(&mut json_string).unwrap();

            character = serde_json::from_str(&json_string).unwrap();
        }

        if !self.check_is_name_free(&character.name) {
            tracing::warn!("* Skipping since this character already exists.");
            return;
        }

        let charsave_file = archive.by_name("FFXIV_CHARA_01.dat").unwrap();
        let charsave_bytes: Vec<u8> = charsave_file.bytes().map(|x| x.unwrap()).collect();
        let charsave = physis::chardat::CharacterData::from_existing(&charsave_bytes).unwrap();

        let customize = CustomizeData::from(charsave.customize);

        let chara_make = CharaMake {
            customize,
            unk1: 73,
            guardian: 1, // TODO: extract these as well
            birth_month: 1,
            birth_day: 1,
            classjob_id: 5,
            unk2: 1,
        };

        // TODO: extract city-state
        // TODO: import inventory
        self.create_player_data(
            &character.name,
            &chara_make.to_json(),
            2,
            132,
            Inventory::default(),
        );

        tracing::info!("{} added to the world!", character.name);
    }

    pub fn find_player_data(&self, actor_id: u32) -> PlayerData {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT content_id, service_account_id FROM characters WHERE actor_id = ?1")
            .unwrap();
        let (content_id, account_id) = stmt
            .query_row((actor_id,), |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        stmt = connection
            .prepare("SELECT pos_x, pos_y, pos_z, rotation, zone_id, inventory FROM character_data WHERE content_id = ?1")
            .unwrap();
        let (pos_x, pos_y, pos_z, rotation, zone_id, inventory_json): (
            f32,
            f32,
            f32,
            f32,
            u16,
            String,
        ) = stmt
            .query_row((content_id,), |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            })
            .unwrap();

        let inventory = serde_json::from_str(&inventory_json).unwrap();

        PlayerData {
            actor_id,
            content_id,
            account_id,
            position: Position {
                x: pos_x,
                y: pos_y,
                z: pos_z,
            },
            rotation,
            zone_id,
            inventory,
            ..Default::default()
        }
    }

    /// Commit the dynamic player data back to the database
    pub fn commit_player_data(&self, data: &PlayerData) {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("UPDATE character_data SET zone_id=?1, pos_x=?2, pos_y=?3, pos_z=?4, rotation=?5, inventory=?6 WHERE content_id = ?7")
            .unwrap();
        stmt.execute((
            data.zone_id,
            data.position.x,
            data.position.y,
            data.position.z,
            data.rotation,
            serde_json::to_string(&data.inventory).unwrap(),
            data.content_id,
        ))
        .unwrap();
    }

    // TODO: from/to sql int

    pub fn find_actor_id(&self, content_id: u64) -> u32 {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("SELECT actor_id FROM characters WHERE content_id = ?1")
            .unwrap();

        stmt.query_row((content_id,), |row| row.get(0)).unwrap()
    }

    pub fn get_character_list(
        &self,
        service_account_id: u32,
        world_id: u16,
        world_name: &str,
    ) -> Vec<CharacterDetails> {
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
            let mut stmt = connection
                .prepare(
                    "SELECT name, chara_make, zone_id FROM character_data WHERE content_id = ?1",
                )
                .unwrap();

            let result: Result<(String, String, u16), rusqlite::Error> = stmt
                .query_row((content_id,), |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                });

            if let Ok((name, chara_make, zone_id)) = result {
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
                    zone_id: zone_id as i32,
                    content_finder_condition: 0,
                    customize: chara_make.customize,
                    model_main_weapon: 0,
                    model_sub_weapon: 0,
                    unk14: [0; 10],
                    unk15: 0,
                    unk16: 0,
                    remake_mode: RemakeMode::None,
                    remake_minutes_remaining: 0,
                    unk19: 0,
                    unk20: 0,
                    world_name: String::new(),
                    unk22: 0,
                    unk23: 0,
                };

                characters.push(CharacterDetails {
                    actor_id: *actor_id,
                    content_id: *content_id as u64,
                    index: index as u8,
                    flags: CharacterFlag::NONE,
                    unk1: [255; 6],
                    origin_server_id: world_id,
                    current_server_id: world_id,
                    character_name: name.clone(),
                    origin_server_name: world_name.to_string(),
                    current_server_name: world_name.to_string(),
                    character_detail_json: select_data.to_json(),
                    unk2: [255; 16],
                    unk3: [4; 5],
                });
            }
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
    pub fn create_player_data(
        &self,
        name: &str,
        chara_make: &str,
        city_state: u8,
        zone_id: u16,
        inventory: Inventory,
    ) -> (u64, u32) {
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
                "INSERT INTO character_data VALUES (?1, ?2, ?3, ?4, ?5, 0.0, 0.0, 0.0, 0.0, ?6);",
                (
                    content_id,
                    name,
                    chara_make,
                    city_state,
                    zone_id,
                    serde_json::to_string(&inventory).unwrap(),
                ),
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
            .prepare(
                "SELECT name, chara_make, city_state, zone_id, pos_x, pos_y, pos_z FROM character_data WHERE content_id = ?1",
            )
            .unwrap();
        let (name, chara_make_json, city_state, zone_id, pos_x, pos_y, pos_z): (
            String,
            String,
            u8,
            u16,
            f32,
            f32,
            f32,
        ) = stmt
            .query_row((content_id,), |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .unwrap();

        CharacterData {
            name,
            chara_make: CharaMake::from_json(&chara_make_json),
            city_state,
            zone_id,
            position: Position {
                x: pos_x,
                y: pos_y,
                z: pos_z,
            },
        }
    }

    /// Deletes a character and all associated data
    pub fn delete_character(&self, content_id: u64) {
        let connection = self.connection.lock().unwrap();

        let mut stmt = connection
            .prepare("DELETE FROM character_data WHERE content_id = ?1; DELETE FROM characters WHERE content_id = ?1;")
            .unwrap();
        stmt.execute((content_id,)).unwrap();
    }
}
