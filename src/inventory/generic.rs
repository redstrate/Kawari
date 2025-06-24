use serde::{Deserialize, Serialize};

use super::{Item, Storage};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenericStorage<const N: usize> {
    pub slots: Vec<Item>,
}

impl<const N: usize> GenericStorage<N> {
    pub fn default() -> Self {
        Self {
            slots: vec![Item::default(); N],
        }
    }
}

impl<const N: usize> Storage for GenericStorage<N> {
    fn max_slots(&self) -> u32 {
        N as u32
    }

    fn num_items(&self) -> u32 {
        self.slots.iter().filter(|item| item.quantity > 0).count() as u32
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        self.slots.get_mut(index as usize).unwrap()
    }

    fn get_slot(&self, index: u16) -> &Item {
        self.slots.get(index as usize).unwrap()
    }
}
