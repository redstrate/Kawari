use crate::ITEM_CONDITION_MAX;
use serde::{Deserialize, Serialize};

/// Represents an item, or if the quanity is zero an empty slot.
#[derive(Default, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Item {
    pub quantity: u32,
    pub id: u32,
    pub condition: u16,
    pub glamour_catalog_id: u32,
}

impl Item {
    pub fn new(quantity: u32, id: u32) -> Self {
        Self {
            quantity,
            id,
            condition: ITEM_CONDITION_MAX,
            ..Default::default()
        }
    }

    /// Returns the catalog ID of the glamour, if applicable.
    pub fn apparent_id(&self) -> u32 {
        if self.glamour_catalog_id > 0 {
            return self.glamour_catalog_id;
        }
        self.id
    }
}
