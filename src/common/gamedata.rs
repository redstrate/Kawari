use icarus::Aetheryte::AetheryteSheet;
use icarus::ClassJob::ClassJobSheet;
use icarus::EquipSlotCategory::EquipSlotCategorySheet;
use icarus::World::WorldSheet;
use icarus::{Tribe::TribeSheet, Warp::WarpSheet};
use physis::common::{Language, Platform};
use physis::exd::{EXD, ExcelRowKind};
use physis::exh::EXH;

use crate::{common::Attributes, config::get_config};

/// Convenient methods built on top of Physis to access data relevant to the server
pub struct GameData {
    pub game_data: physis::gamedata::GameData,
    pub item_exh: EXH,
    pub item_pages: Vec<EXD>,
}

impl Default for GameData {
    fn default() -> Self {
        Self::new()
    }
}

impl GameData {
    pub fn new() -> Self {
        let config = get_config();

        let mut game_data =
            physis::gamedata::GameData::from_existing(Platform::Win32, &config.game_location);

        let mut item_pages = Vec::new();

        let item_exh = game_data.read_excel_sheet_header("Item").unwrap();
        for (i, _) in item_exh.pages.iter().enumerate() {
            item_pages.push(
                game_data
                    .read_excel_sheet("Item", &item_exh, Language::English, i)
                    .unwrap(),
            );
        }

        Self {
            game_data,
            item_exh,
            item_pages,
        }
    }

    /// Gets the world name from an id into the World Excel sheet.
    pub fn get_world_name(&mut self, world_id: u16) -> Option<String> {
        let sheet = WorldSheet::read_from(&mut self.game_data, Language::None)?;
        let row = sheet.get_row(world_id as u32)?;

        row.Name().into_string().map(|x| x.clone())
    }

    /// Gets the starting city-state from a given class/job id.
    pub fn get_citystate(&mut self, classjob_id: u16) -> Option<u8> {
        let sheet = ClassJobSheet::read_from(&mut self.game_data, Language::English)?;
        let row = sheet.get_row(classjob_id as u32)?;

        row.StartingTown().into_u8().map(|x| *x)
    }

    pub fn get_racial_base_attributes(&mut self, tribe_id: u8) -> Option<Attributes> {
        // The Tribe Excel sheet only has deltas (e.g. 2 or -2) which are applied to a base 20 number... from somewhere
        let base_stat = 20;

        let sheet = TribeSheet::read_from(&mut self.game_data, Language::English)?;
        let row = sheet.get_row(tribe_id as u32)?;

        Some(Attributes {
            strength: (base_stat + row.STR().into_i8()?) as u32,
            dexterity: (base_stat + row.DEX().into_i8()?) as u32,
            vitality: (base_stat + row.VIT().into_i8()?) as u32,
            intelligence: (base_stat + row.INT().into_i8()?) as u32,
            mind: (base_stat + row.MND().into_i8()?) as u32,
        })
    }

    /// Gets the primary model ID for a given item ID
    pub fn get_primary_model_id(&mut self, item_id: u32) -> Option<u64> {
        for page in &self.item_pages {
            if let Some(row) = page.get_row(item_id) {
                let ExcelRowKind::SingleRow(item_row) = row else {
                    panic!("Expected a single row!")
                };

                let physis::exd::ColumnData::UInt64(id) = &item_row.columns[47] else {
                    panic!("Unexpected type!");
                };

                return Some(*id);
            }
        }

        None
    }

    /// Returns the pop range object id that's associated with the warp id
    pub fn get_warp(&mut self, warp_id: u32) -> Option<(u32, u16)> {
        let sheet = WarpSheet::read_from(&mut self.game_data, Language::English)?;
        let row = sheet.get_row(warp_id)?;

        let pop_range_id = row.PopRange().into_u32()?;
        let zone_id = row.TerritoryType().into_u16()?;

        Some((*pop_range_id, *zone_id))
    }

    pub fn get_aetheryte(&mut self, aetheryte_id: u32) -> Option<(u32, u16)> {
        let sheet = AetheryteSheet::read_from(&mut self.game_data, Language::English)?;
        let row = sheet.get_row(aetheryte_id)?;

        // TODO: just look in the level sheet?
        let pop_range_id = row.Level()[0].into_u32()?;
        let zone_id = row.Territory().into_u16()?;

        Some((*pop_range_id, *zone_id))
    }

    /// Find an item's equip category and id by name, if it exists.
    pub fn get_item_by_name(&mut self, name: &str) -> Option<(u8, u32)> {
        for page in &self.item_pages {
            for row in &page.rows {
                let ExcelRowKind::SingleRow(single_row) = &row.kind else {
                    panic!("Expected a single row!")
                };

                let physis::exd::ColumnData::String(item_name) = &single_row.columns[9] else {
                    panic!("Unexpected type!");
                };

                if !item_name.to_lowercase().contains(&name.to_lowercase()) {
                    continue;
                }

                let physis::exd::ColumnData::UInt8(equip_category) = &single_row.columns[17] else {
                    panic!("Unexpected type!");
                };

                return Some((*equip_category, row.row_id));
            }
        }

        None
    }

    /// Turn an equip slot category id into a slot for the equipped inventory
    pub fn get_equipslot_category(&mut self, equipslot_id: u8) -> Option<u16> {
        let sheet = EquipSlotCategorySheet::read_from(&mut self.game_data, Language::None)?;
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

        let soul_crystal = row.FingerL().into_i8()?;
        if *soul_crystal == 1 {
            return Some(13);
        }

        None
    }
}
