use physis::{
    common::Language,
    gamedata::GameData,
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, LayerGroup, PopRangeInstanceObject,
    },
};

/// Represents a loaded zone
#[derive(Default)]
pub struct Zone {
    pub id: u16,
    planevent: Option<LayerGroup>,
    vfx: Option<LayerGroup>,
    planmap: Option<LayerGroup>,
    planner: Option<LayerGroup>,
    bg: Option<LayerGroup>,
    sound: Option<LayerGroup>,
    planlive: Option<LayerGroup>,
}

impl Zone {
    pub fn load(game_data: &mut GameData, id: u16) -> Self {
        let mut zone = Self {
            id,
            ..Default::default()
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

        let mut load_lgb = |name: &str| -> Option<LayerGroup> {
            let path = format!("bg/{}/level/{}.lgb", &bg_path[..level_index], name);
            tracing::info!("Loading {path}");
            let lgb = game_data.extract(&path)?;
            LayerGroup::from_existing(&lgb)
        };

        zone.planevent = load_lgb("planevent");
        zone.vfx = load_lgb("vfx");
        zone.planmap = load_lgb("planmap");
        zone.planner = load_lgb("planner");
        zone.bg = load_lgb("bg");
        zone.sound = load_lgb("sound");
        zone.planlive = load_lgb("planlive");

        zone
    }

    /// Search for an exit box matching an id.
    pub fn find_exit_box(
        &self,
        instance_id: u32,
    ) -> Option<(&InstanceObject, &ExitRangeInstanceObject)> {
        // TODO: also check position!
        for group in &self.planmap.as_ref().unwrap().chunks[0].layers {
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
        for group in &self.planmap.as_ref().unwrap().chunks[0].layers {
            for object in &group.objects {
                if let LayerEntryData::PopRange(pop_range) = &object.data {
                    if object.instance_id == instance_id {
                        return Some((object, pop_range));
                    }
                }
            }
        }

        // For certain PopRanges (e.g. the starting position in the opening zones)
        for group in &self.planevent.as_ref().unwrap().chunks[0].layers {
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
