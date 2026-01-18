use std::path::PathBuf;

use icarus::Action::ActionSheet;
use icarus::AetherCurrentCompFlgSet::AetherCurrentCompFlgSetSheet;
use icarus::Aetheryte::AetheryteSheet;
use icarus::BNpcBase::BNpcBaseSheet;
use icarus::ClassJob::ClassJobSheet;
use icarus::ClassJobCategory::ClassJobCategorySheet;
use icarus::ContentFinderCondition::ContentFinderConditionSheet;
use icarus::CustomTalk::CustomTalkSheet;
use icarus::EObj::EObjSheet;
use icarus::EquipSlotCategory::EquipSlotCategorySheet;
use icarus::FateShop::FateShopSheet;
use icarus::GilShopItem::GilShopItemSheet;
use icarus::GimmickRect::GimmickRectSheet;
use icarus::HalloweenNpcSelect::HalloweenNpcSelectSheet;
use icarus::InstanceContent::InstanceContentSheet;
use icarus::Item::ItemSheet;
use icarus::ItemAction::ItemActionSheet;
use icarus::ModelChara::ModelCharaSheet;
use icarus::Mount::MountSheet;
use icarus::Opening::OpeningSheet;
use icarus::ParamGrow::ParamGrowSheet;
use icarus::PlaceName::PlaceNameSheet;
use icarus::PreHandler::PreHandlerSheet;
use icarus::Quest::QuestSheet;
use icarus::SpecialShop::SpecialShopSheet;
use icarus::SwitchTalkVariation::{SwitchTalkVariationRow, SwitchTalkVariationSheet};
use icarus::TerritoryType::TerritoryTypeSheet;
use icarus::TopicSelect::TopicSelectSheet;
use icarus::WarpLogic::WarpLogicSheet;
use icarus::WeatherRate::WeatherRateSheet;
use icarus::{Tribe::TribeSheet, Warp::WarpSheet};
use physis::Language;
use physis::resource::{Resource, ResourceResolver, SqPackResource, UnpackedResource};

use kawari::common::timestamp_secs;
use kawari::common::{InstanceContentType, get_aether_current_comp_flg_set_to_screenimage};
use kawari::{common::Attributes, config::get_config};

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
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct detailing various information about an item, pulled from the Items sheet.
#[derive(Debug, Default, Clone)]
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
    /// The item's ClassJobCategory.
    pub classjob_category: u8,
}

#[derive(Debug)]
pub enum ItemInfoQuery {
    ById(u32),
    ByName(String),
}

#[derive(Debug)]
pub struct GimmickRectInfo {
    pub layout_id: u32,
    pub params: [u32; 8],
    pub trigger_in: u8,
    pub trigger_out: u8,
}

