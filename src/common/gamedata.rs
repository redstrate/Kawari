use std::{collections::HashMap, path::PathBuf};

use icarus::Action::ActionSheet;
use icarus::AetherCurrentCompFlgSet::AetherCurrentCompFlgSetSheet;
use icarus::Aetheryte::AetheryteSheet;
use icarus::BNpcBase::BNpcBaseSheet;
use icarus::ClassJob::ClassJobSheet;
use icarus::ContentFinderCondition::ContentFinderConditionSheet;
use icarus::EquipSlotCategory::EquipSlotCategorySheet;
use icarus::GilShopItem::GilShopItemSheet;
use icarus::InstanceContent::InstanceContentSheet;
use icarus::ModelChara::ModelCharaSheet;
use icarus::Mount::MountSheet;
use icarus::PlaceName::PlaceNameSheet;
use icarus::TerritoryType::TerritoryTypeSheet;
use icarus::WeatherRate::WeatherRateSheet;
use icarus::World::WorldSheet;
use icarus::{Tribe::TribeSheet, Warp::WarpSheet};
use physis::common::{Language, Platform};
use physis::exd::{EXD, ExcelRowKind};
use physis::exh::EXH;
use physis::resource::{
    Resource, ResourceResolver, SqPackResource, UnpackedResource, read_excel_sheet,
    read_excel_sheet_header,
};

use crate::{common::Attributes, config::get_config};

use super::timestamp_secs;

const AETHER_CURRENT_COMP_FLG_SET_TO_SCREENIMAGE: [(u32, u32); 31] = [
    // HW
    (1, 328), // Coerthas Western Highlands
    (2, 329), // The Dravanian Forelands
    (3, 330), // The Dravanian Hinterlands
    (4, 331), // The Churning Mists
    (5, 332), // The Sea of Clouds
    (6, 333), // Azys Lla
    // StB
    (7, 511),  // The Fringes
    (8, 514),  // The Ruby Sea
    (9, 512),  // The Peaks
    (10, 515), // Yanxia
    (11, 513), // The Lochs
    (12, 516), // The Azim Steppe
    // ShB
    (13, 762), // Lakeland
    (14, 763), // Amh Araeng
    (15, 764), // Il Mheg
    (16, 765), // Kholusia
    (17, 766), // The Rak'tika Greatwood
    (18, 767), // The Tempest
    // TODO: maybe Mor Dhona's ScreenImage is the "Flying Unlocked" seen at the end of "The Ultimate Weapon" (end of ARR MSQ)? Need a confirmation.
    (19, 0), // Mor Dhona
    // EW
    (20, 1016), // Labyrinthos
    (21, 1017), // Thavnair
    (22, 1018), // Garlemald
    (23, 1019), // Mare Lamentorum
    (24, 1021), // Elpis
    (25, 1020), // Ultima Thule
    // DT
    (26, 1269), // Urqopacha
    (27, 1270), // Kozama'uka
    (28, 1271), // Yak T'el
    (29, 1272), // Shaaloani
    (30, 1273), // Heritage Found
    (31, 1274), // Living Memory
];

fn get_aether_current_comp_flg_set_to_screenimage() -> HashMap<u32, u32> {
    HashMap::from(AETHER_CURRENT_COMP_FLG_SET_TO_SCREENIMAGE)
}

