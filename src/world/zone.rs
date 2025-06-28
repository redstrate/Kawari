use icarus::TerritoryType::TerritoryTypeSheet;
use physis::{
    common::Language,
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, LayerGroup, PopRangeInstanceObject,
    },
};

use crate::common::{GameData, TerritoryNameKind};

/// Represents a loaded zone
#[derive(Default, Debug)]
pub struct Zone {
    pub id: u16,
    pub internal_name: String,
    pub region_name: String,
    pub place_name: String,
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

        let sheet =
            TerritoryTypeSheet::read_from(&mut game_data.game_data, Language::None).unwrap();
        let Some(row) = sheet.get_row(id as u32) else {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return zone;
        };

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let bg_path = row.Bg().into_string().unwrap();

        let Some(level_index) = bg_path.find("/level/") else {
            return zone;
        };

        let mut load_lgb = |name: &str| -> Option<LayerGroup> {
            let path = format!("bg/{}/level/{}.lgb", &bg_path[..level_index], name);
            let lgb_file = game_data.game_data.extract(&path)?;
            tracing::info!("Loading {path}");
            let lgb = LayerGroup::from_existing(&lgb_file);
            if lgb.is_none() {
                tracing::warn!(
                    "Failed to parse {path}, this is most likely a bug in Physis and should be reported somewhere!"
                )
            }
            lgb
        };

        zone.planevent = load_lgb("planevent");
        zone.vfx = load_lgb("vfx");
        zone.planmap = load_lgb("planmap");
        zone.planner = load_lgb("planner");
        zone.bg = load_lgb("bg");
        zone.sound = load_lgb("sound");
        zone.planlive = load_lgb("planlive");

        // load names
        let fallback = "<Unable to load name!>";
        zone.internal_name = game_data
            .get_territory_name(id as u32, TerritoryNameKind::Internal)
            .unwrap_or(fallback.to_string());
        zone.region_name = game_data
            .get_territory_name(id as u32, TerritoryNameKind::Region)
            .unwrap_or(fallback.to_string());
        zone.place_name = game_data
            .get_territory_name(id as u32, TerritoryNameKind::Place)
            .unwrap_or(fallback.to_string());

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
        if let Some(planmap) = self.planmap.as_ref() {
            for group in &planmap.chunks[0].layers {
                for object in &group.objects {
                    if let LayerEntryData::PopRange(pop_range) = &object.data {
                        if object.instance_id == instance_id {
                            return Some((object, pop_range));
                        }
                    }
                }
            }
        }

        if let Some(planevent) = self.planevent.as_ref() {
            // For certain PopRanges (e.g. the starting position in the opening zones)
            for group in &planevent.chunks[0].layers {
                for object in &group.objects {
                    if let LayerEntryData::PopRange(pop_range) = &object.data {
                        if object.instance_id == instance_id {
                            return Some((object, pop_range));
                        }
                    }
                }
            }
        }

        None
    }
}
