use glam::Vec3;
use serde::Deserialize;
use std::io::Read;

use crate::{
    CharaMake, GameData, WorldDatabase,
    inventory::{Inventory, Item, Storage},
};
use kawari::{
    common::{CustomizeData, Position},
    ipc::zone::GrandCompany,
};

#[derive(Deserialize)]
struct NameValue {
    value: i32,
}

#[derive(Deserialize)]
struct DayMonthValue {
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
    id: u32,
    crafter_content_id: u64,
    item_flags: u8,
    condition: u16,
    spiritbond_or_collectability: u16,
    glamour_id: u32,
    materia: Vec<u16>,
    materia_grades: Vec<u8>,
    stains: Vec<u8>,
}

#[derive(Deserialize)]
struct InventoryContainer {
    items: Vec<InventoryItem>,
}

#[derive(Deserialize)]
struct Appearance {
    model_type: i32,
    height: i32,
    face_type: i32,
    hair_style: i32,
    has_highlights: bool,
    skin_color: i32,
    eye_color: i32,
    hair_color: i32,
    hair_color2: i32,
    face_features: i32,
    face_features_color: i32,
    eyebrows: i32,
    eye_color2: i32,
    eye_shape: i32,
    nose_shape: i32,
    jaw_shape: i32,
    lip_style: i32,
    lip_color: i32,
    race_feature_size: i32,
    race_feature_type: i32,
    bust_size: i32,
    facepaint: i32,
    facepaint_color: i32,
}

#[derive(Deserialize)]
struct CharacterJson {
    name: String,
    city_state: NameValue,
    nameday: DayMonthValue,
    guardian: NameValue,
    gender: NameValue,
    tribe: NameValue,
    race: NameValue,
    classjob_levels: Vec<ClassJobLevelValue>,
    grand_company: NameValue,
    grand_company_ranks: Vec<u8>,
    title: NameValue,
    voice: i32,

    is_battle_mentor: bool,
    is_trade_mentor: bool,
    is_novice: bool,
    is_returner: bool,

    appearance: Appearance,

    inventory1: InventoryContainer,
    inventory2: InventoryContainer,
    inventory3: InventoryContainer,
    inventory4: InventoryContainer,
    equipped: InventoryContainer,

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

    // unlocks
    unlocks: Vec<u8>,
    seen_active_help: Vec<u8>,
    minions: Vec<u8>,
    mounts: Vec<u8>,
    orchestrion_rolls: Vec<u8>,
    cutscene_seen: Vec<u8>,
    ornaments: Vec<u8>,
    caught_fish: Vec<u8>,
    caught_spearfish: Vec<u8>,
    adventures: Vec<u8>,
    triple_triad_cards: Vec<u8>,
    glasses_styles: Vec<u8>,
    chocobo_taxi_stands: Vec<u8>,
    titles: Vec<u8>,
    unlocked_companion_equip: Vec<u8>,

    // aether currents
    comp_flg_set: Vec<u8>,
    unlocked_aether_currents: Vec<u8>,

    // aetheryte
    unlocked_aetherytes: Vec<u8>,
    homepoint: i32,
    favorite_aetherytes: Vec<u16>,
    free_aetheryte: i32,

    // classjob
    current_class: i32,
    first_class: i32,
    rested_exp: i32,

    // content
    unlocked_special_content: Vec<u8>,
    unlocked_raids: Vec<u8>,
    unlocked_dungeons: Vec<u8>,
    unlocked_guildhests: Vec<u8>,
    unlocked_trials: Vec<u8>,
    unlocked_crystalline_conflicts: Vec<u8>,
    unlocked_frontlines: Vec<u8>,
    cleared_raids: Vec<u8>,
    cleared_dungeons: Vec<u8>,
    cleared_guildhests: Vec<u8>,
    cleared_trials: Vec<u8>,
    cleared_crystalline_conflicts: Vec<u8>,
    cleared_frontlines: Vec<u8>,
    cleared_masked_carnivale: Vec<u8>,
    unlocked_misc_content: Vec<u8>,
    cleared_misc_content: Vec<u8>,

