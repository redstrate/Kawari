use serde::{Deserialize, Serialize};

use super::{Item, Storage};

use crate::{GameData, ItemInfoQuery};

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
    pub fn calculate_item_level(&self, game_data: &mut GameData) -> u16 {
        const DIVISOR: u16 = 12;
        const INDEX_MAIN: u32 = 0;
        const INDEX_BODY: u32 = 3;
        const INDEX_BELT: u32 = 5;
        const INDEX_SOUL_CRYSTAL: u32 = 13;
        const RESTRICTED: i8 = -1; // On the EquipSlotCategory sheet, -1 means that item slot can't be equipped while another item restricts it.

        let mut level = 0;

        // First, sum up the item levels of all item slots regardless of restrictions.
        for index in 0..self.max_slots() {
            if index == INDEX_BELT || index == INDEX_SOUL_CRYSTAL {
                continue;
            }

            let item = self.get_slot(index as u16);
            level += item.item_level;
        }

        // Next, calculate additional item levels based off main hand and body equipment restrictions.
        let main_hand_info = game_data
            .get_item_info(ItemInfoQuery::ById(self.main_hand.id))
            .unwrap();

        // If our main hand weapon is two-handed (i.e. restricts off-hands from being equipped), it counts one additional time.
        if main_hand_info.equip_restrictions.off_hand == RESTRICTED {
            level += self.get_slot(INDEX_MAIN as u16).item_level;
        }

        let body_info = game_data
            .get_item_info(ItemInfoQuery::ById(self.body.id))
            .unwrap();

        let body_restrictions = [
            body_info.equip_restrictions.head,
            body_info.equip_restrictions.hands,
            body_info.equip_restrictions.legs,
            body_info.equip_restrictions.feet,
        ];

        // If our body equipment blocks head, hands, legs, or feet, it counts one addtional time per restricted slot.
        for slot in body_restrictions {
            if slot == RESTRICTED {
                level += self.get_slot(INDEX_BODY as u16).item_level;
            }
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

    // TODO: Fix this test so it can run again, calculate_item_level needs GameData now to function correctly.
    /*#[test]
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
    }*/
}