/// Convenient methods built on top of Physis to access data relevant to the server
pub struct GameData {
    pub resource: ResourceResolver,
    pub item_exh: EXH,
    pub item_pages: Vec<EXD>,
    pub classjob_exp_indexes: Vec<i8>,
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct detailing various information about an item, pulled from the Items sheet.
#[derive(Default, Clone)]
pub struct ItemInfo {
    /// The item's textual name.
    pub name: String,
    /// The item's id number.
    pub id: u32,
    /// The item's price, when sold by an NPC.
    pub price_mid: u32,
    /// The item's price, when sold to an NPC by the player.
    pub price_low: u32,
    /// The item's equip category.
    pub equip_category: u8,
    /// The item's primary model id.
    pub primary_model_id: u64,
    /// The item's sub model id.
    pub sub_model_id: u64,
    /// The item's max stack size.
    pub stack_size: u32,
    /// The item's item level.
    pub item_level: u16,
}

#[derive(Debug)]
pub enum ItemInfoQuery {
    ById(u32),
    ByName(String),
}

// From FFXIVClientStructs
// This is actually indexes of InstanceContentType, but we want nice names.
#[derive(Debug)]
pub enum InstanceContentType {
    Raid = 1,
    Dungeon = 2,
    Guildhests = 3,
    Trial = 4,
    CrystallineConflict = 5,
    Frontlines = 6,
    QuestBattle = 7,
    BeginnerTraining = 8,
    DeepDungeon = 9,
    TreasureHuntDungeon = 10,
    SeasonalDungeon = 11,
    RivalWing = 12,
    MaskedCarnivale = 13,
    Mahjong = 14,
    GoldSaucer = 15,
    OceanFishing = 16,
    UnrealTrial = 17,
    TripleTriad = 18,
    VariantDungeon = 19,
    CriterionDungeon = 20,
}

impl TryFrom<u8> for InstanceContentType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Raid),
            2 => Ok(Self::Dungeon),
            3 => Ok(Self::Guildhests),
            4 => Ok(Self::Trial),
            5 => Ok(Self::CrystallineConflict),
            6 => Ok(Self::Frontlines),
            7 => Ok(Self::QuestBattle),
            8 => Ok(Self::BeginnerTraining),
            9 => Ok(Self::DeepDungeon),
            10 => Ok(Self::TreasureHuntDungeon),
            11 => Ok(Self::SeasonalDungeon),
            12 => Ok(Self::RivalWing),
            13 => Ok(Self::MaskedCarnivale),
            14 => Ok(Self::Mahjong),
            15 => Ok(Self::GoldSaucer),
            16 => Ok(Self::OceanFishing),
            17 => Ok(Self::UnrealTrial),
            18 => Ok(Self::TripleTriad),
            19 => Ok(Self::VariantDungeon),
            20 => Ok(Self::CriterionDungeon),
            _ => Err(()),
        }
    }
}

impl GameData {
    pub fn new() -> Self {
        let config = get_config();

        // setup resolvers
        let sqpack_resource = SqPackResourceSpy::from(
            SqPackResource::from_existing(Platform::Win32, &config.filesystem.game_path),
            &config.filesystem.unpack_path,
        );

        if sqpack_resource.sqpack_resource.repositories.is_empty() {
            tracing::warn!(
                "You have an empty game directory ({:?}). This may be a configuration issue, you may want to read the usage documentation.",
                config.filesystem.game_path
            );
        }

        let mut resource_resolver = ResourceResolver::new();
        for path in config.filesystem.additional_search_paths {
            let unpacked_resource = UnpackedResource::from_existing(&path);
            resource_resolver.add_source(Box::new(unpacked_resource));
        }
        resource_resolver.add_source(Box::new(sqpack_resource));

        let mut item_pages = Vec::new();

        let item_exh = read_excel_sheet_header(&mut resource_resolver, "Item")
            .expect("Failed to read Item EXH, does the file exist?");
        for (i, _) in item_exh.pages.iter().enumerate() {
            item_pages.push(
                read_excel_sheet(
                    &mut resource_resolver,
                    "Item",
                    &item_exh,
                    Language::English,
                    i,
                )
                .expect("Failed to read Item EXD, does the file exist?"),
            );
        }

        let mut classjob_exp_indexes = Vec::new();

        let sheet = ClassJobSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read ClassJobSheet, does the Excel files exist?");
        // TODO: ids are hardcoded until we have API in Icarus to do this
        for i in 0..43 {
            let row = sheet.get_row(i).unwrap();

            classjob_exp_indexes.push(*row.ExpArrayIndex().into_i8().unwrap());
        }

        Self {
            resource: resource_resolver,
            item_exh,
            item_pages,
            classjob_exp_indexes,
        }
    }

