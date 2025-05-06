use physis::common::{Language, Platform};
use physis::exd::EXD;
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
            physis::gamedata::GameData::from_existing(Platform::Win32, &config.game_location)
                .unwrap();

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
    pub fn get_world_name(&mut self, world_id: u16) -> String {
        let exh = self.game_data.read_excel_sheet_header("World").unwrap();
        let exd = self
            .game_data
            .read_excel_sheet("World", &exh, Language::None, 0)
            .unwrap();

        let world_row = &exd.read_row(&exh, world_id as u32).unwrap()[0];

        let physis::exd::ColumnData::String(name) = &world_row.data[1] else {
            panic!("Unexpected type!");
        };

        name.clone()
    }

    /// Gets the starting city-state from a given class/job id.
    pub fn get_citystate(&mut self, classjob_id: u16) -> u8 {
        let exh = self.game_data.read_excel_sheet_header("ClassJob").unwrap();
        let exd = self
            .game_data
            .read_excel_sheet("ClassJob", &exh, Language::English, 0)
            .unwrap();

        let world_row = &exd.read_row(&exh, classjob_id as u32).unwrap()[0];

        let physis::exd::ColumnData::UInt8(town_id) = &world_row.data[33] else {
            panic!("Unexpected type!");
        };

        *town_id
    }

    pub fn get_racial_base_attributes(&mut self, tribe_id: u8) -> Attributes {
        // The Tribe Excel sheet only has deltas (e.g. 2 or -2) which are applied to a base 20 number... from somewhere
        let base_stat = 20;

        let exh = self.game_data.read_excel_sheet_header("Tribe").unwrap();
        let exd = self
            .game_data
            .read_excel_sheet("Tribe", &exh, Language::English, 0)
            .unwrap();

        let tribe_row = &exd.read_row(&exh, tribe_id as u32).unwrap()[0];

        let get_column = |column_index: usize| {
            let physis::exd::ColumnData::Int8(delta) = &tribe_row.data[column_index] else {
                panic!("Unexpected type!");
            };

            *delta
        };

        Attributes {
            strength: (base_stat + get_column(4)) as u32,
            dexterity: (base_stat + get_column(6)) as u32,
            vitality: (base_stat + get_column(5)) as u32,
            intelligence: (base_stat + get_column(7)) as u32,
            mind: (base_stat + get_column(8)) as u32,
        }
    }

    /// Gets the primary model ID for a given item ID
    pub fn get_primary_model_id(&mut self, item_id: u32) -> Option<u64> {
        for page in &self.item_pages {
            if let Some(row) = page.read_row(&self.item_exh, item_id) {
                let item_row = &row[0];

                let physis::exd::ColumnData::UInt64(id) = &item_row.data[47] else {
                    panic!("Unexpected type!");
                };

                return Some(*id);
            }
        }

        None
    }

    /// Returns the pop range object id that's associated with the warp id
    pub fn get_warp(&mut self, warp_id: u32) -> (u32, u16) {
        let exh = self.game_data.read_excel_sheet_header("Warp").unwrap();
        let exd = self
            .game_data
            .read_excel_sheet("Warp", &exh, Language::English, 0)
            .unwrap();

        let row = &exd.read_row(&exh, warp_id).unwrap()[0];

        let physis::exd::ColumnData::UInt32(pop_range_id) = &row.data[0] else {
            panic!("Unexpected type!");
        };

        let physis::exd::ColumnData::UInt16(zone_id) = &row.data[1] else {
            panic!("Unexpected type!");
        };

        (*pop_range_id, *zone_id)
    }
}
