use physis::equipment::EquipSlot;
use serde::{Deserialize, Serialize};
use strum::{EnumCount, IntoEnumIterator};

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

// On the EquipSlotCategory sheet, -1 means that item slot can't be equipped while another item restricts it.
pub const EQUIP_RESTRICTED: i8 = -1;

impl EquippedStorage {
    /// Calculates the player's average item level.
    ///
    /// Retail averages over 12 gear slots: MainHand, OffHand, Head, Body, Hands, Legs, Feet, Ears,
    /// Neck, Wrists, and both Rings. The Waist slot is a legacy slot that no longer exists, and the
    /// Soul Crystal is never counted. For a two-handed weapon the (empty) off-hand slot inherits the
    /// main hand's item level, so a full set of N-ilvl gear reports N.
    ///
    /// The item level is resolved live from the Item sheet via `item_id` rather than trusting the
    /// cached `item_level` field. That field is `#[serde(skip)]` and only populated by
    /// `prepare_player_inventory` at load time, so items moved around mid-session (e.g. via a
    /// gearset swap) can carry a stale `0`. Resolving from `item_id` keeps the result correct
    /// regardless of how the item got into the slot.
    pub fn calculate_item_level(&self, game_data: &mut GameData) -> u16 {
        const DIVISOR: u16 = 12;

        // Resolve an equipped item's true item level from its catalog id (0 for empty slots).
        let resolve_ilvl = |game_data: &mut GameData, item: &Item| -> u32 {
            if item.quantity == 0 || item.item_id == 0 {
                return 0;
            }
            game_data
                .get_item_info(ItemInfoQuery::ById(item.item_id))
                .map(|info| info.item_level as u32)
                .unwrap_or(item.item_level as u32)
        };

        let main_hand_ilvl = resolve_ilvl(game_data, &self.main_hand);

        // Is the main hand two-handed? If so, the off-hand slot counts as the main hand.
        let main_hand_is_two_handed = game_data
            .get_item_info(ItemInfoQuery::ById(self.main_hand.item_id))
            .map(|info| info.equip_restrictions.off_hand == EQUIP_RESTRICTED)
            .unwrap_or(false);

        let mut level: u32 = 0;

        for index in EquipSlot::iter() {
            // The waist slot is legacy/removed and the soul crystal is never counted.
            if index == EquipSlot::Waist || index == EquipSlot::SoulCrystal {
                continue;
            }

            // For a two-handed weapon the empty off-hand slot inherits the main hand's item level.
            if index == EquipSlot::OffHand && main_hand_is_two_handed && self.off_hand.quantity == 0
            {
                level += main_hand_ilvl;
                continue;
            }

            let item = *self.get_slot(index as u16);
            level += resolve_ilvl(game_data, &item);
        }

        std::cmp::min((level / DIVISOR as u32) as u16, 9999)
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
            EquipSlot::Waist => &mut self.belt,
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
            EquipSlot::Waist => &self.belt,
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