    /// Gets the world name from an id into the World Excel sheet.
    pub fn get_world_name(&mut self, world_id: u16) -> Option<String> {
        let sheet = WorldSheet::read_from(&mut self.resource, Language::None)?;
        let row = sheet.get_row(world_id as u32)?;

        row.Name().into_string().cloned()
    }

    /// Gets the starting city-state from a given class/job id.
    pub fn get_citystate(&mut self, classjob_id: u16) -> Option<u8> {
        let sheet = ClassJobSheet::read_from(&mut self.resource, Language::English)?;
        let row = sheet.get_row(classjob_id as u32)?;

        row.StartingTown().into_u8().copied()
    }

    pub fn get_racial_base_attributes(&mut self, tribe_id: u8) -> Option<Attributes> {
        // The Tribe Excel sheet only has deltas (e.g. 2 or -2) which are applied to a base 20 number... from somewhere
        let base_stat = 20;

        let sheet = TribeSheet::read_from(&mut self.resource, Language::English)?;
        let row = sheet.get_row(tribe_id as u32)?;

        Some(Attributes {
            strength: (base_stat + row.STR().into_i8()?) as u32,
            dexterity: (base_stat + row.DEX().into_i8()?) as u32,
            vitality: (base_stat + row.VIT().into_i8()?) as u32,
            intelligence: (base_stat + row.INT().into_i8()?) as u32,
            mind: (base_stat + row.MND().into_i8()?) as u32,
        })
    }

    /// Gets various information from the Item sheet.
    pub fn get_item_info(&mut self, query: ItemInfoQuery) -> Option<ItemInfo> {
        let mut result = None;
        'outer: for page in &self.item_pages {
            match query {
                ItemInfoQuery::ById(ref query_item_id) => {
                    if let Some(row) = page.get_row(*query_item_id) {
                        let ExcelRowKind::SingleRow(item_row) = row else {
                            panic!("Expected a single row!");
                        };
                        result = Some((item_row, query_item_id));
                        break 'outer;
                    }
                }

                ItemInfoQuery::ByName(ref query_item_name) => {
                    for row in &page.rows {
                        let ExcelRowKind::SingleRow(single_row) = &row.kind else {
                            panic!("Expected a single row!");
                        };

                        let physis::exd::ColumnData::String(item_name) = &single_row.columns[9]
                        else {
                            panic!("Unexpected type!");
                        };

                        if !item_name
                            .to_lowercase()
                            .contains(&query_item_name.to_lowercase())
                        {
                            continue;
                        }

                        result = Some((single_row.clone(), &row.row_id));
                        break 'outer;
                    }
                }
            }
        }

        if let Some((matched_row, item_id)) = result {
            let physis::exd::ColumnData::String(name) = &matched_row.columns[9] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt16(item_level) = &matched_row.columns[11] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt8(equip_category) = &matched_row.columns[17] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt32(stack_size) = &matched_row.columns[20] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt32(price_mid) = &matched_row.columns[25] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt32(price_low) = &matched_row.columns[26] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt64(primary_model_id) = &matched_row.columns[47] else {
                panic!("Unexpected type!");
            };

            let physis::exd::ColumnData::UInt64(sub_model_id) = &matched_row.columns[48] else {
                panic!("Unexpected type!");
            };

            let item_info = ItemInfo {
                id: *item_id,
                name: name.to_string(),
                price_mid: *price_mid,
                price_low: *price_low,
                equip_category: *equip_category,
                primary_model_id: *primary_model_id,
                sub_model_id: *sub_model_id,
                stack_size: *stack_size,
                item_level: *item_level,
            };

            return Some(item_info);
        }

