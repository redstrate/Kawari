use serde::{Deserialize, Serialize};

use super::{Item, Storage};

#[derive(Clone, Copy, Deserialize, Serialize, Debug)]
pub struct CurrencyStorage {
    pub gil: Item,
}

impl Default for CurrencyStorage {
    fn default() -> Self {
        Self {
            gil: Item { quantity: 0, id: 1 },
        }
    }
}

impl Storage for CurrencyStorage {
    fn max_slots(&self) -> u32 {
        1
    }

    fn num_items(&self) -> u32 {
        self.gil.quantity
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        match index {
            0 => &mut self.gil,
            _ => panic!("{} is not a valid src_container_index?!?", index),
        }
    }

    fn get_slot(&self, index: u16) -> &Item {
        match index {
            0 => &self.gil,
            _ => panic!("{} is not a valid src_container_index?!?", index),
        }
    }
}
