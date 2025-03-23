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
    layer_group: Option<LayerGroup>,
}

impl Zone {
    pub fn load(id: u16) -> Self {
        let config = get_config();

        let mut zone = Self {
            id,
            layer_group: None,
        };

        let Some(mut game_data) = GameData::from_existing(Platform::Win32, &config.game_location)
        else {
            return zone;
        };

        let Some(exh) = game_data.read_excel_sheet_header("TerritoryType") else {
            return zone;
        };
        let Some(exd) = game_data.read_excel_sheet("TerritoryType", &exh, Language::None, 0) else {
            return zone;
        };

        let Some(territory_type_row) = &exd.read_row(&exh, id as u32) else {
            return zone;
        };
        let territory_type_row = &territory_type_row[0];

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let physis::exd::ColumnData::String(bg_path) = &territory_type_row.data[1] else {
            panic!("Unexpected type!");
        };

        let Some(level_index) = bg_path.find("/level/") else {
            return zone;
        };

        let path = format!("bg/{}/level/planmap.lgb", &bg_path[..level_index]);
        let Some(lgb) = game_data.extract(&path) else {
            return zone;
        };
        zone.layer_group = LayerGroup::from_existing(&lgb);

        zone
    }

    /// Search for an exit box matching an id.
    pub fn find_exit_box(
        &self,
        instance_id: u32,
    ) -> Option<(&InstanceObject, &ExitRangeInstanceObject)> {
        // TODO: also check position!
        for group in &self.layer_group.as_ref().unwrap().layers {
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
        for group in &self.layer_group.as_ref().unwrap().layers {
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