impl GameData {
    pub fn new() -> Self {
        let config = get_config();

        // setup resolvers
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

        // We want to preload all index files, because the cost for not doing this can be high.
        // For example: someone travels to a new zone (that wasn't previously loaded), so the server has to basically halt to read the index file from disk.
        // Index files are small and will take up very little memory once serialized, so this is an easy optimization.
        // (We could move this to an option, if you prefer marginally faster loading times over I/O overhead.)
        sqpack_resource.sqpack_resource.preload_index_files();

        let mut resource_resolver = ResourceResolver::new();
        for path in config.filesystem.additional_search_paths {
            let unpacked_resource = UnpackedResource::from_existing(&path);
            resource_resolver.add_source(unpacked_resource);
        }
        resource_resolver.add_source(sqpack_resource);

        let mut classjob_exp_indexes = Vec::new();

        let sheet = ClassJobSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read ClassJobSheet, does the Excel files exist?");
        for (_, row) in sheet.into_iter().flatten_subrows() {
            classjob_exp_indexes.push(*row.ExpArrayIndex().into_i8().unwrap());
        }

        let item_sheet = ItemSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read ItemSheet, does the Excel files exist?");

        let weather_rate_sheet =
            WeatherRateSheet::read_from(&mut resource_resolver, Language::None)
                .expect("Failed to read WeatherRateSheet, does the Excel files exist?");

        let quest_sheet = QuestSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read Quest, does the Excel files exist?");

        let territory_type_sheet =
            TerritoryTypeSheet::read_from(&mut resource_resolver, Language::None)
                .expect("Failed to read TerritoryTypeSheet, does the Excel files exist?");

        let warp_sheet = WarpSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read Warp, does the Excel files exist?");

        let action_sheet = ActionSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read Action, does the Excel files exist?");

        let place_name_sheet = PlaceNameSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read PlaceName, does the Excel files exist?");

        let custom_talk_sheet =
            CustomTalkSheet::read_from(&mut resource_resolver, Language::English)
                .expect("Failed to read CustomTalk, does the Excel files exist?");

        let tribe_sheet = TribeSheet::read_from(&mut resource_resolver, Language::English)
            .expect("Failed to read Tribe, does the Excel files exist?");

        let eobj_sheet = EObjSheet::read_from(&mut resource_resolver, Language::None)
            .expect("Failed to read EObj, does the Excel files exist?");

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
        }
    }

    /// Gets the starting city-state from a given class/job id.
    pub fn get_citystate(&mut self, classjob_id: u16) -> Option<u8> {
        let sheet = ClassJobSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(classjob_id as u32)?;

        row.StartingTown().into_u8().copied()
    }

    pub fn get_racial_base_attributes(&mut self, tribe_id: u8) -> Option<Attributes> {
        // The Tribe Excel sheet only has deltas (e.g. 2 or -2) which are applied to a base 20 number... from somewhere
        let base_stat = 20;

        let row = self.tribe_sheet.row(tribe_id as u32)?;

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
        match query {
            ItemInfoQuery::ById(ref query_item_id) => {
                if let Some(row) = self.item_sheet.row(*query_item_id) {
                    result = Some((row, *query_item_id));
                }
            }

            ItemInfoQuery::ByName(ref query_item_name) => {
                for (id, row) in self.item_sheet.into_iter().flatten_subrows() {
                    if let Some(name) = row.Name().into_string()
                        && name
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
            let item_info = ItemInfo {
                id: item_id,
                name: matched_row.Name().into_string().unwrap().clone(),
                price_mid: *matched_row.PriceMid().into_u32().unwrap(),
                price_low: *matched_row.PriceLow().into_u32().unwrap(),
                equip_category: *matched_row.EquipSlotCategory().into_u8().unwrap(),
                primary_model_id: *matched_row.ModelMain().into_u64().unwrap(),
                sub_model_id: *matched_row.ModelSub().into_u64().unwrap(),
                stack_size: *matched_row.StackSize().into_u32().unwrap(),
                item_level: *matched_row.LevelItem().into_u16().unwrap(),
                classjob_category: *matched_row.ClassJobCategory().into_u8().unwrap(),
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

        let pop_range_id = row.PopRange().into_u32()?;
        let zone_id = row.TerritoryType().into_u16()?;

        Some((*pop_range_id, *zone_id))
    }

    /// Returns the warp logic name (if any) for this Warp.
    pub fn get_warp_logic_name(&mut self, warp_id: u32) -> String {
        let row = self.warp_sheet.row(warp_id).unwrap();

        let warp_logic_id = row.WarpLogic().into_u16().unwrap();

        let warp_logic_sheet =
            WarpLogicSheet::read_from(&mut self.resource, Language::English).unwrap();
        let warp_logic_row = warp_logic_sheet.row(*warp_logic_id as u32).unwrap();

        warp_logic_row
            .WarpName()
            .into_string()
            .cloned()
            .unwrap_or_default()
    }

    pub fn get_aetheryte(&mut self, aetheryte_id: u32) -> Option<(u32, u16)> {
        let sheet = AetheryteSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(aetheryte_id)?;

        // TODO: just look in the level sheet?
        let pop_range_id = row.Level()[0].into_u32()?;
        let zone_id = row.Territory().into_u16()?;

        Some((*pop_range_id, *zone_id))
    }

    /// Checks if it's a big Aetheryte (true) or just a shard (false.)
    pub fn is_aetheryte(&mut self, aetheryte_id: u32) -> bool {
        let sheet = AetheryteSheet::read_from(&mut self.resource, Language::English).unwrap();
        let row = sheet.row(aetheryte_id).unwrap();

        row.IsAetheryte().into_bool().cloned().unwrap_or_default()
    }

    /// Retrieves a zone's internal name, place name or parent region name.
    pub fn get_territory_name(&mut self, zone_id: u32, which: TerritoryNameKind) -> Option<String> {
        let row = self.territory_type_sheet.row(zone_id)?;

        let offset = match which {
            TerritoryNameKind::Internal => {
                return row.Name().into_string().cloned();
            }
            TerritoryNameKind::Region => row.PlaceNameRegion().into_u16()?,
            TerritoryNameKind::Place => row.PlaceName().into_u16()?,
        };

        let row = self.place_name_sheet.row(*offset as u32)?;
        let value = row.Name().into_string()?;

        Some(value.clone())
    }

    /// Turn an equip slot category id into a slot for the equipped inventory
    pub fn get_equipslot_category(&mut self, equipslot_id: u8) -> Option<u16> {
        let sheet = EquipSlotCategorySheet::read_from(&mut self.resource, Language::None).ok()?;
        let row = sheet.row(equipslot_id as u32)?;

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
        let row = self.action_sheet.row(action_id)?;

        row.Cast100ms().into_u16().copied()
    }

    /// Calculates the current weather at the current time
    // TODO: instead allow targetting a specific time to calculate forcecasts
    pub fn get_weather_rate(&mut self, weather_rate_id: u32) -> Option<i32> {
        let row = self.weather_rate_sheet.row(weather_rate_id)?;

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
        let row = self.territory_type_sheet.row(zone_id)?;

        let weather_rate_id = row.WeatherRate().into_u8()?;
        self.get_weather_rate(*weather_rate_id as u32)
    }

    /// Gets the array index used in EXP & levels.
    pub fn get_exp_array_index(&self, classjob_id: u16) -> Option<i8> {
        self.classjob_exp_indexes.get(classjob_id as usize).copied()
    }

    /// Gets the job index for a given class.
    pub fn get_job_index(&mut self, classjob_id: u16) -> Option<u8> {
        let sheet = ClassJobSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(classjob_id as u32)?;

        row.JobIndex().into_u8().cloned()
    }

    /// Gets the item and its cost from the specified shop.
    pub fn get_gilshop_item(&mut self, gilshop_id: u32, index: u16) -> Option<ItemInfo> {
        let sheet = GilShopItemSheet::read_from(&mut self.resource, Language::None).ok()?;
        let row = sheet.subrow(gilshop_id, index)?;
        let item_id = row.Item().into_i32()?;

        self.get_item_info(ItemInfoQuery::ById(*item_id as u32))
    }

    /// Gets the item and its cost from the specified SpecialShop.
    pub fn get_specialshop_item(&mut self, gilshop_id: u32, index: u16) -> Option<ItemInfo> {
        let sheet = SpecialShopSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(gilshop_id)?;
        let item_id = row.Item()[index as usize].Item[0].into_i32()?; // TODO: why are there two items?

        self.get_item_info(ItemInfoQuery::ById(*item_id as u32))
    }

    /// Gets the zone id for the given ContentFinderCondition ID.
    pub fn find_zone_for_content(&mut self, content_id: u16) -> Option<u16> {
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, Language::English).unwrap();
        let content_finder_row = content_finder_sheet.row(content_id as u32)?;

        content_finder_row.TerritoryType().into_u16().copied()
    }

    /// Grabs needed BattleNPC information such as their name, model id and more.
    pub fn find_bnpc(&mut self, id: u32) -> Option<u16> {
        let bnpc_sheet = BNpcBaseSheet::read_from(&mut self.resource, Language::None).unwrap();
        let bnpc_row = bnpc_sheet.row(id)?;

        let model_row_id = bnpc_row.ModelChara().into_u16()?;
        let model_sheet = ModelCharaSheet::read_from(&mut self.resource, Language::None).unwrap();
        let model_row = model_sheet.row(*model_row_id as u32)?;

        model_row.Model().into_u16().copied()
    }

    /// Gets the content type for the given InstanceContent.
    pub fn find_type_for_content(&mut self, content_id: u16) -> Option<InstanceContentType> {
        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row = instance_content_sheet.row(content_id as u32)?;

        InstanceContentType::from_repr(
            instance_content_row
                .InstanceContentType()
                .into_u8()
                .copied()?,
        )
    }

    /// Gets the order of the mount.
    pub fn find_mount_order(&mut self, mount_id: u32) -> Option<i16> {
        let instance_content_sheet =
            MountSheet::read_from(&mut self.resource, Language::English).unwrap();
        let mount_row = instance_content_sheet.row(mount_id)?;

        mount_row.Order().into_i16().copied()
    }

    /// Gets the Item ID of the Orchestrion Roll.
    pub fn find_orchestrion_item_id(&mut self, orchestrion_id: u32) -> Option<u32> {
        for (id, row) in self.item_sheet.into_iter().flatten_subrows() {
            // If filter_group is 32, then this item is an Orchestrion Roll...
            if *row.FilterGroup().into_u8()? != 32 {
                continue;
            }

            if *row.AdditionalData().into_u32()? == orchestrion_id {
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
            let aether_currents: Vec<i32> = row
                .AetherCurrents()
                .iter()
                .filter_map(|x| x.into_i32().cloned())
                .collect();
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

        let aether_currents_from_zone = row
            .AetherCurrents()
            .iter()
            .map(|x| *x.into_i32().unwrap())
            .filter(|x| *x != 0)
            .collect();

        Some(aether_currents_from_zone)
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

        row.Name().into_string().cloned().unwrap_or_default()
    }

    /// Returns the internal script name for this Opening event.
    pub fn get_opening_name(&mut self, opening_id: u32) -> String {
        let sheet = OpeningSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(opening_id).unwrap();

        row.Name().into_string().cloned().unwrap_or_default()
    }

    /// Returns data useful for performing item actions.
    pub fn lookup_item_action_data(&mut self, item_id: u32) -> Option<(u16, [u16; 9], u32)> {
        if let Some(row) = self.item_sheet.row(item_id) {
            let additional_data = row.AdditionalData().into_u32()?;

            let item_action_sheet =
                ItemActionSheet::read_from(&mut self.resource, Language::None).ok()?;
            let item_action_row =
                item_action_sheet.row(row.ItemAction().into_u16().cloned()? as u32)?;

            return Some((
                item_action_row.Action().into_u16().cloned()?,
                item_action_row
                    .Data()
                    .map(|x| x.into_u16().cloned().unwrap_or_default()),
                *additional_data,
            ));
        }

        None
    }

    /// Returns the target event for a given PreHandler event.
    pub fn get_pre_handler_target(&mut self, pre_handler_id: u32) -> Option<u32> {
        let sheet = PreHandlerSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(pre_handler_id)?;

        Some(*row.Target().into_u32()?)
    }

    /// Returns a list of SwitchTalkRows for a given id.
    pub fn get_switch_talk_subrows(
        &mut self,
        switch_talk_id: u32,
    ) -> Vec<(u16, SwitchTalkVariationRow)> {
        let sheet =
            SwitchTalkVariationSheet::read_from(&mut self.resource, Language::None).unwrap();
        let subrows = sheet
            .into_iter()
            .filter(|(row_id, _)| switch_talk_id == *row_id)
            .take(1)
            .next();
        subrows.map(|(_, subrows)| subrows).unwrap_or_default()
    }

    /// Returns the target Transform Row ID for a given selected NPC. (Only applicable to the Halloween Transform NPC.)
    pub fn get_halloween_npc_transform(&mut self, npc_id: u32) -> Option<u16> {
        let sheet =
            HalloweenNpcSelectSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(npc_id)?;

        Some(*row.Transformation().into_u16()?)
    }

    /// Returns the internal script name for this Quest event.
    pub fn get_quest_name(&mut self, quest_id: u32) -> String {
        let row = self.quest_sheet.row(quest_id).unwrap();

        row.Id().into_string().cloned().unwrap_or_default()
    }

    /// Returns the target event of a TopicSelect event.
    pub fn get_topic_select_target(
        &mut self,
        topic_select_id: u32,
        selected_index: usize,
    ) -> Option<u32> {
        let sheet = TopicSelectSheet::read_from(&mut self.resource, Language::English).ok()?;
        let row = sheet.row(topic_select_id)?;

        Some(*row.Shop()[selected_index].into_u32()?)
    }

    /// Returns the rewards for this Quest, EXP and Gil respectively.
    pub fn get_quest_rewards(&mut self, quest_id: u32) -> (u16, u32) {
        let row = self.quest_sheet.row(quest_id).unwrap();

        (
            row.ExpFactor().into_u16().cloned().unwrap_or_default(),
            row.GilReward().into_u32().cloned().unwrap_or_default(),
        )
    }

    /// Returns the max EXP or the exp "needed to grow" for a given level.
    pub fn get_max_exp(&mut self, level: u32) -> i32 {
        let sheet = ParamGrowSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(level).unwrap();

        row.ExpToNext().into_i32().cloned().unwrap_or_default()
    }

    /// Gets the short name for a given content finder condition.
    pub fn get_content_short_name(&mut self, content_finder_row_id: u16) -> Option<String> {
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, Language::English).unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_row_id as u32)?;

        content_finder_row.ShortCode().into_string().cloned()
    }

    /// Returns the DefaultTalk for a given FateShop and rank.
    pub fn get_fate_default_talk(&mut self, fate_shop_id: u32, rank: u8) -> u32 {
        let sheet = FateShopSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(fate_shop_id).unwrap();

        row.DefaultTalk()[rank as usize]
            .into_u32()
            .cloned()
            .unwrap_or_default()
    }

    /// Returns the pop type for this EObj.
    pub fn get_eobj_pop_type(&mut self, eobj_id: u32) -> u8 {
        let row = self.eobj_sheet.row(eobj_id).unwrap();

        row.PopType().into_u8().cloned().unwrap_or_default()
    }

    /// Returns the InstanceContent for a given ContentFinderCondition id.
    pub fn find_content_for_content_finder_id(
        &mut self,
        content_finder_row_id: u16,
    ) -> Option<u16> {
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, Language::English).unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_row_id as u32)?;

        content_finder_row.Content().into_u16().copied()
    }

    /// Returns information about a specific GimmickRect.
    pub fn get_gimmick_rect_info(&mut self, gimmick_rect_id: u32) -> Option<GimmickRectInfo> {
        let sheet = GimmickRectSheet::read_from(&mut self.resource, Language::None).unwrap();
        let row = sheet.row(gimmick_rect_id)?;

        Some(GimmickRectInfo {
            layout_id: row.LayoutID().into_u32().copied()?,
            params: row.Params().map(|x| x.into_u32().copied().unwrap()),
            trigger_in: row.TriggerIn().into_u8().copied()?,
            trigger_out: row.TriggerOut().into_u8().copied()?,
        })
    }

    /// Returns the data associated with this EObj.
    pub fn get_eobj_data(&mut self, eobj_id: u32) -> u32 {
        let row = self.eobj_sheet.row(eobj_id).unwrap();

        row.Data().into_u32().cloned().unwrap_or_default()
    }

    /// Returns the Map column value on the TerritoryType sheet. Used for revealing portions of ingame maps.
    pub fn get_territory_info_map_data(&mut self, zone_id: u16) -> Option<u16> {
        let row = self.territory_type_sheet.row(zone_id.into())?;
        row.Map().into_u16().copied()
    }

    /// Returns the entrance ID for this content finder condition.
    pub fn get_content_entrance_id(&mut self, content_finder_id: u16) -> Option<u32> {
        let content_finder_sheet =
            ContentFinderConditionSheet::read_from(&mut self.resource, Language::English).unwrap();
        let content_finder_row = content_finder_sheet.row(content_finder_id as u32)?;

        let instance_content_sheet =
            InstanceContentSheet::read_from(&mut self.resource, Language::None).unwrap();
        let instance_content_row =
            instance_content_sheet.row(content_finder_row.Content().into_u16().copied()? as u32)?;

        instance_content_row.LGBEventRange().into_u32().copied()
    }

    /// Returns the list of applicable classjob IDs based on the ClassJobCategory.
    pub fn get_applicable_classjobs(&mut self, classjob_category_id: u16) -> Vec<u8> {
        let sheet =
            ClassJobCategorySheet::read_from(&mut self.resource, Language::English).unwrap();
        let row = sheet.row(classjob_category_id as u32).unwrap();

        // TODO: find a better way to write this
        let mut classjobs = Vec::new();
        if *row.ADV().into_bool().unwrap() {
            classjobs.push(0);
        }
        if *row.GLA().into_bool().unwrap() {
            classjobs.push(1);
        }
        if *row.PGL().into_bool().unwrap() {
            classjobs.push(2);
        }
        if *row.MRD().into_bool().unwrap() {
            classjobs.push(3);
        }
        if *row.LNC().into_bool().unwrap() {
            classjobs.push(4);
        }
        if *row.ARC().into_bool().unwrap() {
            classjobs.push(5);
        }
        if *row.CNJ().into_bool().unwrap() {
            classjobs.push(6);
        }
        if *row.THM().into_bool().unwrap() {
            classjobs.push(7);
        }
        if *row.CRP().into_bool().unwrap() {
            classjobs.push(8);
        }
        if *row.BSM().into_bool().unwrap() {
            classjobs.push(9);
        }
        if *row.ARM().into_bool().unwrap() {
            classjobs.push(10);
        }
        if *row.GSM().into_bool().unwrap() {
            classjobs.push(11);
        }
        if *row.LTW().into_bool().unwrap() {
            classjobs.push(12);
        }
        if *row.WVR().into_bool().unwrap() {
            classjobs.push(13);
        }
        if *row.ALC().into_bool().unwrap() {
            classjobs.push(14);
        }
        if *row.CUL().into_bool().unwrap() {
            classjobs.push(15);
        }
        if *row.MIN().into_bool().unwrap() {
            classjobs.push(16);
        }
        if *row.BTN().into_bool().unwrap() {
            classjobs.push(17);
        }
        if *row.FSH().into_bool().unwrap() {
            classjobs.push(18);
        }
        if *row.PLD().into_bool().unwrap() {
            classjobs.push(19);
        }
        if *row.MNK().into_bool().unwrap() {
            classjobs.push(20);
        }
        if *row.WAR().into_bool().unwrap() {
            classjobs.push(21);
        }
        if *row.DRG().into_bool().unwrap() {
            classjobs.push(22);
        }
        if *row.BRD().into_bool().unwrap() {
            classjobs.push(23);
        }
        if *row.WHM().into_bool().unwrap() {
            classjobs.push(24);
        }
        if *row.BLM().into_bool().unwrap() {
            classjobs.push(25);
        }
        if *row.ACN().into_bool().unwrap() {
            classjobs.push(26);
        }
        if *row.SMN().into_bool().unwrap() {
            classjobs.push(27);
        }
        if *row.SCH().into_bool().unwrap() {
            classjobs.push(28);
        }
        if *row.ROG().into_bool().unwrap() {
            classjobs.push(29);
        }
        if *row.NIN().into_bool().unwrap() {
            classjobs.push(30);
        }
        if *row.MCH().into_bool().unwrap() {
            classjobs.push(31);
        }
        if *row.DRK().into_bool().unwrap() {
            classjobs.push(32);
        }
        if *row.AST().into_bool().unwrap() {
            classjobs.push(33);
        }
        if *row.SAM().into_bool().unwrap() {
            classjobs.push(34);
        }
        if *row.RDM().into_bool().unwrap() {
            classjobs.push(35);
        }
        if *row.BLU().into_bool().unwrap() {
            classjobs.push(36);
        }
        if *row.GNB().into_bool().unwrap() {
            classjobs.push(37);
        }
        if *row.DNC().into_bool().unwrap() {
            classjobs.push(38);
        }
        if *row.RPR().into_bool().unwrap() {
            classjobs.push(39);
        }
        if *row.SGE().into_bool().unwrap() {
            classjobs.push(40);
        }
        if *row.VPR().into_bool().unwrap() {
            classjobs.push(41);
        }
        if *row.PCT().into_bool().unwrap() {
            classjobs.push(42);
        }

        classjobs
    }
}

