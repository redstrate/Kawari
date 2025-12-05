use super::Item;

/// Represents a generic item storage.
pub trait Storage: Sync {
    fn max_slots(&self) -> u32;
    fn num_items(&self) -> u32;
    fn get_slot_mut(&mut self, index: u16) -> &mut Item;
    fn get_slot(&self, index: u16) -> &Item;
}

/// Finds the first free slot in this container.
pub fn get_next_free_slot(storage: &dyn Storage) -> Option<u16> {
    (0..storage.max_slots() as u16).find(|&i| storage.get_slot(i).is_empty_slot())
}
