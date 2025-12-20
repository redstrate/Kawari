use kawari::common::ITEM_CONDITION_MAX;

use serde::{Deserialize, Serialize};

use crate::ItemInfo;

/// Represents an item, or if the quantity is zero, an empty slot.
#[derive(Default, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Item {
    pub quantity: u32,
    pub id: u32,
    pub condition: u16,
    pub glamour_catalog_id: u32,
    #[serde(skip)]
    pub item_level: u16,
    #[serde(skip)]
    pub stack_size: u32,
    #[serde(skip)]
    pub price_low: u32,
}

impl Item {
    pub fn new(item_info: ItemInfo, quantity: u32) -> Self {
        Self {
            quantity,
            id: item_info.id,
            condition: ITEM_CONDITION_MAX,
            item_level: item_info.item_level,
            stack_size: item_info.stack_size,
            price_low: item_info.price_low,
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

    pub fn is_empty_slot(&self) -> bool {
        self.quantity == 0
    }
}
