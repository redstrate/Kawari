use serde::{Deserialize, Serialize};

use super::{Item, Storage};

use crate::{GameData, ItemInfoQuery};

use strum::{EnumCount, IntoEnumIterator};
use strum_macros::{EnumCount as EnumCountMacro, EnumIter, FromRepr};

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

#[repr(u16)]
#[derive(Clone, Copy, Debug, EnumCountMacro, EnumIter, FromRepr, PartialEq)]
pub enum EquipSlot {
    MainHand = 0,
    OffHand = 1,
    Head = 2,
    Body = 3,
    Hands = 4,
    Belt = 5,
    Legs = 6,
    Feet = 7,
    Ears = 8,
    Neck = 9,
    Wrists = 10,
    RightRing = 11,
    LeftRing = 12,
    SoulCrystal = 13,
}

// On the EquipSlotCategory sheet, -1 means that item slot can't be equipped while another item restricts it.
pub const EQUIP_RESTRICTED: i8 = -1;

impl EquippedStorage {
    /// Calculates the player's item level.
    pub fn calculate_item_level(&self, game_data: &mut GameData) -> u16 {
        const DIVISOR: u16 = 12;

        let mut level = 0;

        // First, sum up the item levels of all item slots regardless of restrictions.
        for index in EquipSlot::iter() {
            if index == EquipSlot::Belt || index == EquipSlot::SoulCrystal {
                continue;
            }

            let item = self.get_slot(index as u16);
            level += item.item_level;
        }

        // Next, calculate additional item levels based off main hand and body equipment restrictions.
        if let Some(main_hand_info) =
            game_data.get_item_info(ItemInfoQuery::ById(self.main_hand.id))
        {
            // If our main hand weapon is two-handed (i.e. restricts off-hands from being equipped), it counts one additional time.
            if main_hand_info.equip_restrictions.off_hand == EQUIP_RESTRICTED {
                level += self.get_slot(EquipSlot::MainHand as u16).item_level;
            }
        }

        if let Some(body_info) = game_data.get_item_info(ItemInfoQuery::ById(self.body.id)) {
            let body_restrictions = [
                body_info.equip_restrictions.head,
                body_info.equip_restrictions.hands,
                body_info.equip_restrictions.legs,
                body_info.equip_restrictions.feet,
            ];

            // If our body equipment blocks head, hands, legs, or feet, it counts one addtional time per restricted slot.
            for slot in body_restrictions {
                if slot == EQUIP_RESTRICTED {
                    level += self.get_slot(EquipSlot::Body as u16).item_level;
                }
            }
        }

        std::cmp::min(level / DIVISOR, 9999)
    }
}

impl Storage for EquippedStorage {
    fn max_slots(&self) -> u32 {
        EquipSlot::COUNT as u32
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        let Some(index) = EquipSlot::from_repr(index) else {
            panic!("{index} is not a valid src_container_index?!?")
        };

        match index {
            EquipSlot::MainHand => &mut self.main_hand,
            EquipSlot::OffHand => &mut self.off_hand,
            EquipSlot::Head => &mut self.head,
            EquipSlot::Body => &mut self.body,
            EquipSlot::Hands => &mut self.hands,
            EquipSlot::Belt => &mut self.belt,
            EquipSlot::Legs => &mut self.legs,
            EquipSlot::Feet => &mut self.feet,
            EquipSlot::Ears => &mut self.ears,
            EquipSlot::Neck => &mut self.neck,
            EquipSlot::Wrists => &mut self.wrists,
            EquipSlot::RightRing => &mut self.right_ring,
            EquipSlot::LeftRing => &mut self.left_ring,
            EquipSlot::SoulCrystal => &mut self.soul_crystal,
        }
    }

    fn get_slot(&self, index: u16) -> &Item {
        let Some(index) = EquipSlot::from_repr(index) else {
            panic!("{index} is not a valid src_container_index?!?")
        };

        match index {
            EquipSlot::MainHand => &self.main_hand,
            EquipSlot::OffHand => &self.off_hand,
            EquipSlot::Head => &self.head,
            EquipSlot::Body => &self.body,
            EquipSlot::Hands => &self.hands,
            EquipSlot::Belt => &self.belt,
            EquipSlot::Legs => &self.legs,
            EquipSlot::Feet => &self.feet,
            EquipSlot::Ears => &self.ears,
            EquipSlot::Neck => &self.neck,
            EquipSlot::Wrists => &self.wrists,
            EquipSlot::RightRing => &self.right_ring,
            EquipSlot::LeftRing => &self.left_ring,
            EquipSlot::SoulCrystal => &self.soul_crystal,
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO: Fix this test so it can run again, calculate_item_level needs GameData now to function correctly.
    /*use super::*;

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
    }*/
}
