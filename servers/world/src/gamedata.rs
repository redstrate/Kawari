use std::collections::HashSet;
use std::path::PathBuf;

use icarus::Action::ActionSheet;
use icarus::AetherCurrentCompFlgSet::AetherCurrentCompFlgSetSheet;
use icarus::Aetheryte::AetheryteSheet;
use icarus::AetheryteSystemDefine::AetheryteSystemDefineSheet;
use icarus::BNpcBase::BNpcBaseSheet;
use icarus::BNpcCustomize::BNpcCustomizeSheet;
use icarus::BaseParam::{BaseParamRow, BaseParamSheet};
use icarus::Battalion::BattalionSheet;
use icarus::ClassJob::ClassJobSheet;
use icarus::ContentDirectorManagedSG::ContentDirectorManagedSGSheet;
use icarus::ContentFinderCondition::ContentFinderConditionSheet;
use icarus::CraftAction::CraftActionSheet;
use icarus::CustomTalk::CustomTalkSheet;
use icarus::EObj::EObjSheet;
use icarus::Emote::EmoteSheet;
use icarus::EquipSlotCategory::EquipSlotCategorySheet;
use icarus::FateShop::FateShopSheet;
use icarus::FittingShopCategoryItem::FittingShopCategoryItemSheet;
use icarus::GatheringItem::GatheringItemSheet;
use icarus::GatheringPoint::GatheringPointSheet;
use icarus::GatheringPointBase::GatheringPointBaseSheet;
use icarus::GilShopItem::GilShopItemSheet;
use icarus::GimmickRect::{GimmickRectRow, GimmickRectSheet};
use icarus::HalloweenNpcSelect::HalloweenNpcSelectSheet;
use icarus::HousingAethernet::HousingAethernetSheet;
use icarus::InstanceContent::InstanceContentSheet;
use icarus::Item::ItemSheet;
use icarus::ItemAction::ItemActionSheet;
use icarus::ItemLevel::ItemLevelSheet;
use icarus::Mount::MountSheet;
use icarus::NpcYell::NpcYellSheet;
use icarus::OnlineStatus::OnlineStatusSheet;
use icarus::Opening::OpeningSheet;
use icarus::ParamGrow::{ParamGrowRow, ParamGrowSheet};
use icarus::PlaceName::PlaceNameSheet;
use icarus::PreHandler::PreHandlerSheet;
use icarus::Quest::QuestSheet;
use icarus::Recipe::RecipeSheet;
use icarus::SpecialShop::SpecialShopSheet;
use icarus::SwitchTalkVariation::{SwitchTalkVariationRow, SwitchTalkVariationSheet};
use icarus::TerritoryType::TerritoryTypeSheet;
use icarus::TopicSelect::TopicSelectSheet;
use icarus::WarpLogic::WarpLogicSheet;
use icarus::WeatherRate::WeatherRateSheet;
use icarus::{Tribe::TribeSheet, Warp::WarpSheet};
use physis::Language;
use physis::resource::{Resource, ResourceResolver, SqPackResource, UnpackedResource};

use kawari::common::{CustomizeData, timestamp_secs};
use kawari::common::{InstanceContentType, get_aether_current_comp_flg_set_to_screenimage};
use kawari::config::get_config;
use strum::FromRepr;

/// Convenient methods built on top of Physis to access data relevant to the server
#[derive(Clone)]
pub struct GameData {
    pub resource: ResourceResolver,

    // Remember to keep frequently accessed or large sheets here, until we have a better caching solution.
    pub item_sheet: ItemSheet,
    pub classjob_exp_indexes: Vec<i8>,
    pub weather_rate_sheet: WeatherRateSheet,
    pub territory_type_sheet: TerritoryTypeSheet,
    pub quest_sheet: QuestSheet,
    pub warp_sheet: WarpSheet,
    pub action_sheet: ActionSheet,
    pub place_name_sheet: PlaceNameSheet,
    pub custom_talk_sheet: CustomTalkSheet,
    pub tribe_sheet: TribeSheet,
    pub eobj_sheet: EObjSheet,
    pub switch_talk_sheet: SwitchTalkVariationSheet,
    pub param_grow_sheet: ParamGrowSheet,
    pub bnpc_base_sheet: BNpcBaseSheet,
    pub bnpc_customize_sheet: BNpcCustomizeSheet,
    pub item_level_sheet: ItemLevelSheet,
    pub gimmick_rect_sheet: GimmickRectSheet,
    pub base_param_sheet: BaseParamSheet,
    pub classjob_sheet: ClassJobSheet,
    pub battalion_sheet: BattalionSheet,
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct Attributes {
    pub strength: i8,
    pub dexterity: i8,
    pub vitality: i8,
    pub intelligence: i8,
    pub mind: i8,
    pub piety: i8,
}

/// Struct detailing various information about an item, pulled from the Items sheet.
#[derive(Debug, Default, Clone)]
pub struct ItemRow {
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
    /// The item's equip restrictions.
    pub equip_restrictions: ItemEquipRestrictions,
    /// The item's primary model id.
    pub primary_model_id: u64,
    /// The item's sub model id.
    pub sub_model_id: u64,
    /// The item's max stack size.
    pub stack_size: u32,
    /// The item's item level.
    pub item_level: u16,
    /// The item's ClassJobCategory.
    pub classjob_category: u8,

    /// Stat modifier stuff
    pub base_param_ids: [u8; 6],
    pub base_param_values: [i16; 6],

