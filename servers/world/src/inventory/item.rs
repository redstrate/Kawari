use kawari::{common::ITEM_CONDITION_MAX, ipc::zone::ItemInfo};

use serde::{Deserialize, Serialize};

use crate::ItemRow;

/// Represents an item, or if the quantity is zero, an empty slot.
#[derive(Default, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Item {
    /// How many of this item occupies it's slot.
    pub quantity: u32,
    /// Index into the Item Excel sheet.
    pub item_id: u32,
    /// The player who crafted this item.
    pub crafter_content_id: u64,
    /// Unknown flags.
    pub item_flags: u8,
    /// The condition of this item from 0 to 30000.
    pub condition: u16,
    /// Spiritbond or collectability stat.
    pub spiritbond_or_collectability: u16,
    /// If not zero, what Item this is glamoured to.
    pub glamour_id: u32,
    /// The materia melded into this item.
    pub materia: [u16; 5],
    /// The grade of said materia.
    pub materia_grades: [u8; 5],
    /// Dye information?
    pub stains: [u8; 2],

    // Data only used by us, and not stored.
    #[serde(skip)]
    pub item_level: u16,
    #[serde(skip)]
    pub stack_size: u32,
    #[serde(skip)]
    pub price_low: u32,
    #[serde(skip)]
    pub base_param_ids: [u8; 6],
    #[serde(skip)]
    pub base_param_values: [i16; 6],
}

impl Item {
    pub fn new(item_info: &ItemRow, quantity: u32) -> Self {
        Self {
            quantity,
            item_id: item_info.id,
            condition: ITEM_CONDITION_MAX,
            item_level: item_info.item_level,
            stack_size: item_info.stack_size,
            price_low: item_info.price_low,
            base_param_ids: item_info.base_param_ids,
            base_param_values: item_info.base_param_values,
            ..Default::default()
        }
    }

    /// Returns the catalog ID of the glamour, if applicable.
    pub fn apparent_id(&self) -> u32 {
        if self.quantity == 0 {
            return 0;
        }
        if self.glamour_id > 0 {
            return self.glamour_id;
        }
        self.item_id
    }

    pub fn is_empty_slot(&self) -> bool {
        self.quantity == 0
    }
}

impl From<Item> for ItemInfo {
    fn from(val: Item) -> Self {
        ItemInfo {
            quantity: val.quantity,
            item_id: val.item_id,
            crafter_content_id: val.crafter_content_id,
            item_flags: val.item_flags,
            condition: val.condition,
            spiritbond_or_collectability: val.spiritbond_or_collectability,
            glamour_id: val.glamour_id,
            materia: val.materia,
            materia_grades: val.materia_grades,
            stains: val.stains,
            ..Default::default()
        }
    }
}

impl From<ItemRow> for Item {
    fn from(value: ItemRow) -> Self {
        Self::new(&value, 0)
    }
}
