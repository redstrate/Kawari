use std::path::PathBuf;

use icarus::TerritoryType::TerritoryTypeSheet;
use physis::{
    common::Language,
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, LayerGroup, PopRangeInstanceObject,
    },
    lvb::Lvb,
    resource::Resource,
};

use crate::{
    common::{GameData, TerritoryNameKind},
    config::get_config,
};

/// Represents a loaded zone
#[derive(Default, Debug)]
pub struct Zone {
    pub id: u16,
    pub internal_name: String,
    pub region_name: String,
    pub place_name: String,
    pub intended_use: u8,
    pub layer_groups: Vec<LayerGroup>,
    pub navimesh_path: String,
}

impl Zone {
    pub fn load(game_data: &mut GameData, id: u16) -> Self {
        let mut zone = Self {
            id,
            ..Default::default()
        };

        let sheet = TerritoryTypeSheet::read_from(&mut game_data.resource, Language::None).unwrap();
        let Some(row) = sheet.get_row(id as u32) else {
            tracing::warn!("Invalid zone id {id}, allowing anyway...");
            return zone;
        };

        zone.intended_use = *row.TerritoryIntendedUse().into_u8().unwrap();

        // e.g. ffxiv/fst_f1/fld/f1f3/level/f1f3
        let bg_path = row.Bg().into_string().unwrap();

        let path = format!("bg/{}.lvb", &bg_path);
        let lgb_file = game_data.resource.read(&path).unwrap();
        let lgb = Lvb::from_existing(&lgb_file).unwrap();

        for layer_set in &lgb.scns[0].unk3.unk2 {
            // FIXME: this is wrong. I think there might be multiple, separate navimeshes in really big zones but I'm not sure yet.
            zone.navimesh_path = layer_set.path_nvm.replace("/server/data/", "").to_string();
        }

        let config = get_config();
        if config.filesystem.navimesh_path.is_empty() {
            tracing::warn!("Navimesh path is not set! Monsters will not function correctly!");
        } else {
            let mut nvm_path = PathBuf::from(config.filesystem.navimesh_path);
            nvm_path.push(&zone.navimesh_path);

            Self::load_navimesh(&nvm_path.to_str().unwrap());
        }

        let mut load_lgb = |path: &str| -> Option<LayerGroup> {
            let lgb_file = game_data.resource.read(path)?;
            tracing::info!("Loading {path}");
            let lgb = LayerGroup::from_existing(&lgb_file);
            if lgb.is_none() {
                tracing::warn!(
                    "Failed to parse {path}, this is most likely a bug in Physis and should be reported somewhere!"
                )
            }
            lgb
        };

        for path in &lgb.scns[0].header.path_layer_group_resources {
            if let Some(lgb) = load_lgb(path) {
                zone.layer_groups.push(lgb);
            }
        }

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
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                for object in &layer.objects {
                    if let LayerEntryData::ExitRange(exit_range) = &object.data {
                        if object.instance_id == instance_id {
                            return Some((object, exit_range));
                        }
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
        for layer_group in &self.layer_groups {
            for layer in &layer_group.chunks[0].layers {
                for object in &layer.objects {
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

    // TODO: add better error handling here
    fn load_navimesh(path: &str) -> Option<()> {
        if !std::fs::exists(path).unwrap_or_default() {
            tracing::warn!("Navimesh {path} does not exist, monsters will not function correctly!");
            return None;
        }

        Some(())
    }
}