impl mlua::UserData for GameData {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("is_aetheryte", |_, this, aetheryte_id: u32| {
            Ok(this.is_aetheryte(aetheryte_id))
        });
        methods.add_method_mut("get_warp_logic_name", |_, this, warp_id: u32| {
            Ok(this.get_warp_logic_name(warp_id))
        });
        methods.add_method_mut("get_custom_talk_name", |_, this, custom_talk_id: u32| {
            Ok(this.get_custom_talk_name(custom_talk_id))
        });
        methods.add_method_mut("get_opening_name", |_, this, opening_id: u32| {
            Ok(this.get_opening_name(opening_id))
        });
        methods.add_method_mut("get_pre_handler_target", |_, this, pre_handler_id: u32| {
            Ok(this.get_pre_handler_target(pre_handler_id))
        });
        methods.add_method_mut("get_halloween_npc_transform", |_, this, npc_id: u32| {
            Ok(this.get_halloween_npc_transform(npc_id))
        });
        methods.add_method_mut("get_quest_name", |_, this, quest_id: u32| {
            Ok(this.get_quest_name(quest_id))
        });
        methods.add_method_mut(
            "get_topic_select_target",
            |_, this, (topic_select_id, selected_topic): (u32, usize)| {
                Ok(this.get_topic_select_target(topic_select_id, selected_topic))
            },
        );
        methods.add_method_mut(
            "get_content_short_name",
            |_, this, content_finder_row_id: u16| {
                Ok(this.get_content_short_name(content_finder_row_id))
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
