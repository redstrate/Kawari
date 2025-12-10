use serde::{Deserialize, Serialize};

use super::{Item, Storage};

#[derive(Default, Clone, Copy, Deserialize, Serialize, Debug)]
pub struct EquippedStorage {
    pub main_hand: Item,
    pub off_hand: Item,
    pub head: Item,
    pub body: Item,
    pub hands: Item,
    pub belt: Item,
    pub legs: Item,
    pub feet: Item,
    pub ears: Item,
    pub neck: Item,
    pub wrists: Item,
    pub right_ring: Item,
    pub left_ring: Item,
    pub soul_crystal: Item,
}

impl EquippedStorage {
    /// Calculates the player's item level.
    /// TODO: This is not accurate, for several reasons.
    /// First, it does not take into account if the main hand is a one or two hander.
    /// Second, it does not take into account if body armour occupies multiple slots or not (e.g. Herklaedi: cannot equip anything to hands, legs, or feet).
    /// There is currently no known way of properly figuring those out. Presumably, the information is somewhere in the Items sheet.
    pub fn calculate_item_level(&self) -> u16 {
        const DIVISOR: u16 = 13;
        const INDEX_BELT: u32 = 5;
        const INDEX_SOUL_CRYSTAL: u32 = 13;

        let mut level = self.main_hand.item_level;

        if !self.off_hand.is_empty_slot() {
            level += self.off_hand.item_level;
        } else {
            // Main hand counts twice if off hand is empty. See comments above why this isn't always correct.
            level += self.main_hand.item_level;
        }

        for index in 2..self.max_slots() {
            if index == INDEX_BELT || index == INDEX_SOUL_CRYSTAL {
                continue;
            }

            let item = self.get_slot(index as u16);
            level += item.item_level;
        }

        std::cmp::min(level / DIVISOR, 9999)
    }
}

impl Storage for EquippedStorage {
    fn max_slots(&self) -> u32 {
        14
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        match index {
            0 => &mut self.main_hand,
            1 => &mut self.off_hand,
            2 => &mut self.head,
            3 => &mut self.body,
            4 => &mut self.hands,
            5 => &mut self.belt,
            6 => &mut self.legs,
            7 => &mut self.feet,
            8 => &mut self.ears,
            9 => &mut self.neck,
            10 => &mut self.wrists,
            11 => &mut self.right_ring,
            12 => &mut self.left_ring,
            13 => &mut self.soul_crystal,
            _ => panic!("{index} is not a valid src_container_index?!?"),
        }
    }

    fn get_slot(&self, index: u16) -> &Item {
        match index {
            0 => &self.main_hand,
            1 => &self.off_hand,
            2 => &self.head,
            3 => &self.body,
            4 => &self.hands,
            5 => &self.belt,
            6 => &self.legs,
            7 => &self.feet,
            8 => &self.ears,
            9 => &self.neck,
            10 => &self.wrists,
            11 => &self.right_ring,
            12 => &self.left_ring,
            13 => &self.soul_crystal,
            _ => panic!("{index} is not a valid src_container_index?!?"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_level() {
        let equipped = EquippedStorage::default();
        assert_eq!(equipped.calculate_item_level(), 0);

        let base_item = Item {
            quantity: 1,
            item_level: 5,
            ..Default::default()
        };
        let equipped = EquippedStorage {
            main_hand: base_item,
            body: base_item,
            hands: base_item,
            legs: base_item,
            feet: base_item,
            ears: base_item,
            neck: base_item,
            wrists: base_item,
            right_ring: base_item,
            left_ring: base_item,
            ..Default::default()
        };
        assert_eq!(equipped.calculate_item_level(), 4);
    }
}
