use physis::{
    common::{Language, Platform},
    gamedata::GameData,
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, LayerGroup, PopRangeInstanceObject,
    },
};

use crate::config::get_config;

/// Represents a loaded zone
pub struct Zone {
    pub id: u16,
    layer_group: LayerGroup,
}

impl Zone {
    pub fn load(id: u16) -> Self {
        let config = get_config();

        let mut game_data =
            GameData::from_existing(Platform::Win32, &config.game_location).unwrap();

        let exh = game_data.read_excel_sheet_header("TerritoryType").unwrap();
        let exd = game_data.read_excel_sheet("TerritoryType", &exh, Language::None, 0).unwrap();

        let territory_type_row = &exd.read_row(&exh, id as u32).unwrap()[0];

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let physis::exd::ColumnData::String(bg_path) = &territory_type_row.data[1] else {
            panic!("Unexpected type!");
        };

        let path = format!("bg/{}/level/planmap.lgb", &bg_path[..bg_path.find("/level/").unwrap()]);
        let lgb = game_data.extract(&path).unwrap();
        let layer_group = LayerGroup::from_existing(&lgb).unwrap();
        Self { id, layer_group }
    }

    /// Search for an exit box matching an id.
    pub fn find_exit_box(
        &self,
        instance_id: u32,
    ) -> Option<(&InstanceObject, &ExitRangeInstanceObject)> {
        // TODO: also check position!
        for group in &self.layer_group.layers {
            for object in &group.objects {
                if let LayerEntryData::ExitRange(exit_range) = &object.data {
                    if object.instance_id == instance_id {
                        return Some((object, exit_range));
                    }
                }
            }
        }

        None
    }

    pub fn find_pop_range(
        &self,
        instance_id: u32,
    ) -> Option<(&InstanceObject, &PopRangeInstanceObject)> {
        // TODO: also check position!
        for group in &self.layer_group.layers {
            for object in &group.objects {
                if let LayerEntryData::PopRange(pop_range) = &object.data {
                    if object.instance_id == instance_id {
                        return Some((object, pop_range));
                    }
                }
            }
        }

        None
    }
}