    /// Physical defense.
    pub defense: u16,
    /// Magic defense;
    pub magic_defense: u16,
}

#[derive(Debug)]
pub enum ItemInfoQuery {
    ById(u32),
    ByName(String),
}

/// Struct detailing what slots, if any, this item blocks from equipping. Accessories are not considered here since they never block other items.
#[derive(Clone, Debug, Default)]
pub struct ItemEquipRestrictions {
    pub main_hand: i8,
    pub off_hand: i8,
    pub head: i8,
    pub body: i8,
    pub hands: i8,
    pub legs: i8,
    pub feet: i8,
}

#[derive(Debug, Clone, Copy)]
pub struct Recipe {
    pub id: u32,
    pub item_id: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct Modifiers {
    pub hp: u16,
    pub mp: u16,
    pub strength: u16,
    pub vitality: u16,
    pub dexterity: u16,
    pub intelligence: u16,
    pub mind: u16,
    pub piety: u16,
}

impl Modifiers {
    pub fn apply_to(&self, index: u8, value: u32) -> u32 {
        let modifier = match index {
            1 => self.strength,
            2 => self.dexterity,
            3 => self.vitality,
            4 => self.intelligence,
            5 => self.mind,
            6 => self.piety,
            _ => return value,
        };

        (value as f32 * (modifier as f32 / 100.0)).floor() as u32
    }
}

#[repr(u8)]
#[derive(FromRepr)]
pub enum Roulette {
    Leveling = 1,
    HighLevel = 2,
    MSQ = 3,
    GuildHest = 4,
    Expert = 5,
    Trial = 6,
    DailyFrontline = 7,
    LevelCap = 8,
    Mentor = 9,
    Alliance = 15,
    NormalRaid = 17,
    CrystallineConflictCasual = 40,
    CrystallineConflictRanked = 41,
}

impl GameData {
    pub fn new() -> Self {
        let config = get_config();

        let mut sqpack_resource = SqPackResourceSpy::from(
            SqPackResource::from_existing(&config.filesystem.game_path),
            &config.filesystem.unpack_path,
        );

        if sqpack_resource.sqpack_resource.repositories.is_empty() {
            tracing::warn!(
                "You have an empty game directory ({:?}). This may be a configuration issue, you may want to read the usage documentation.",
                config.filesystem.game_path
            );
        }

        // We preload all index files, because the cost for not doing this can be high.
        // For example: someone travels to a new zone (that wasn't previously loaded), so the server has to basically halt to read a bunch of index files from disk.
        // Index files are small and will take up very little memory, so this is a no-brainer optimization.
        sqpack_resource.sqpack_resource.preload_index_files();

        let mut resource_resolver = ResourceResolver::new();
        for path in config.filesystem.additional_search_paths {
            let unpacked_resource = UnpackedResource::from_existing(&path);
            resource_resolver.add_source(unpacked_resource);
        }
        resource_resolver.add_source(sqpack_resource);

        let mut classjob_exp_indexes = Vec::new();

        let classjob_sheet =
            ClassJobSheet::read_from(&mut resource_resolver, config.world.language())
                .expect("Failed to read ClassJobSheet, does the Excel files exist?");
        for (_, row) in classjob_sheet.into_iter().flatten_subrows() {
            classjob_exp_indexes.push(row.ExpArrayIndex());
        }

        let item_sheet = ItemSheet::read_from(&mut resource_resolver, config.world.language())
            .expect("Failed to read ItemSheet, does the Excel files exist?");

        let weather_rate_sheet =
            WeatherRateSheet::read_from(&mut resource_resolver, Language::None)
                .expect("Failed to read WeatherRateSheet, does the Excel files exist?");

        let quest_sheet = QuestSheet::read_from(&mut resource_resolver, config.world.language())
            .expect("Failed to read Quest, does the Excel files exist?");

        let territory_type_sheet =
            TerritoryTypeSheet::read_from(&mut resource_resolver, Language::None)
                .expect("Failed to read TerritoryTypeSheet, does the Excel files exist?");

        let warp_sheet = WarpSheet::read_from(&mut resource_resolver, config.world.language())
            .expect("Failed to read Warp, does the Excel files exist?");

        let action_sheet = ActionSheet::read_from(&mut resource_resolver, config.world.language())
            .expect("Failed to read Action, does the Excel files exist?");

        let place_name_sheet =
            PlaceNameSheet::read_from(&mut resource_resolver, config.world.language())
                .expect("Failed to read PlaceName, does the Excel files exist?");

        let custom_talk_sheet =
            CustomTalkSheet::read_from(&mut resource_resolver, config.world.language())
                .expect("Failed to read CustomTalk, does the Excel files exist?");

        let tribe_sheet = TribeSheet::read_from(&mut resource_resolver, config.world.language())
            .expect("Failed to read Tribe, does the Excel files exist?");

        let eobj_sheet = EObjSheet::read_from(&mut resource_resolver, Language::None)
            .expect("Failed to read EObj, does the Excel files exist?");

        let switch_talk_sheet =
            SwitchTalkVariationSheet::read_from(&mut resource_resolver, Language::None).unwrap();

        let param_grow_sheet =
            ParamGrowSheet::read_from(&mut resource_resolver, Language::None).unwrap();

        let bnpc_base_sheet =
            BNpcBaseSheet::read_from(&mut resource_resolver, Language::None).unwrap();

        let bnpc_customize_sheet =
            BNpcCustomizeSheet::read_from(&mut resource_resolver, Language::None).unwrap();

        let item_level_sheet =
            ItemLevelSheet::read_from(&mut resource_resolver, Language::None).unwrap();

        let gimmick_rect_sheet =
            GimmickRectSheet::read_from(&mut resource_resolver, Language::None).unwrap();

        let base_param_sheet =
            BaseParamSheet::read_from(&mut resource_resolver, config.world.language())
                .ok()
                .unwrap();

        let battalion_sheet = BattalionSheet::read_from(&mut resource_resolver, Language::None)
            .ok()
            .unwrap();

        Self {
            resource: resource_resolver,
            item_sheet,
            classjob_exp_indexes,
            weather_rate_sheet,
            quest_sheet,
            territory_type_sheet,
            warp_sheet,
            action_sheet,
            place_name_sheet,
            custom_talk_sheet,
            tribe_sheet,
            eobj_sheet,
            switch_talk_sheet,
            param_grow_sheet,
            bnpc_base_sheet,
            bnpc_customize_sheet,
            item_level_sheet,
            gimmick_rect_sheet,
            base_param_sheet,
            classjob_sheet,
            battalion_sheet,
        }
    }