        None
    }

    /// Gets the primary model ID for a given item ID.
    pub fn get_primary_model_id(&mut self, item_id: u32) -> Option<u64> {
        if let Some(item_info) = self.get_item_info(ItemInfoQuery::ById(item_id)) {
            return Some(item_info.primary_model_id);
        }

        None
    }

    /// Gets the sub model ID for a given item ID.
    pub fn get_sub_model_id(&mut self, item_id: u32) -> Option<u64> {
        if let Some(item_info) = self.get_item_info(ItemInfoQuery::ById(item_id)) {
            // Only return an id if the item actually has a sub model.
            if item_info.sub_model_id != 0 {
                return Some(item_info.sub_model_id);
            }
        }

        None
    }

    /// Returns the pop range object id that's associated with the warp id
    pub fn get_warp(&mut self, warp_id: u32) -> Option<(u32, u16)> {
        let sheet = WarpSheet::read_from(&mut self.resource, Language::English)?;
        let row = sheet.get_row(warp_id)?;

        let pop_range_id = row.PopRange().into_u32()?;
        let zone_id = row.TerritoryType().into_u16()?;

        Some((*pop_range_id, *zone_id))
    }

    pub fn get_aetheryte(&mut self, aetheryte_id: u32) -> Option<(u32, u16)> {
        let sheet = AetheryteSheet::read_from(&mut self.resource, Language::English)?;
        let row = sheet.get_row(aetheryte_id)?;

        // TODO: just look in the level sheet?
        let pop_range_id = row.Level()[0].into_u32()?;
        let zone_id = row.Territory().into_u16()?;

        Some((*pop_range_id, *zone_id))
    }

    /// Retrieves a zone's internal name, place name or parent region name.
    pub fn get_territory_name(&mut self, zone_id: u32, which: TerritoryNameKind) -> Option<String> {
        let sheet = TerritoryTypeSheet::read_from(&mut self.resource, Language::None)?;
        let row = sheet.get_row(zone_id)?;

        let offset = match which {
            TerritoryNameKind::Internal => {
                return row.Name().into_string().cloned();
            }
            TerritoryNameKind::Region => row.PlaceNameRegion().into_u16()?,
            TerritoryNameKind::Place => row.PlaceName().into_u16()?,
        };

        let sheet = PlaceNameSheet::read_from(&mut self.resource, Language::English)?;
        let row = sheet.get_row(*offset as u32)?;

        let value = row.Name().into_string()?;

        Some(value.clone())
    }

    /// Turn an equip slot category id into a slot for the equipped inventory
    pub fn get_equipslot_category(&mut self, equipslot_id: u8) -> Option<u16> {
        let sheet = EquipSlotCategorySheet::read_from(&mut self.resource, Language::None)?;
        let row = sheet.get_row(equipslot_id as u32)?;

        let main_hand = row.MainHand().into_i8()?;
        if *main_hand == 1 {
            return Some(0);
        }

        let off_hand = row.OffHand().into_i8()?;
        if *off_hand == 1 {
            return Some(1);
        }

        let head = row.Head().into_i8()?;
        if *head == 1 {
            return Some(2);
        }

        let body = row.Body().into_i8()?;
        if *body == 1 {
            return Some(3);
        }

        let gloves = row.Gloves().into_i8()?;
        if *gloves == 1 {
            return Some(4);
        }

        let legs = row.Legs().into_i8()?;
        if *legs == 1 {
            return Some(6);
        }

        let feet = row.Feet().into_i8()?;
        if *feet == 1 {
            return Some(7);
        }

        let ears = row.Ears().into_i8()?;
        if *ears == 1 {
            return Some(8);
        }

        let neck = row.Neck().into_i8()?;
        if *neck == 1 {
            return Some(9);
        }

        let wrists = row.Wrists().into_i8()?;
        if *wrists == 1 {
            return Some(10);
        }

        let right_finger = row.FingerR().into_i8()?;
        if *right_finger == 1 {
            return Some(11);
        }

        let left_finger = row.FingerL().into_i8()?;
        if *left_finger == 1 {
            return Some(12);
        }

        let soul_crystal = row.SoulCrystal().into_i8()?;
        if *soul_crystal == 1 {
            return Some(13);
        }

        None
    }

    pub fn get_casttime(&mut self, action_id: u32) -> Option<u16> {
        let sheet = ActionSheet::read_from(&mut self.resource, Language::English)?;
        let row = sheet.get_row(action_id)?;

        row.Cast100ms().into_u16().copied()
    }

    /// Calculates the current weather at the current time
    // TODO: instead allow targetting a specific time to calculate forcecasts
    pub fn get_weather_rate(&mut self, weather_rate_id: u32) -> Option<i32> {
        let sheet = WeatherRateSheet::read_from(&mut self.resource, Language::None)?;
        let row = sheet.get_row(weather_rate_id)?;

        // sum up the rates
        let mut rates = row.Rate().map(|x| *x.into_u8().unwrap());
        let mut sum = 0;
        for rate in &mut rates {
            sum += *rate;
            *rate = sum;
        }

        let target = Self::calculate_target();
        let weather_and_rates: Vec<(i32, i32)> = row
            .Weather()
            .iter()
            .cloned()
            .zip(rates)
            .map(|(x, y)| (*x.into_i32().unwrap(), y as i32))
            .filter(|x| x.0 > 0) // don't take into account invalid weather ids
            .collect();

        Some(
            weather_and_rates
                .iter()
                .filter(|(_, rate)| target <= *rate)
                .take(1)
                .collect::<Vec<&(i32, i32)>>()
                .first()?
                .0,
        )
    }

    /// Calculate target window for weather calculations
    fn calculate_target() -> i32 {
        let unix_seconds = timestamp_secs();
        let eorzean_hours = f32::floor(unix_seconds as f32 / 175.0) as u32;
        let eorzean_days = f32::floor(eorzean_hours as f32 / 24.0) as u32;

        let time_chunk = (eorzean_hours % 24) - (eorzean_hours % 8);
        let time_chunk = (time_chunk + 8) % 24;
        let calc_base = (eorzean_days * 100) + time_chunk;

        let step1 = (calc_base << 0xB) ^ calc_base;
        let step2 = (step1 >> 8) ^ step1;

        (step2 % 100) as i32
    }

    /// Gets the current weather for the given zone id
    pub fn get_weather(&mut self, zone_id: u32) -> Option<i32> {
        let sheet = TerritoryTypeSheet::read_from(&mut self.resource, Language::None)?;
        let row = sheet.get_row(zone_id)?;

        let weather_rate_id = row.WeatherRate().into_u8()?;
        self.get_weather_rate(*weather_rate_id as u32)
    }

    /// Gets the array index used in EXP & levels.
    pub fn get_exp_array_index(&self, classjob_id: u16) -> Option<i8> {
        self.classjob_exp_indexes.get(classjob_id as usize).copied()
    }

    /// Gets the item and its cost from the specified shop.
    pub fn get_gilshop_item(&mut self, gilshop_id: u32, index: u16) -> Option<ItemInfo> {
        let sheet = GilShopItemSheet::read_from(&mut self.resource, Language::None)?;
        let row = sheet.get_subrow(gilshop_id, index)?;
        let item_id = row.Item().into_i32()?;

        self.get_item_info(ItemInfoQuery::ById(*item_id as u32))
    }

    /// Gets the zone id for the given InstanceContent.
    pub fn find_zone_for_content(&mut self, content_id: u16) -> Option<u16> {
        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row = instance_content_sheet.get_row(content_id as u32)?;

        let content_finder_row_id = instance_content_row.ContentFinderCondition().into_u16()?;
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, Language::English).unwrap();
        let content_finder_row = content_finder_sheet.get_row(*content_finder_row_id as u32)?;

        content_finder_row.TerritoryType().into_u16().copied()
    }

    /// Grabs needed BattleNPC information such as their name, model id and more.
    pub fn find_bnpc(&mut self, id: u32) -> Option<u16> {
        let bnpc_sheet = BNpcBaseSheet::read_from(&mut self.resource, Language::None).unwrap();
        let bnpc_row = bnpc_sheet.get_row(id)?;

        let model_row_id = bnpc_row.ModelChara().into_u16()?;
        let model_sheet = ModelCharaSheet::read_from(&mut self.resource, Language::None).unwrap();
        let model_row = model_sheet.get_row(*model_row_id as u32)?;

        model_row.Model().into_u16().copied()
    }

    /// Gets the content type for the given InstanceContent.
    pub fn find_type_for_content(&mut self, content_id: u16) -> Option<InstanceContentType> {
        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row = instance_content_sheet.get_row(content_id as u32)?;

        instance_content_row
            .InstanceContentType()
            .into_u8()
            .copied()?
            .try_into()
            .ok()
    }

    /// Gets the order of the mount.
    pub fn find_mount_order(&mut self, mount_id: u32) -> Option<i16> {
        let instance_content_sheet =
            MountSheet::read_from(&mut self.resource, Language::English).unwrap();
        let mount_row = instance_content_sheet.get_row(mount_id)?;

        mount_row.Order().into_i16().copied()
    }

    /// Gets the Item ID of the Orchestrion Roll.
    pub fn find_orchestrion_item_id(&mut self, orchestrion_id: u32) -> Option<u32> {
        let mut result = None;
        'outer: for page in &self.item_pages {
            for row in &page.rows {
                let ExcelRowKind::SingleRow(single_row) = &row.kind else {
                    panic!("Expected a single row!");
                };

                let filter_group = single_row.columns[13].into_u8()?;

                // If filter_group is 32, then this item is an Orchestrion Roll...
                if *filter_group != 32 {
                    continue;
                }

                // ...and additional_data will be the Orchestrion ID
                let additional_data = single_row.columns[14].into_u32()?;

                if *additional_data != orchestrion_id {
                    continue;
                }

                result = Some(&row.row_id);
                break 'outer;
            }
        }

        if let Some(item_id) = result {
            return Some(*item_id);
        }

        None
    }

    /// Gets the Set/Zone of the Aether Current
    pub fn find_aether_current_set(&mut self, aether_current_id: i32) -> Option<u32> {
        // Get AetherCurrentCompFlgSet sheet
        let mut aether_current_comp_flg_set_pages = Vec::new();
        let aether_current_comp_flg_set_exh =
            read_excel_sheet_header(&mut self.resource, "AetherCurrentCompFlgSet")
                .expect("Failed to read AetherCurrentCompFlgSet EXH, does the file exist?");
        for (i, _) in aether_current_comp_flg_set_exh.pages.iter().enumerate() {
            aether_current_comp_flg_set_pages.push(
                read_excel_sheet(
                    &mut self.resource,
                    "AetherCurrentCompFlgSet",
                    &aether_current_comp_flg_set_exh,
                    Language::None,
                    i,
                )
                .expect("Failed to read AetherCurrentCompFlgSet EXD, does the file exist?"),
            );
        }

        // Start searching for Zone ID
        let mut result = None;
        'outer: for page in &aether_current_comp_flg_set_pages {
            for row in &page.rows {
                let ExcelRowKind::SingleRow(single_row) = &row.kind else {
                    panic!("Expected a single row!");
                };

                let aether_current_0 = single_row.columns[1].into_i32()?;
                let aether_current_1 = single_row.columns[2].into_i32()?;
                let aether_current_2 = single_row.columns[3].into_i32()?;
                let aether_current_3 = single_row.columns[4].into_i32()?;
                let aether_current_4 = single_row.columns[5].into_i32()?;
                let aether_current_5 = single_row.columns[6].into_i32()?;
                let aether_current_6 = single_row.columns[7].into_i32()?;
                let aether_current_7 = single_row.columns[8].into_i32()?;
                let aether_current_8 = single_row.columns[9].into_i32()?;
                let aether_current_9 = single_row.columns[10].into_i32()?;
                let aether_current_10 = single_row.columns[11].into_i32()?;
                let aether_current_11 = single_row.columns[12].into_i32()?;
                let aether_current_12 = single_row.columns[13].into_i32()?;
                let aether_current_13 = single_row.columns[14].into_i32()?;
                let aether_current_14 = single_row.columns[15].into_i32()?;

                if *aether_current_0 == aether_current_id
                    || *aether_current_1 == aether_current_id
                    || *aether_current_2 == aether_current_id
                    || *aether_current_3 == aether_current_id
                    || *aether_current_4 == aether_current_id
                    || *aether_current_5 == aether_current_id
                    || *aether_current_6 == aether_current_id
                    || *aether_current_7 == aether_current_id
                    || *aether_current_8 == aether_current_id
                    || *aether_current_9 == aether_current_id
                    || *aether_current_10 == aether_current_id
                    || *aether_current_11 == aether_current_id
                    || *aether_current_12 == aether_current_id
                    || *aether_current_13 == aether_current_id
                    || *aether_current_14 == aether_current_id
                {
                    result = Some(&row.row_id);
                    break 'outer;
                }
            }
        }

        if let Some(item_id) = result {
            return Some(*item_id);
        }

        None
    }

    /// Gets the Aether Currents needed for a zone.
    pub fn get_aether_currents_from_zone(
        &mut self,
        aether_current_comp_flg_set_id: u32,
    ) -> Option<Vec<i32>> {
        let aether_current_comp_flg_set_sheet =
            AetherCurrentCompFlgSetSheet::read_from(&mut self.resource, Language::None).unwrap();

        let row = aether_current_comp_flg_set_sheet.get_row(aether_current_comp_flg_set_id)?;

        let aether_currents_from_zone = row
            .AetherCurrents()
            .iter()
            .map(|x| *x.into_i32().unwrap())
            .filter(|x| *x != 0)
            .collect();

        return Some(aether_currents_from_zone);
    }

    pub fn get_screenimage_from_aether_current_comp_flg_set(
        &mut self,
        aether_current_comp_flg_set_id: u32,
    ) -> Option<u32> {
        let aether_current_comp_flg_set_to_screenimage =
            get_aether_current_comp_flg_set_to_screenimage();
        return Some(aether_current_comp_flg_set_to_screenimage[&aether_current_comp_flg_set_id]);
    }
}