    // quest
    completed_quests: Vec<u8>,

    // volatile
    position_x: f32,
    position_y: f32,
    position_z: f32,
    rotation: f32,
    zone_id: u16,
}

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
        &mut self,
        game_data: &mut GameData,
        service_account_id: u64,
        path: &str,
    ) -> Result<(), ImportError> {
        tracing::info!("Importing character backup from {path}...");

        let file = std::fs::File::open(path).unwrap();

        let mut archive = zip::ZipArchive::new(file).unwrap();

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

        let customize = CustomizeData {
            race: character.race.value as u8,
            gender: character.gender.value as u8,
            age: character.appearance.model_type as u8,
            height: character.appearance.height as u8,
            subrace: character.tribe.value as u8,
            face: character.appearance.face_type as u8,
            hair: character.appearance.hair_style as u8,
            enable_highlights: character.appearance.has_highlights as u8,
            skin_tone: character.appearance.skin_color as u8,
            right_eye_color: character.appearance.eye_color2 as u8,
            hair_tone: character.appearance.hair_color as u8,
            highlights: character.appearance.hair_color2 as u8,
            facial_features: character.appearance.face_features as u8,
            facial_feature_color: character.appearance.face_features_color as u8,
            eyebrows: character.appearance.eyebrows as u8,
            left_eye_color: character.appearance.eye_color as u8,
            eyes: character.appearance.eye_shape as u8,
            nose: character.appearance.nose_shape as u8,
            jaw: character.appearance.jaw_shape as u8,
            mouth: character.appearance.lip_style as u8,
            lips_tone_fur_pattern: character.appearance.lip_color as u8,
            race_feature_size: character.appearance.race_feature_size as u8,
            race_feature_type: character.appearance.race_feature_type as u8,
            bust: character.appearance.bust_size as u8,
            face_paint: character.appearance.facepaint as u8,
            face_paint_color: character.appearance.facepaint_color as u8,
        };

        let chara_make = CharaMake {
            customize,
            voice_id: character.voice,
            guardian: character.guardian.value,
            birth_month: character.nameday.month,
            birth_day: character.nameday.day,
            classjob_id: character.current_class,
            unk2: 1,
        };

        let (_, actor_id) = self.create_player_data(
            service_account_id,
            &character.name,
            &chara_make.to_json(),
            character.city_state.value as u8,
            character.zone_id,
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

            player_data.classjob.levels.0[index as usize] = classjob.level as u16;
            if let Some(exp) = classjob.exp {
                player_data.classjob.exp.0[index as usize] = exp as i32;
            }
        }

        player_data.grand_company.active_company =
            GrandCompany::from_repr(character.grand_company.value as usize).unwrap_or_default();
        player_data.grand_company.company_ranks.0 = character.grand_company_ranks;
        player_data.volatile.title = character.title.value;

        // mentor status
        player_data.mentor.is_battle = character.is_battle_mentor as i32;
        player_data.mentor.is_trade = character.is_trade_mentor as i32;
        player_data.mentor.is_novice = character.is_novice as i32;
        player_data.mentor.is_returner = character.is_returner as i32;

        let process_inventory_container =
            |container: &InventoryContainer, target: &mut dyn Storage| {
                for item in &container.items {
                    if item.slot as u32 > target.max_slots() {
                        continue;
                    }
                    *target.get_slot_mut(item.slot as u16) = Item {
                        quantity: item.quantity,
                        item_id: item.id,
                        crafter_content_id: item.crafter_content_id,
                        item_flags: item.item_flags,
                        condition: item.condition,
                        spiritbond_or_collectability: item.spiritbond_or_collectability,
                        glamour_id: item.glamour_id,
                        materia: item.materia.clone().try_into().unwrap_or_default(),
                        materia_grades: item.materia_grades.clone().try_into().unwrap_or_default(),
                        stains: item.stains.clone().try_into().unwrap_or_default(),
                        ..Default::default()
                    };
                }
            };

        // import inventory
        process_inventory_container(&character.inventory1, &mut player_data.inventory.pages[0]);
        process_inventory_container(&character.inventory2, &mut player_data.inventory.pages[1]);
        process_inventory_container(&character.inventory3, &mut player_data.inventory.pages[2]);
        process_inventory_container(&character.inventory4, &mut player_data.inventory.pages[3]);
        process_inventory_container(&character.equipped, &mut player_data.inventory.equipped);

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

        // unlocks
        player_data.unlock.unlocks.data = character.unlocks.clone();
        player_data.unlock.seen_active_help.data = character.seen_active_help.clone();
        player_data.unlock.minions.data = character.minions.clone();
        player_data.unlock.mounts.data = character.mounts.clone();
        player_data.unlock.orchestrion_rolls.data = character.orchestrion_rolls.clone();
        player_data.unlock.cutscene_seen.data = character.cutscene_seen.clone();
        player_data.unlock.ornaments.data = character.ornaments.clone();
        player_data.unlock.caught_fish.data = character.caught_fish.clone();
        player_data.unlock.caught_spearfish.data = character.caught_spearfish.clone();
        player_data.unlock.adventures.data = character.adventures.clone();
        player_data.unlock.triple_triad_cards.data = character.triple_triad_cards.clone();
        player_data.unlock.glasses_styles.data = character.glasses_styles.clone();
        player_data.unlock.chocobo_taxi_stands.data = character.chocobo_taxi_stands.clone();
        player_data.unlock.titles.data = character.titles.clone();
        player_data.companion.unlocked_equip.data = character.unlocked_companion_equip.clone();

        // aether current
        player_data.aether_current.unlocked.data = character.unlocked_aether_currents.clone();
        player_data.aether_current.comp_flg_set.data = character.comp_flg_set.clone();

        // aetheryte
        player_data.aetheryte.unlocked.data = character.unlocked_aetherytes.clone();
        player_data.aetheryte.homepoint = character.homepoint;
        player_data.aetheryte.favorite_aetherytes.0 = character.favorite_aetherytes;
        player_data.aetheryte.free_aetheryte = character.free_aetheryte;

        // classjob
        player_data.classjob.current_class = character.current_class;
        player_data.classjob.first_class = character.first_class;
        player_data.classjob.rested_exp = character.rested_exp;

        // content
        player_data.content.unlocked_special_content.data =
            character.unlocked_special_content.clone();
        player_data.content.unlocked_raids.data = character.unlocked_raids.clone();
        player_data.content.unlocked_dungeons.data = character.unlocked_dungeons.clone();
        player_data.content.unlocked_guildhests.data = character.unlocked_guildhests.clone();
        player_data.content.unlocked_trials.data = character.unlocked_trials.clone();
        player_data.content.unlocked_crystalline_conflicts.data =
            character.unlocked_crystalline_conflicts.clone();
        player_data.content.unlocked_frontlines.data = character.unlocked_frontlines.clone();
        player_data.content.cleared_raids.data = character.cleared_raids.clone();
        player_data.content.cleared_dungeons.data = character.cleared_dungeons.clone();
        player_data.content.cleared_guildhests.data = character.cleared_guildhests.clone();
        player_data.content.cleared_trials.data = character.cleared_trials.clone();
        player_data.content.cleared_crystalline_conflicts.data =
            character.cleared_crystalline_conflicts.clone();
        player_data.content.cleared_frontlines.data = character.cleared_frontlines.clone();
        player_data.content.cleared_masked_carnivale.data =
            character.cleared_masked_carnivale.clone();
        player_data.content.unlocked_misc_content.data = character.unlocked_misc_content.clone();
        player_data.content.cleared_misc_content.data = character.cleared_misc_content.clone();

        // quest
        player_data.quest.completed.data = character.completed_quests.clone();

        // volatile
        player_data.volatile.position = Position(Vec3 {
            x: character.position_x,
            y: character.position_y,
            z: character.position_z,
        });
        player_data.volatile.rotation = character.rotation as f64;

        self.commit_player_data(&player_data);

        // Clean up file
        let _ = std::fs::remove_file(path); // It's okay if this fails

        Ok(())
    }
}
