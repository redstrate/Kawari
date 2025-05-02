use serde::{Deserialize, Serialize};

/// Represents an item, or if the quanity is zero an empty slot.
#[derive(Default, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Item {
    pub quantity: u32,
    pub id: u32,
}

impl Item {
    pub fn new(quantity: u32, id: u32) -> Self {
        Self { quantity, id }
    }
}
