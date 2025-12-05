use parking_lot::Mutex;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::zone_connection::PersistentQuest;
use kawari::{
    common::{
        EquipDisplayFlag, GameData, ItemInfoQuery, ObjectId, Position,
        workdefinitions::{CharaMake, ClientSelectData, RemakeMode},
    },
    constants::CLASSJOB_ARRAY_SIZE,
    ipc::lobby::{CharacterDetails, CharacterFlag},
};

use super::{
    PlayerData,
    inventory::{Inventory, Storage},
    zone_connection::UnlockData,
};

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

#[derive(Serialize, Deserialize)]
pub struct BasicCharacterData {
    pub content_id: u64,
    pub name: String,
}

impl Default for WorldDatabase {
    fn default() -> Self {
        Self::new()
    }
}

fn json_unpack<T: for<'a> Deserialize<'a>>(json_str: String) -> T {
    serde_json::from_str(&json_str).unwrap()
}

impl WorldDatabase {
    pub fn new() -> Self {
        let connection = Connection::open("world.db").expect("Failed to open database!");

        // Create characters table
        {
            let query = "CREATE TABLE IF NOT EXISTS character (content_id INTEGER PRIMARY KEY, service_account_id INTEGER, actor_id INTEGER);";
            connection.execute(query, ()).unwrap();
        }

        // Create characters data table
        {
            let query = "CREATE TABLE IF NOT EXISTS character_data
                (content_id INTEGER PRIMARY KEY,
                name STRING,
                chara_make STRING,
                city_state INTEGER,
                zone_id INTEGER,
                pos_x REAL,
                pos_y REAL,
                pos_z REAL,
                rotation REAL,
                inventory STRING,
                remake_mode INTEGER,
                gm_rank INTEGER,
                classjob_id INTEGER,
                classjob_levels STRING,
                classjob_exp STRING,
                unlocks STRING,
                display_flags INTEGER,
                active_quests STRING);";
            connection.execute(query, ()).unwrap();
        }

        Self {
            connection: Mutex::new(connection),
        }
    }

