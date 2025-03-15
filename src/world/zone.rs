use physis::{
    common::Platform,
    gamedata::GameData,
    layer::{
        ExitRangeInstanceObject, InstanceObject, LayerEntryData, LayerGroup, PopRangeInstanceObject,
    },
};

use crate::{config::get_config, world::Position};

/// Represents a loaded zone
pub struct Zone {
    id: u16,
    layer_group: LayerGroup,
}

impl Zone {
    pub fn load(id: u16) -> Self {
        let config = get_config();

        let mut game_data =
            GameData::from_existing(Platform::Win32, &config.game_location).unwrap();
        let mdl;
        println!("loading {id}");
        if id == 133 {
            mdl = game_data
                .extract("bg/ffxiv/fst_f1/twn/f1t2/level/planmap.lgb")
                .unwrap();
        } else {
            mdl = game_data
                .extract("bg/ffxiv/fst_f1/twn/f1t1/level/planmap.lgb")
                .unwrap();
        }

        let layer_group = LayerGroup::from_existing(&mdl).unwrap();
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