/// Simple enum for GameData::get_territory_name
pub enum TerritoryNameKind {
    Internal,
    Region,
    Place,
}

/// Wrapper around SqPackResource to let us spy when it reads files
struct SqPackResourceSpy {
    sqpack_resource: SqPackResource,
    output_directory: String,
}

impl SqPackResourceSpy {
    pub fn from(sqpack_resource: SqPackResource, output_directory: &str) -> Self {
        Self {
            sqpack_resource,
            output_directory: output_directory.to_string(),
        }
    }
}

impl Resource for SqPackResourceSpy {
    fn read(&mut self, path: &str) -> Option<physis::ByteBuffer> {
        if let Some(buffer) = self.sqpack_resource.read(path) {
            if !self.output_directory.is_empty() {
                let mut new_path = PathBuf::from(&self.output_directory);
                new_path.push(path.to_lowercase());

                if !std::fs::exists(&new_path).unwrap_or_default() {
                    // create directory if it doesn't exist'
                    let parent_directory = new_path.parent().unwrap();
                    if !std::fs::exists(parent_directory).unwrap_or_default() {
                        std::fs::create_dir_all(parent_directory)
                            .expect("Couldn't create directory for extraction?!");
                    }

                    std::fs::write(new_path, &buffer).expect("Couldn't extract file!!");
                }
            }

            return Some(buffer);
        }

        None
    }

    fn exists(&mut self, path: &str) -> bool {
        self.sqpack_resource.exists(path)
    }
}