    pub fn find_player_data(&self, actor_id: ObjectId, game_data: &mut GameData) -> PlayerData {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT content_id, service_account_id FROM character WHERE actor_id = ?1")
            .unwrap();
        let (content_id, account_id): (u64, u64) = stmt
            .query_row((actor_id.0,), |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();

        stmt = connection
            .prepare(
                "SELECT pos_x,
                     pos_y,
                     pos_z,
                     rotation,
                     zone_id,
                     inventory,
                     gm_rank,
                     classjob_id,
                     classjob_levels,
                     classjob_exp,
                     unlocks,
                     display_flags,
                     city_state,
                     active_quests
                     FROM character_data WHERE content_id = ?1",
            )
            .unwrap();
        let mut player_data: PlayerData = stmt
            .query_row((content_id,), |row| {
                Ok(PlayerData {
                    actor_id,
                    content_id,
                    account_id,
                    position: Position {
                        x: row.get(0)?,
                        y: row.get(1)?,
                        z: row.get(2)?,
                    },
                    rotation: row.get(3)?,
                    zone_id: row.get(4)?,
                    inventory: row.get(5)?,
                    gm_rank: row.get(6)?,
                    classjob_id: row.get(7)?,
                    classjob_levels: json_unpack(row.get(8)?),
                    classjob_exp: json_unpack(row.get(9)?),
                    unlocks: json_unpack(row.get(10)?),
                    display_flags: EquipDisplayFlag::from_bits(row.get(11)?).unwrap_or_default(),
                    city_state: row.get(12)?,
                    active_quests: json_unpack(row.get(13)?),
                    ..Default::default()
                })
            })
            .unwrap();

        // Before we're finished, we need to populate the items in the inventory with additional static information that we don't bother caching in the db.
        self.prepare_player_inventory(&mut player_data.inventory, game_data);

        player_data
    }

    // TODO: Should this and prepare_player_inventory be instead placed somewhere in the inventory modules?
    fn prepare_items_in_container(&self, container: &mut impl Storage, data: &mut GameData) {
        for index in 0..container.max_slots() {
            let item = container.get_slot_mut(index as u16);

            if item.is_empty_slot() {
                continue;
            }

            if let Some(info) = data.get_item_info(ItemInfoQuery::ById(item.id)) {
                item.item_level = info.item_level;
                item.stack_size = info.stack_size;
                item.price_low = info.price_low;
                // TODO: There will be much more in the future.
            }
        }
    }

    fn prepare_player_inventory(&self, inventory: &mut Inventory, data: &mut GameData) {
        // TODO: implement iter_mut for Inventory so all of this can be reduced down
        for index in 0..inventory.pages.len() {
            self.prepare_items_in_container(&mut inventory.pages[index], data);
        }

        self.prepare_items_in_container(&mut inventory.equipped, data);
        self.prepare_items_in_container(&mut inventory.armoury_main_hand, data);
        self.prepare_items_in_container(&mut inventory.armoury_body, data);
        self.prepare_items_in_container(&mut inventory.armoury_hands, data);
        self.prepare_items_in_container(&mut inventory.armoury_legs, data);
        self.prepare_items_in_container(&mut inventory.armoury_feet, data);
        self.prepare_items_in_container(&mut inventory.armoury_off_hand, data);
        self.prepare_items_in_container(&mut inventory.armoury_earring, data);
        self.prepare_items_in_container(&mut inventory.armoury_necklace, data);
        self.prepare_items_in_container(&mut inventory.armoury_bracelet, data);
        self.prepare_items_in_container(&mut inventory.armoury_rings, data);
        // Skip soul crystals
    }

    /// Commit the dynamic player data back to the database
    pub fn commit_player_data(&self, data: &PlayerData) {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare(
                "UPDATE character_data SET
                        zone_id=?1,
                        pos_x=?2,
                        pos_y=?3,
                        pos_z=?4,
                        rotation=?5,
                        inventory=?6,
                        classjob_id=?7,
                        classjob_levels=?8,
                        classjob_exp=?9,
                        unlocks=?10,
                        display_flags=?11,
                        active_quests=?12
                        WHERE content_id = ?13",
            )
            .unwrap();
        stmt.execute(rusqlite::params![
            data.zone_id,
            data.position.x,
            data.position.y,
            data.position.z,
            data.rotation,
            serde_json::to_string(&data.inventory).unwrap(),
            data.classjob_id,
            serde_json::to_string(&data.classjob_levels).unwrap(),
            serde_json::to_string(&data.classjob_exp).unwrap(),
            serde_json::to_string(&data.unlocks).unwrap(),
            data.display_flags.0,
            serde_json::to_string(&data.active_quests).unwrap(),
            data.content_id,
        ])
        .unwrap();
    }

    // TODO: from/to sql int

    pub fn find_actor_id(&self, content_id: u64) -> u32 {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT actor_id FROM character WHERE content_id = ?1")
            .unwrap();

        stmt.query_row((content_id,), |row| row.get(0)).unwrap()
    }

    pub fn get_character_list(
        &self,
        service_account_id: u64,
        world_id: u16,
        world_name: &str,
        game_data: &mut GameData,
    ) -> Vec<CharacterDetails> {
        let connection = self.connection.lock();

        let content_actor_ids: Vec<(u32, u32)>;

        // find the content ids associated with the service account
        {
            let mut stmt = connection
                .prepare("SELECT content_id, actor_id FROM character WHERE service_account_id = ?1")
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
                    "SELECT name, chara_make, zone_id, inventory, remake_mode, classjob_id, classjob_levels, display_flags FROM character_data WHERE content_id = ?1",
                )
                .unwrap();

            struct CharaListQuery {
                name: String,
                chara_make: CharaMake,
                zone_id: u16,
                inventory: Inventory,
                remake_mode: RemakeMode,
                classjob_id: i32,
                classjob_levels: Vec<i32>,
                display_flags: u16,
            }

            let result: Result<CharaListQuery, rusqlite::Error> =
                stmt.query_row((content_id,), |row| {
                    Ok(CharaListQuery {
                        name: row.get(0)?,
                        chara_make: row.get(1)?,
                        zone_id: row.get(2)?,
                        inventory: row.get(3)?,
                        remake_mode: row.get(4)?,
                        classjob_id: row.get(5)?,
                        classjob_levels: json_unpack(row.get(6)?),
                        display_flags: row.get(7)?,
                    })
                });

            if let Ok(query) = result {
                let select_data = ClientSelectData {
                    character_name: query.name.clone(),
                    current_class: query.classjob_id,
                    class_levels: query.classjob_levels,
                    race: query.chara_make.customize.race as i32,
                    subrace: query.chara_make.customize.subrace as i32,
                    gender: query.chara_make.customize.gender as i32,
                    birth_month: query.chara_make.birth_month,
                    birth_day: query.chara_make.birth_day,
                    guardian: query.chara_make.guardian,
                    unk8: 0,
                    unk9: 0,
                    zone_id: query.zone_id as i32,
                    content_finder_condition: 0,
                    customize: query.chara_make.customize,
                    model_main_weapon: query.inventory.get_main_weapon_id(game_data),
                    model_sub_weapon: query.inventory.get_sub_weapon_id(game_data) as i32,
                    model_ids: query.inventory.get_model_ids(game_data).to_vec(),
                    equip_stain: [0; 10].to_vec(),
                    glasses: [0; 2].to_vec(),
                    remake_mode: query.remake_mode,
                    remake_minutes_remaining: 0,
                    voice_id: query.chara_make.voice_id,
                    display_flags: EquipDisplayFlag::from_bits(query.display_flags)
                        .unwrap_or_default(),
                    unk21: 0,
                    world_name: String::new(),
                    unk22: 0,
                    unk23: 0,
                };

                characters.push(CharacterDetails {
                    player_id: *actor_id as u64, // TODO: not correct
                    content_id: *content_id as u64,
                    index: index as u8,
                    flags: CharacterFlag::NONE,
                    unk1: [255; 6],
                    origin_server_id: world_id,
                    current_server_id: world_id,
                    character_name: query.name.clone(),
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
        fastrand::u32(..)
    }

    fn generate_actor_id() -> u32 {
        fastrand::u32(..)
    }

    /// Gives (content_id, actor_id)
    pub fn create_player_data(
        &self,
        service_account_id: u64,
        name: &str,
        chara_make_str: &str,
        city_state: u8,
        zone_id: u16,
        inventory: Inventory,
        game_data: &mut GameData,
    ) -> (u64, u32) {
        let content_id = Self::generate_content_id();
        let actor_id = Self::generate_actor_id();

        let connection = self.connection.lock();

        // fill out the initial classjob
        let chara_make = CharaMake::from_json(chara_make_str);
        let mut classjob_levels = vec![0i32; CLASSJOB_ARRAY_SIZE];

        {
            let index = game_data
                .get_exp_array_index(chara_make.classjob_id as u16)
                .unwrap();

            classjob_levels[index as usize] = 1; // inital level
        }

        let classjob_exp = vec![0u32; CLASSJOB_ARRAY_SIZE];

        // insert ids
        connection
            .execute(
                "INSERT INTO character VALUES (?1, ?2, ?3);",
                (content_id, service_account_id, actor_id),
            )
            .unwrap();

        // insert char data
        connection
            .execute(
                "INSERT INTO character_data VALUES (?1, ?2, ?3, ?4, ?5, 0.0, 0.0, 0.0, 0.0, ?6, 0, 90, ?7, ?8, ?9, ?10, 0, ?11);",
                (
                    content_id,
                    name,
                    chara_make_str,
                    city_state,
                    zone_id,
                    serde_json::to_string(&inventory).unwrap(),
                    chara_make.classjob_id,
                    serde_json::to_string(&classjob_levels).unwrap(),
                    serde_json::to_string(&classjob_exp).unwrap(),
                    serde_json::to_string(&UnlockData::default()).unwrap(),
                    serde_json::to_string::<Vec<PersistentQuest>>(&Vec::default()).unwrap(),
                ),
            )
            .unwrap();

        (content_id as u64, actor_id)
    }

    /// Checks if `name` is in the character data table
    pub fn check_is_name_free(&self, name: &str) -> bool {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT content_id FROM character_data WHERE name = ?1")
            .unwrap();

        !stmt.exists((name,)).unwrap()
    }

    pub fn find_chara_make(&self, content_id: u64) -> CharacterData {
        let connection = self.connection.lock();

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
        let connection = self.connection.lock();

        // delete data
        {
            let mut stmt = connection
                .prepare("DELETE FROM character_data WHERE content_id = ?1")
                .unwrap();
            stmt.execute((content_id,)).unwrap();
        }

        // delete char
        {
            let mut stmt = connection
                .prepare("DELETE FROM character WHERE content_id = ?1")
                .unwrap();
            stmt.execute((content_id,)).unwrap();
        }
    }

    /// Sets the remake mode for a character
    pub fn set_remake_mode(&self, content_id: u64, mode: RemakeMode) {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("UPDATE character_data SET remake_mode=?1 WHERE content_id = ?2")
            .unwrap();
        stmt.execute((mode as i32, content_id)).unwrap();
    }

    /// Sets the chara make JSON for a character
    pub fn set_chara_make(&self, content_id: u64, chara_make_json: &str) {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("UPDATE character_data SET chara_make=?1 WHERE content_id = ?2")
            .unwrap();
        stmt.execute((chara_make_json, content_id)).unwrap();
    }

    /// Deletes all character associated with the service account.
    pub fn delete_characters(&self, service_account_id: u64) {
        let connection = self.connection.lock();

        let content_actor_ids: Vec<(u32, u32)>;

        // find the content ids associated with the service account
        {
            let mut stmt = connection
                .prepare("SELECT content_id, actor_id FROM character WHERE service_account_id = ?1")
                .unwrap();

            content_actor_ids = stmt
                .query_map((service_account_id,), |row| Ok((row.get(0)?, row.get(1)?)))
                .unwrap()
                .map(|x| x.unwrap())
                .collect();
        }

        for (content_id, _actor_id) in content_actor_ids {
            // delete from characters table
            connection
                .execute("DELETE FROM character WHERE content_id = ?1", (content_id,))
                .unwrap();

            // delete from character_data table
            connection
                .execute(
                    "DELETE FROM character_data WHERE content_id = ?1",
                    (content_id,),
                )
                .unwrap();
        }
    }

    /// Returns surface-level information about all of the characters in the database.
    pub fn request_full_character_list(&self) -> String {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT content_id, name FROM character_data")
            .unwrap();

        let characters: Vec<BasicCharacterData> = stmt
            .query_map((), |row| {
                Ok(BasicCharacterData {
                    content_id: row.get(0)?,
                    name: row.get(1)?,
                })
            })
            .unwrap()
            .map(|x| x.unwrap())
            .collect();

        serde_json::to_string(&characters).unwrap_or_default()
    }

    /// returns
    pub fn find_service_account(&self, content_id: u64) -> u64 {
        let connection = self.connection.lock();

        let mut stmt = connection
            .prepare("SELECT service_account_id FROM character WHERE content_id = ?1")
            .unwrap();
        stmt.query_row((content_id,), |row| row.get(0)).unwrap()
    }
}