    /// Gets the starting city-state from a given class/job id.
    pub fn get_citystate(&mut self, classjob_id: u16) -> Option<u8> {
        let row = self.classjob_sheet.row(classjob_id as u32)?;

        Some(row.StartingTown())
    }

    pub fn get_racial_base_attributes(&mut self, tribe_id: u8) -> Option<Attributes> {
        let row = self.tribe_sheet.row(tribe_id as u32)?;

        Some(Attributes {
            strength: row.STR(),
            dexterity: row.DEX(),
            vitality: row.VIT(),
            intelligence: row.INT(),
            mind: row.MND(),
            piety: row.PIE(),
        })
    }

    /// Gets various information from the Item sheet.
    pub fn get_item_info(&mut self, query: ItemInfoQuery) -> Option<ItemRow> {
        let mut result = None;
        match query {
            ItemInfoQuery::ById(ref query_item_id) => {
                if let Some(row) = self.item_sheet.row(*query_item_id) {
                    result = Some((row, *query_item_id));
                }
            }

            ItemInfoQuery::ByName(ref query_item_name) => {
                for (id, row) in self.item_sheet.into_iter().flatten_subrows() {
                    if row
                        .Name()
                        .to_lowercase()
                        .contains(&query_item_name.to_lowercase())
                    {
                        result = Some((row.clone(), id));
                        break;
                    }
                }
            }
        }

        if let Some((matched_row, item_id)) = result {
            let item_info = ItemRow {
                id: item_id,
                name: matched_row.Name().to_string(),
                price_mid: matched_row.PriceMid(),
                price_low: matched_row.PriceLow(),
                equip_category: matched_row.EquipSlotCategory(),
                primary_model_id: matched_row.ModelMain(),
                sub_model_id: matched_row.ModelSub(),
                stack_size: matched_row.StackSize(),
                item_level: matched_row.LevelItem(),
                classjob_category: matched_row.ClassJobCategory(),
                base_param_ids: matched_row.BaseParam(),
                base_param_values: matched_row.BaseParamValue(),
                defense: matched_row.DefensePhys(),
                magic_defense: matched_row.DefenseMag(),
                equip_restrictions: self
                    .get_equipslot_restrictions(matched_row.EquipSlotCategory())
                    .unwrap(),
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
        let row = self.warp_sheet.row(warp_id)?;

        Some((row.PopRange(), row.TerritoryType()))
    }

    /// Returns the warp logic name (if any) for this Warp.
    pub fn get_warp_logic_name(&mut self, warp_id: u32) -> String {
        let row = self.warp_sheet.row(warp_id).unwrap();

        let warp_logic_id = row.WarpLogic();

        let config = get_config();
        let warp_logic_sheet =
            WarpLogicSheet::read_from(&mut self.resource, config.world.language()).unwrap();
        let warp_logic_row = warp_logic_sheet.row(warp_logic_id as u32).unwrap();

        warp_logic_row.WarpName().to_string()
    }

    pub fn get_aetheryte(
        &mut self,
        aetheryte_id: u32,
        housing_aethernet: bool,
    ) -> Option<(u32, u16)> {
        let config = get_config();

        if !housing_aethernet {
            let sheet =
                AetheryteSheet::read_from(&mut self.resource, config.world.language()).ok()?;
            let row = sheet.row(aetheryte_id)?;

            // TODO: just look in the level sheet?
            let pop_range_id = row.Level()[0];
            let zone_id = row.Territory();

            Some((pop_range_id, zone_id))
        } else {
            let sheet =
                HousingAethernetSheet::read_from(&mut self.resource, Language::None).ok()?;
            let row = sheet.row(aetheryte_id)?;

            // TODO: just look in the level sheet?
            // Note that the HousingAethernet sheet's Level column isn't an array.
            let pop_range_id = row.Level();
            let zone_id = row.TerritoryType();

            Some((pop_range_id, zone_id))
        }
    }

    /// Checks if it's a big Aetheryte (true) or just a shard (false.)
    pub fn is_aetheryte(&mut self, aetheryte_id: u32) -> bool {
        let config = get_config();
        let sheet = AetheryteSheet::read_from(&mut self.resource, config.world.language()).unwrap();
        let row = sheet.row(aetheryte_id).unwrap();

        row.IsAetheryte()
    }

    /// Retrieves a zone's internal name, place name or parent region name.
    pub fn get_territory_name(&mut self, zone_id: u32, which: TerritoryNameKind) -> Option<String> {
        let row = self.territory_type_sheet.row(zone_id)?;

        let offset = match which {
            TerritoryNameKind::Internal => {
                return Some(row.Name().to_string());
            }
            TerritoryNameKind::Region => row.PlaceNameRegion(),
            TerritoryNameKind::Place => row.PlaceName(),
        };

        let row = self.place_name_sheet.row(offset as u32)?;
        Some(row.Name().to_string())
    }

    /// Turn an equip slot category id into a slot for the equipped inventory
    pub fn get_equipslot_category(&mut self, equipslot_id: u8) -> Option<u16> {
        let sheet = EquipSlotCategorySheet::read_from(&mut self.resource, Language::None).ok()?;
        let row = sheet.row(equipslot_id as u32)?;

        let main_hand = row.MainHand();
        if main_hand == 1 {
            return Some(0);
        }

        let off_hand = row.OffHand();
        if off_hand == 1 {
            return Some(1);
        }

        let head = row.Head();
        if head == 1 {
            return Some(2);
        }

        let body = row.Body();
        if body == 1 {
            return Some(3);
        }

        let gloves = row.Gloves();
        if gloves == 1 {
            return Some(4);
        }

        let legs = row.Legs();
        if legs == 1 {
            return Some(6);
        }

        let feet = row.Feet();
        if feet == 1 {
            return Some(7);
        }

        let ears = row.Ears();
        if ears == 1 {
            return Some(8);
        }

        let neck = row.Neck();
        if neck == 1 {
            return Some(9);
        }

        let wrists = row.Wrists();
        if wrists == 1 {
            return Some(10);
        }

        let right_finger = row.FingerR();
        if right_finger == 1 {
            return Some(11);
        }

        let left_finger = row.FingerL();
        if left_finger == 1 {
            return Some(12);
        }

        let soul_crystal = row.SoulCrystal();
        if soul_crystal == 1 {
            return Some(13);
        }

        None
    }

    // Returns information on what item slots, if any, this equipslot configuration blocks.
    // For example, a two-handed weapon (MainHand = 1, OffHand = -1) will always block an off-hand from being equipped.
    fn get_equipslot_restrictions(&mut self, equipslot_id: u8) -> Option<ItemEquipRestrictions> {
        let sheet = EquipSlotCategorySheet::read_from(&mut self.resource, Language::None).ok()?;
        let row = sheet.row(equipslot_id as u32)?;

        Some(ItemEquipRestrictions {
            main_hand: row.MainHand(),
            off_hand: row.OffHand(),
            head: row.Head(),
            body: row.Body(),
            hands: row.Gloves(),
            legs: row.Legs(),
            feet: row.Feet(),
        })
    }

    pub fn get_casttime(&mut self, action_id: u32) -> Option<u16> {
        let row = self.action_sheet.row(action_id)?;

        Some(row.Cast100ms())
    }

    /// Calculates the current weather at the current time
    // TODO: instead allow targetting a specific time to calculate forcecasts
    pub fn get_weather_rate(&mut self, weather_rate_id: u32) -> Option<i32> {
        let row = self.weather_rate_sheet.row(weather_rate_id)?;

        // sum up the rates
        let mut rates = row.Rate();
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
            .map(|(x, y)| (x, y as i32))
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
        let row = self.territory_type_sheet.row(zone_id)?;

        let weather_rate_id = row.WeatherRate();
        self.get_weather_rate(weather_rate_id as u32)
    }

    /// Gets the array index used in EXP & levels.
    pub fn get_exp_array_index(&self, classjob_id: u16) -> Option<i8> {
        self.classjob_exp_indexes
            .get(classjob_id as usize)
            .copied()
            .filter(|x| *x != -1)
    }

    /// Gets the job index for a given class.
    pub fn get_job_index(&mut self, classjob_id: u16) -> Option<u8> {
        let row = self.classjob_sheet.row(classjob_id as u32)?;

        Some(row.JobIndex())
    }

    /// Gets the item and its cost from the specified shop.
    pub fn get_gilshop_item(&mut self, gilshop_id: u32, index: u16) -> Option<ItemRow> {
        let sheet = GilShopItemSheet::read_from(&mut self.resource, Language::None).ok()?;
        let row = sheet.subrow(gilshop_id, index)?;
        let item_id = row.Item();

        self.get_item_info(ItemInfoQuery::ById(item_id as u32))
    }

    /// Gets the item and its cost from the specified SpecialShop.
    pub fn get_specialshop_item(&mut self, gilshop_id: u32, index: u16) -> Option<ItemRow> {
        let config = get_config();
        let sheet =
            SpecialShopSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        let row = sheet.row(gilshop_id)?;
        let item_id = row.Item()[index as usize].Item[0]; // TODO: why are there two items?

        self.get_item_info(ItemInfoQuery::ById(item_id as u32))
    }

    /// Gets the zone id for the given ContentFinderCondition ID.
    pub fn find_zone_for_content(&mut self, content_id: u16) -> Option<u16> {
        let config = get_config();
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, config.world.language())
                .unwrap();
        let content_finder_row = content_finder_sheet.row(content_id as u32)?;

        Some(content_finder_row.TerritoryType())
    }

    /// Grabs needed BattleNPC information such as their name, model id and more.
    pub fn find_bnpc(&mut self, id: u32) -> Option<(u16, u8, CustomizeData, u8)> {
        let bnpc_row = self.bnpc_base_sheet.row(id)?;
        let model_row_id = bnpc_row.ModelChara();
        let customize_row_id = bnpc_row.BNpcCustomize();
        let customize_row = self.bnpc_customize_sheet.row(customize_row_id as u32)?;

        let customize = CustomizeData {
            race: customize_row.Race(),
            gender: customize_row.Gender(),
            age: customize_row.BodyType(),
            height: customize_row.Height(),
            subrace: customize_row.Tribe(),
            face: customize_row.Face(),
            hair: customize_row.HairStyle(),
            enable_highlights: customize_row.HairHighlight(),
            skin_tone: customize_row.SkinColor(),
            right_eye_color: customize_row.EyeColor(),
            hair_tone: customize_row.HairColor(),
            highlights: customize_row.HairHighlightColor(),
            facial_features: customize_row.FacialFeature(),
            facial_feature_color: customize_row.FacialFeatureColor(),
            eyebrows: customize_row.Eyebrows(),
            left_eye_color: customize_row.EyeHeterochromia(),
            eyes: customize_row.EyeShape(),
            nose: customize_row.Nose(),
            jaw: customize_row.Jaw(),
            mouth: customize_row.Mouth(),
            lips_tone_fur_pattern: customize_row.LipColor(),
            race_feature_size: customize_row.BustOrTone1(),
            race_feature_type: customize_row.ExtraFeature1(),
            bust: customize_row.ExtraFeature2OrBust(),
            face_paint: customize_row.FacePaint(),
            face_paint_color: customize_row.FacePaintColor(),
        };

        Some((
            model_row_id,
            bnpc_row.Battalion(),
            customize,
            bnpc_row.Rank(),
        ))
    }

    /// Gets the content type for the given InstanceContent.
    pub fn find_type_for_content(&mut self, content_id: u16) -> Option<InstanceContentType> {
        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row = instance_content_sheet.row(content_id as u32)?;

        InstanceContentType::from_repr(instance_content_row.InstanceContentType())
    }

    /// Gets the order of the mount.
    pub fn find_mount_order(&mut self, mount_id: u32) -> Option<i16> {
        let config = get_config();
        let instance_content_sheet =
            MountSheet::read_from(&mut self.resource, config.world.language()).unwrap();
        let mount_row = instance_content_sheet.row(mount_id)?;

        Some(mount_row.Order())
    }

    /// Gets the Item ID of the Orchestrion Roll.
    pub fn find_orchestrion_item_id(&mut self, orchestrion_id: u32) -> Option<u32> {
        for (id, row) in self.item_sheet.into_iter().flatten_subrows() {
            // If filter_group is 32, then this item is an Orchestrion Roll...
            if row.FilterGroup() != 32 {
                continue;
            }

            if row.AdditionalData() == orchestrion_id {
                return Some(id);
            }
        }

        None
    }

    /// Gets the Set/Zone of the Aether Current
    pub fn find_aether_current_set(&mut self, aether_current_id: i32) -> Option<u32> {
        let sheet =
            AetherCurrentCompFlgSetSheet::read_from(&mut self.resource, Language::None).unwrap();

        // Start searching for Zone ID
        for (id, row) in sheet.into_iter().flatten_subrows() {
            let aether_currents = row.AetherCurrents();
            if aether_currents.contains(&aether_current_id) {
                return Some(id);
            }
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

        let row = aether_current_comp_flg_set_sheet.row(aether_current_comp_flg_set_id)?;

        let aether_currents_from_zone = row.AetherCurrents();
        Some(aether_currents_from_zone.to_vec())
    }

    pub fn get_screenimage_from_aether_current_comp_flg_set(
        &mut self,
        aether_current_comp_flg_set_id: u32,
    ) -> Option<u32> {
        let aether_current_comp_flg_set_to_screenimage =
            get_aether_current_comp_flg_set_to_screenimage();
        Some(aether_current_comp_flg_set_to_screenimage[&aether_current_comp_flg_set_id])
    }

    /// Returns the internal script name for this CustomTalk event.
    pub fn get_custom_talk_name(&mut self, custom_talk_id: u32) -> String {
        let row = self.custom_talk_sheet.row(custom_talk_id).unwrap();

        row.Name().to_string()
    }

    /// Returns the internal script name for this Opening event.
    pub fn get_opening_name(&mut self, opening_id: u32) -> String {
        let sheet = OpeningSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(opening_id).unwrap();

        row.Name().to_string()
    }

    /// Returns data useful for performing item actions.
    pub fn lookup_item_action_data(&mut self, item_id: u32) -> Option<(u16, [u16; 9], u32)> {
        if let Some(row) = self.item_sheet.row(item_id) {
            let additional_data = row.AdditionalData();

            let item_action_sheet =
                ItemActionSheet::read_from(&mut self.resource, Language::None).ok()?;
            let item_action_row = item_action_sheet.row(row.ItemAction() as u32)?;

            return Some((
                item_action_row.Action(),
                item_action_row.Data(),
                additional_data,
            ));
        }

        None
    }

    /// Returns the target event for a given PreHandler event.
    pub fn get_pre_handler_target(&mut self, pre_handler_id: u32) -> Option<u32> {
        let config = get_config();
        let sheet = PreHandlerSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        let row = sheet.row(pre_handler_id)?;

        Some(row.Target())
    }

    /// Returns a list of SwitchTalkRows for a given id.
    pub fn get_switch_talk_subrows(
        &mut self,
        switch_talk_id: u32,
    ) -> Vec<(u16, SwitchTalkVariationRow<'_>)> {
        let subrows = self
            .switch_talk_sheet
            .into_iter()
            .filter(|(row_id, _)| switch_talk_id == *row_id)
            .take(1)
            .next();
        subrows.map(|(_, subrows)| subrows).unwrap_or_default()
    }

    /// Returns the target Transform Row ID for a given selected NPC. (Only applicable to the Halloween Transform NPC.)
    pub fn get_halloween_npc_transform(&mut self, npc_id: u32) -> Option<u16> {
        let config = get_config();
        let sheet =
            HalloweenNpcSelectSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        let row = sheet.row(npc_id)?;

        Some(row.Transformation())
    }

    /// Returns the internal script name for this Quest event.
    pub fn get_quest_name(&mut self, quest_id: u32) -> String {
        let row = self.quest_sheet.row(quest_id).unwrap();

        row.Id().to_string()
    }

    /// Returns the target event of a TopicSelect event.
    pub fn get_topic_select_target(
        &mut self,
        topic_select_id: u32,
        selected_index: usize,
    ) -> Option<u32> {
        let config = get_config();
        let sheet =
            TopicSelectSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        let row = sheet.row(topic_select_id)?;

        Some(row.Shop()[selected_index])
    }

    /// Returns the rewards for this Quest, EXP and Gil respectively.
    pub fn get_quest_rewards(&mut self, quest_id: u32) -> (u16, u32) {
        let row = self.quest_sheet.row(quest_id).unwrap();

        (row.ExpFactor(), row.GilReward())
    }

    /// Returns the max EXP or the exp "needed to grow" for a given level.
    pub fn get_max_exp(&mut self, level: u32) -> i32 {
        let row = self.param_grow_sheet.row(level).unwrap();

        row.ExpToNext()
    }

    /// Gets the short name for a given content finder condition.
    pub fn get_content_short_name(&mut self, content_finder_row_id: u16) -> Option<String> {
        let config = get_config();
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, config.world.language())
                .unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_row_id as u32)?;

        Some(content_finder_row.ShortCode().to_string())
    }

