use serde::Deserialize;
use std::{io::BufReader, io::Read};

use crate::{
    common::{CustomizeData, GameData, workdefinitions::CharaMake},
    inventory::{Inventory, Item, Storage},
    world::WorldDatabase,
};

pub enum ImportError {
    CharacterExists,
    ReadError,
    ParseError,
    MissingData,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let message = match self {
            ImportError::CharacterExists => "Character already exists",
            ImportError::ReadError => "Error while reading files",
            ImportError::ParseError => "Error while parsing files",
            ImportError::MissingData => {
                "Not all required data can be found. You need to use the Dalamud integration in Auracite!"
            }
        };

        write!(f, "{message}")
    }
}

impl WorldDatabase {
    /// Imports a character from an Auracite backup at `path`.
    pub fn import_character(
        &self,
        game_data: &mut GameData,
        service_account_id: u64,
        path: &str,
    ) -> Result<(), ImportError> {
        tracing::info!("Importing character backup from {path}...");

        let file = std::fs::File::open(path).unwrap();

        let mut archive = zip::ZipArchive::new(file).unwrap();

        #[derive(Deserialize)]
        struct GenericValue {
            value: i32,
        }

        #[derive(Deserialize)]
        struct NamedayValue {
            day: i32,
            month: i32,
        }

        #[derive(Deserialize)]
        struct ClassJobLevelValue {
            level: i32,
            exp: Option<u32>,
            value: i32,
        }

        #[derive(Deserialize)]
        struct InventoryItem {
            slot: i32,
            quantity: u32,
            condition: u16,
            id: u32,
            glamour_id: u32,
        }

        #[derive(Deserialize)]
        struct InventoryContainer {
            items: Vec<InventoryItem>,
        }

        #[derive(Deserialize)]
        struct CharacterJson {
            name: String,
            city_state: GenericValue,
            nameday: NamedayValue,
            guardian: GenericValue,
            voice: i32,
            classjob_levels: Vec<ClassJobLevelValue>,

            inventory1: InventoryContainer,
            inventory2: InventoryContainer,
            inventory3: InventoryContainer,
            inventory4: InventoryContainer,
            equipped_items: InventoryContainer,

            currency: InventoryContainer,

            armory_off_hand: InventoryContainer,
            armory_head: InventoryContainer,
            armory_body: InventoryContainer,
            armory_hands: InventoryContainer,
            armory_legs: InventoryContainer,
            armory_ear: InventoryContainer,
            armory_neck: InventoryContainer,
            armory_wrist: InventoryContainer,
            armory_rings: InventoryContainer,
            armory_soul_crystal: InventoryContainer,
            armory_main_hand: InventoryContainer,

            unlock_flags: Vec<u8>,
            unlock_aetherytes: Vec<u8>,
        }

        let character: CharacterJson;
        {
            let mut character_file = archive
                .by_name("character.json")
                .map_err(|_| ImportError::ReadError)?;

            let mut json_string = String::new();
            character_file
                .read_to_string(&mut json_string)
                .map_err(|_| ImportError::ReadError)?;

            character = serde_json::from_str(&json_string).map_err(|_| ImportError::MissingData)?;
        }

        if !self.check_is_name_free(&character.name) {
            return Err(ImportError::CharacterExists);
        }

        let charsave_file = archive
            .by_name("FFXIV_CHARA_01.dat")
            .map_err(|_| ImportError::MissingData)?;
        let mut charsave_bytes = Vec::<u8>::new();
        let mut bufrdr = BufReader::new(charsave_file);
        if bufrdr.read_to_end(&mut charsave_bytes).is_err() {
            return Err(ImportError::ReadError);
        };

        let charsave = physis::savedata::chardat::CharacterData::from_existing(&charsave_bytes)
            .ok_or(ImportError::ParseError)?;

        let customize = CustomizeData::from(charsave.customize);

        let chara_make = CharaMake {
            customize,
            voice_id: character.voice,
            guardian: character.guardian.value,
            birth_month: character.nameday.month,
            birth_day: character.nameday.day,
            classjob_id: 5,
            unk2: 1,
        };

        let (_, actor_id) = self.create_player_data(
            service_account_id,
            &character.name,
            &chara_make.to_json(),
            character.city_state.value as u8,
            132,
            Inventory::default(),
            game_data,
        );

        let mut player_data = self.find_player_data(actor_id, game_data);

        // import jobs
        for classjob in &character.classjob_levels {
            // find the array index of the job
            let index = game_data
                .get_exp_array_index(classjob.value as u16)
                .ok_or(ImportError::ParseError)?;

            player_data.classjob_levels[index as usize] = classjob.level as u16;
            if let Some(exp) = classjob.exp {
                player_data.classjob_exp[index as usize] = exp as i32;
            }
        }

        let process_inventory_container =
            |container: &InventoryContainer, target: &mut dyn Storage| {
                for item in &container.items {
                    if item.slot as u32 > target.max_slots() {
                        continue;
                    }
                    *target.get_slot_mut(item.slot as u16) = Item {
                        quantity: item.quantity,
                        id: item.id,
                        condition: item.condition,
                        glamour_catalog_id: item.glamour_id,
                        ..Default::default()
                    };
                }
            };

        // import inventory
        process_inventory_container(&character.inventory1, &mut player_data.inventory.pages[0]);
        process_inventory_container(&character.inventory2, &mut player_data.inventory.pages[1]);
        process_inventory_container(&character.inventory3, &mut player_data.inventory.pages[2]);
        process_inventory_container(&character.inventory4, &mut player_data.inventory.pages[3]);
        process_inventory_container(
            &character.equipped_items,
            &mut player_data.inventory.equipped,
        );

        process_inventory_container(&character.currency, &mut player_data.inventory.currency);

        process_inventory_container(
            &character.armory_off_hand,
            &mut player_data.inventory.armoury_off_hand,
        );
        process_inventory_container(
            &character.armory_head,
            &mut player_data.inventory.armoury_head,
        );
        process_inventory_container(
            &character.armory_body,
            &mut player_data.inventory.armoury_body,
        );
        process_inventory_container(
            &character.armory_hands,
            &mut player_data.inventory.armoury_hands,
        );
        process_inventory_container(
            &character.armory_legs,
            &mut player_data.inventory.armoury_legs,
        );
        process_inventory_container(
            &character.armory_ear,
            &mut player_data.inventory.armoury_earring,
        );
        process_inventory_container(
            &character.armory_neck,
            &mut player_data.inventory.armoury_necklace,
        );
        process_inventory_container(
            &character.armory_wrist,
            &mut player_data.inventory.armoury_bracelet,
        );
        process_inventory_container(
            &character.armory_rings,
            &mut player_data.inventory.armoury_rings,
        );
        process_inventory_container(
            &character.armory_soul_crystal,
            &mut player_data.inventory.armoury_soul_crystal,
        );
        process_inventory_container(
            &character.armory_main_hand,
            &mut player_data.inventory.armoury_main_hand,
        );

        // import unlock flags
        player_data.unlocks.unlocks = character.unlock_flags.into();
        player_data.unlocks.aetherytes = character.unlock_aetherytes.into();

        self.commit_player_data(&player_data);

        Ok(())
    }
}