    /// Returns the DefaultTalk for a given FateShop and rank.
    pub fn get_fate_default_talk(&mut self, fate_shop_id: u32, rank: u8) -> u32 {
        let sheet = FateShopSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(fate_shop_id).unwrap();

        row.DefaultTalk()[rank as usize]
    }

    /// Returns the pop type for this EObj.
    pub fn get_eobj_pop_type(&mut self, eobj_id: u32) -> u8 {
        let row = self.eobj_sheet.row(eobj_id).unwrap();

        row.PopType()
    }

    /// Returns the InstanceContent for a given ContentFinderCondition id.
    pub fn find_content_for_content_finder_id(
        &mut self,
        content_finder_row_id: u16,
    ) -> Option<u16> {
        let config = get_config();
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, config.world.language())
                .unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_row_id as u32)?;

        Some(content_finder_row.Content())
    }

    /// Returns the time limit in minutes for a given InstanceContent id.
    pub fn find_content_time_limit(&mut self, instance_content_id: u16) -> Option<u16> {
        let sheet = InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(instance_content_id as u32)?;

        Some(row.TimeLimitmin())
    }

    /// Returns information about a specific GimmickRect.
    pub fn get_gimmick_rect_info(&mut self, gimmick_rect_id: u32) -> Option<GimmickRectRow<'_>> {
        self.gimmick_rect_sheet.row(gimmick_rect_id)
    }

    /// Returns information about a specific GimmickRect based on the layout id.
    pub fn lookup_gimmick_rect(&mut self, layout_id: u32) -> Option<GimmickRectRow<'_>> {
        self.gimmick_rect_sheet
            .into_iter()
            .flatten_subrows()
            .map(|(_, row)| row)
            .find(|row| row.LayoutID() == layout_id)
    }

    /// Returns the data associated with this EObj.
    pub fn get_eobj_data(&mut self, eobj_id: u32) -> u32 {
        let row = self.eobj_sheet.row(eobj_id).unwrap();

        row.Data()
    }

    /// Returns the Map column value on the TerritoryType sheet. Used for revealing portions of ingame maps.
    pub fn get_territory_info_map_data(&mut self, zone_id: u16) -> Option<u16> {
        let row = self.territory_type_sheet.row(zone_id.into())?;
        Some(row.Map())
    }

    /// Returns the PlaceNameZone column value on the TerritoryType sheet. Used for determining if a zone is located in a certain region in the world.
    pub fn get_territory_placenamezone_data(&mut self, zone_id: u16) -> Option<u16> {
        let row = self.territory_type_sheet.row(zone_id.into())?;
        Some(row.PlaceNameZone())
    }

    /// Returns the entrance ID for this content finder condition.
    pub fn get_content_entrance_id(&mut self, content_finder_id: u16) -> Option<u32> {
        let config = get_config();
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, config.world.language())
                .unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_id as u32)?;

        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row =
            instance_content_sheet.row(content_finder_row.Content() as u32)?;

        Some(instance_content_row.LGBEventRange())
    }

    /// Returns the list of applicable classjob IDs based on the ClassJobCategory.
    pub fn get_applicable_classjobs(&mut self, classjob_category_id: u16) -> Vec<u8> {
        let config = get_config();

        let exh = self
            .resource
            .read_excel_sheet_header("ClassJobCategory")
            .unwrap();
        let sheet = self
            .resource
            .read_excel_sheet(&exh, "ClassJobCategory", config.world.language())
            .unwrap();

        let mut classjobs = Vec::new();
        let row = sheet.row(classjob_category_id as u32).unwrap();

        let applicable_classes = &row.columns[1..]; // First column is the label e.g. "Disciple of War"
        for (i, applicable) in applicable_classes.iter().enumerate() {
            if applicable.into_bool().copied().unwrap_or_default() {
                classjobs.push(i as u8);
            }
        }

        classjobs
    }

    /// Gets the soul crystal item ID for the classjob, if applicable.
    pub fn get_soul_crystal_item_id(&mut self, classjob_id: u16) -> Option<u32> {
        let row = self.classjob_sheet.row(classjob_id as u32)?;

        let item_id = row.ItemSoulCrystal();
        if item_id != 0 {
            return Some(item_id);
        }
        None
    }

    /// Gets the starting level for the classjob.
    pub fn get_starting_level(&mut self, classjob_id: u16) -> Option<u8> {
        let row = self.classjob_sheet.row(classjob_id as u32)?;

        Some(row.StartingLevel())
    }

    /// Returns information about the BaseParam.
    pub fn get_base_param(&mut self, base_param_id: u16) -> Option<BaseParamRow<'_>> {
        self.base_param_sheet.row(base_param_id as u32)
    }

    /// Returns the ParamGrow for this level.
    pub fn get_param_grow(&mut self, level: u32) -> Option<ParamGrowRow<'_>> {
        self.param_grow_sheet.row(level)
    }

    /// Returns the ParamGrow for this level.
    pub fn get_class_job_modifiers(&mut self, classjob_id: u32) -> Option<Modifiers> {
        let row = self.classjob_sheet.row(classjob_id)?;

        Some(Modifiers {
            hp: row.ModifierHitPoints(),
            mp: row.ModifierManaPoints(),
            strength: row.ModifierStrength(),
            vitality: row.ModifierVitality(),
            dexterity: row.ModifierDexterity(),
            intelligence: row.ModifierIntelligence(),
            mind: row.ModifierMind(),
            piety: row.ModifierPiety(),
        })
    }

    /// Gets the classjob ID associated with this soul crystal item ID.
    pub fn get_applicable_classjob(&mut self, soul_crystal_id: u32) -> Option<u32> {
        for (id, row) in self.classjob_sheet.into_iter().flatten_subrows() {
            if row.ItemSoulCrystal() == soul_crystal_id {
                return Some(id);
            }
        }

        None
    }

    /// Returns the layout IDs for the map effects of this InstanceContent.
    pub fn get_map_effects(&mut self, content_id: u32) -> Option<Vec<i32>> {
        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row = instance_content_sheet.row(content_id)?;
        let content_id = instance_content_row.ContentDirectorManagedSG();

        let sheet =
            ContentDirectorManagedSGSheet::read_from(&mut self.resource, Language::None).ok()?;
        let subrows = sheet
            .into_iter()
            .find(|(row_id, _)| *row_id == content_id as u32)?;

        Some(
            subrows
                .1
                .iter()
                .map(|(_, row)| row.Unknown0()) // FIXME: This will be renamed to LayoutId in the future.
                .collect(),
        )
    }

    /// Returns a list of variable name and value pairs for this Opening.
    pub fn get_opening_variables(&mut self, opening_id: u32) -> Vec<(String, u32)> {
        let sheet = OpeningSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(opening_id).unwrap();

        let mut translated_variables = Vec::new();
        for variable in row.Variables() {
            let name = variable.Name;
            let value = variable.Value;

            if !name.is_empty() {
                translated_variables.push((name.to_string(), value));
            }
        }

        translated_variables
    }

    /// Returns a list of variable name and value pairs for this Quest.
    pub fn get_quest_variables(&mut self, quest_id: u32) -> Vec<(String, u32)> {
        let row = self.quest_sheet.row(quest_id).unwrap();

        let mut translated_variables = Vec::new();
        for variable in row.QuestParams() {
            let name = variable.ScriptInstruction;
            let value = variable.ScriptArg;

            if !name.is_empty() {
                translated_variables.push((name.to_string(), value));
            }
        }

        translated_variables
    }

    /// Returns a list of variable name and value pairs for this CustomTalk.
    pub fn get_custom_talk_variables(&mut self, custom_talk_id: u32) -> Vec<(String, u32)> {
        let row = self.custom_talk_sheet.row(custom_talk_id).unwrap();

        let mut translated_variables = Vec::new();
        for variable in row.Script() {
            let name = variable.ScriptInstruction;
            let value = variable.ScriptArg;

            if !name.is_empty() {
                translated_variables.push((name.to_string(), value));
            }
        }

        translated_variables
    }

    /// Tries to guess the latest items for the Fitting Shop.
    /// Since this is server-controlled, we will never know - but we can guess!
    pub fn get_latest_fittingshop_display_ids(&mut self) -> [u8; 8] {
        let sheet =
            FittingShopCategoryItemSheet::read_from(&mut self.resource, Language::None).unwrap();

        let mut display_id_set = HashSet::new();

        // Assuming row 1 is "Latest Trends".
        let subrows = sheet
            .into_iter()
            .filter(|(row_id, _)| 1 == *row_id)
            .take(1)
            .next()
            .unwrap()
            .1;
        for (_, subrow) in subrows {
            // This is needed to weed out certain items that are invalid, but still in the Excel sheet.
            // FIXME: Name will change to Item in the future.
            let item_id = subrow.Unknown0();
            if item_id == 0 || (item_id < 1000000 && !self.is_item_valid(item_id as u32)) {
                continue;
            }

            // FIXME: Name will change to DisplayId in the future.
            display_id_set.insert(subrow.Unknown1());
        }

        // Sort so the highest display id is sent first.
        let mut display_id_vec: Vec<u8> = display_id_set.into_iter().collect();
        display_id_vec.resize(8, 0); // ensure we are at least eight items
        display_id_vec.sort();
        display_id_vec.reverse();

        let mut display_id_arr = [0u8; 8];
        display_id_arr.copy_from_slice(&display_id_vec);

        display_id_arr
    }

    /// Simple heurestic to determine if the Item is actually filled with useful/valid data.
    pub fn is_item_valid(&mut self, item_id: u32) -> bool {
        let Some(row) = self.item_sheet.row(item_id) else {
            return false;
        };

        !row.Singular().is_empty()
    }

    /// Returns a list of variable name and value pairs for all Aetherytes.
    pub fn get_aetheryte_variables(&mut self) -> Vec<(String, u32)> {
        let sheet =
            AetheryteSystemDefineSheet::read_from(&mut self.resource, Language::None).unwrap();

        let mut variables = Vec::new();
        for (_, row) in sheet.into_iter().flatten_subrows() {
            let name = row.Text();
            let value = row.DefineValue();

            if !name.is_empty() {
                variables.push((name.to_string(), value));
            }
        }

        variables
    }

    /// Returns the base id, level and count for a GatheringPoint.
    pub fn get_gathering_point(&mut self, id: u32) -> (i32, u8, u8) {
        let sheet = GatheringPointSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(id).unwrap();

        let base_id = row.GatheringPointBase();
        let base_sheet =
            GatheringPointBaseSheet::read_from(&mut self.resource, Language::None).unwrap();
        let base_row = base_sheet.row(base_id as u32).unwrap();

        (base_id, base_row.GatheringLevel(), row.Count())
    }

    /// Returns the item list for a gathering point.
    pub fn get_gathering_point_items(&mut self, id: u32) -> [i32; 8] {
        let sheet = GatheringPointSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(id).unwrap();

        let base_id = row.GatheringPointBase();
        let base_sheet =
            GatheringPointBaseSheet::read_from(&mut self.resource, Language::None).unwrap();
        let base_row = base_sheet.row(base_id as u32).unwrap();

        base_row.Item().map(|x| x)
    }

    /// Converts from a GatheringItem to a regular Item.
    pub fn convert_gathering_point_item(&mut self, id: u32) -> i32 {
        let sheet = GatheringItemSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(id).unwrap();

        row.Item()
    }

    /// Returns the ClassJobCategory for this item.
    pub fn get_item_classjobcategory(&mut self, item_id: u32) -> u8 {
        let row = self.item_sheet.row(item_id).unwrap();
        row.ClassJobCategory()
    }

    /// Returns a Recipe.
    pub fn get_recipe(&mut self, id: u32) -> Recipe {
        let sheet = RecipeSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(id).unwrap();

        Recipe {
            id,
            item_id: row.ItemResult(),
        }
    }

    /// Returns a CraftAction's animation start/end.
    pub fn get_craft_action_animations(&mut self, id: u32) -> (u16, u16) {
        let sheet = CraftActionSheet::read_from(&mut self.resource, Language::English).unwrap();
        let row = sheet.row(id).unwrap();

        (row.AnimationStart(), row.AnimationEnd())
    }

    /// Returns a list of priorities for each online status.
    pub fn online_status_priorities(&mut self) -> Vec<u8> {
        let mut priorities = Vec::new();

        let sheet = OnlineStatusSheet::read_from(&mut self.resource, Language::English).unwrap();
        for (_, row) in sheet.into_iter().flatten_subrows() {
            priorities.push(row.Priority());
        }

        priorities
    }

    /// Returns the synced level for this content.
    pub fn find_content_synced_level(&mut self, content_finder_row_id: u16) -> Option<u8> {
        let config = get_config();
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, config.world.language())
                .unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_row_id as u32)?;

        Some(content_finder_row.ClassJobLevelSync()).filter(|x| *x != 0)
    }

    /// Returns the attributes for a given item level;
    pub fn get_item_level_attributes(&mut self, item_level: u16) -> [u16; 6] {
        let row = self.item_level_sheet.row(item_level as u32).unwrap();

        [
            row.Strength(),
            row.Dexterity(),
            row.Vitality(),
            row.Intelligence(),
            row.Mind(),
            row.Piety(),
        ]
    }

    pub fn get_action_cooldown_group(&mut self, id: u32) -> u8 {
        let row = self.action_sheet.row(id).unwrap();

        row.CooldownGroup()
    }

    /// Checks if this zone is associated with a ContentFinderCondition.
    pub fn is_zone_associated_with_content(&mut self, zone_id: u16) -> bool {
        let Some(row) = self.territory_type_sheet.row(zone_id.into()) else {
            return false;
        };
        row.ContentFinderCondition() != 0
    }

    /// Tells us if this item belongs to the Seasonal Miscellany or Miscellany categories.
    pub fn item_is_misc(&mut self, id: u32) -> bool {
        let Some(row) = self.item_sheet.row(id) else {
            return false;
        };

        let is_seasonal_misc = row.ItemUICategory() == 85;
        let is_misc = row.ItemUICategory() == 61;

        is_seasonal_misc || is_misc
    }

    /// Checks if this zone is valid.
    pub fn is_zone_valid(&mut self, zone_id: u16) -> bool {
        let Some(row) = self.territory_type_sheet.row(zone_id.into()) else {
            return false;
        };
        !row.Bg().is_empty()
    }

    /// Returns the emote mode (if any), really only relevant for persistent/loopable emotes.
    pub fn get_emote_mode(&mut self, emote_id: u32) -> Option<u8> {
        let config = get_config();
        let sheet = EmoteSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        let row = sheet.row(emote_id)?;

        let mode = row.EmoteMode();
        if mode != 0 {
            return Some(mode);
        }
        None
    }

    pub fn get_mount_id_from_name(&mut self, mount_name: String) -> Option<u16> {
        let config = get_config();
        let sheet = MountSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        for (id, row) in sheet.into_iter().flatten_subrows() {
            if row
                .Singular()
                .to_lowercase()
                .contains(&mount_name.to_lowercase())
            {
                return Some(id as u16);
            }
        }

        None
    }

    /// Returns the list of battalions that should be considered an enemy to this battalion.
    pub fn get_battalion_enemies(&mut self, battalion_id: u32) -> Vec<bool> {
        // TODO: will change to IsEnemyTo in the future

        let row = self.battalion_sheet.row(battalion_id).unwrap();
        vec![
            row.Unknown0(),
            row.Unknown1(),
            row.Unknown2(),
            row.Unknown3(),
            row.Unknown4(),
            row.Unknown5(),
            row.Unknown6(),
            row.Unknown7(),
            row.Unknown8(),
            row.Unknown9(),
            row.Unknown10(),
            row.Unknown11(),
            row.Unknown12(),
            row.Unknown13(),
            row.Unknown14(),
        ]
    }

    /// Returns a ContentFinderCondition for a given roulette.
    pub fn pick_roulette_duty(&mut self, roulette: Roulette) -> u32 {
        let config = get_config();
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, config.world.language())
                .unwrap();

        let rows: Vec<u32> = content_finder_sheet
            .into_iter()
            .flatten_subrows()
            .filter(|(_, row)| match roulette {
                Roulette::Leveling => row.LevelingRoulette(),
                Roulette::HighLevel => row.HighLevelRoulette(),
                Roulette::MSQ => row.MSQRoulette(),
                Roulette::GuildHest => row.GuildHestRoulette(),
                Roulette::Expert => row.ExpertRoulette(),
                Roulette::Trial => row.TrialRoulette(),
                Roulette::DailyFrontline => row.DailyFrontlineChallenge(),
                Roulette::LevelCap => row.LevelCapRoulette(),
                Roulette::Mentor => row.MentorRoulette(),
                Roulette::Alliance => row.AllianceRoulette(),
                Roulette::NormalRaid => row.NormalRaidRoulette(),
                Roulette::CrystallineConflictCasual => row.Unknown27(), // NOTE: Will be CrystallineConflictCasualRoulette in the future
                Roulette::CrystallineConflictRanked => row.Unknown28(), // NOTE: Will be CrystallineConflictRankedRoulette in the future
            })
            .map(|(id, _)| id)
            .collect();

        fastrand::choice(rows).unwrap()
    }

    /// Returns the name ID for a given NpcYell.
    pub fn get_npc_yell_name_id(&mut self, npc_yell_id: u32) -> Option<u32> {
        let config = get_config();
        let sheet = NpcYellSheet::read_from(&mut self.resource, config.world.language()).ok()?;
        let row = sheet.row(npc_yell_id)?;

        Some(row.Unknown0()) // NOTE: will be Name in the future
    }
}

impl mlua::UserData for GameData {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("get_pre_handler_target", |_, this, pre_handler_id: u32| {
            Ok(this.get_pre_handler_target(pre_handler_id))
        });
        methods.add_method_mut("get_halloween_npc_transform", |_, this, npc_id: u32| {
            Ok(this.get_halloween_npc_transform(npc_id))
        });
        methods.add_method_mut(
            "get_topic_select_target",
            |_, this, (topic_select_id, selected_topic): (u32, usize)| {
                Ok(this.get_topic_select_target(topic_select_id, selected_topic))
            },
        );
        methods.add_method_mut(
            "get_fate_default_talk",
            |_, this, (fate_shop_id, rank): (u32, u8)| {
                Ok(this.get_fate_default_talk(fate_shop_id, rank))
            },
        );
    }
}

/// Simple enum for GameData::get_territory_name
pub enum TerritoryNameKind {
    Internal,
    Region,
    Place,
}

/// Wrapper around SqPackResource to let us spy when it reads files
#[derive(Clone)]
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
