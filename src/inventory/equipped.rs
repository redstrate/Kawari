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

impl Storage for EquippedStorage {
    fn max_slots(&self) -> u32 {
        14
    }

    fn num_items(&self) -> u32 {
        self.main_hand.quantity
            + self.off_hand.quantity
            + self.head.quantity
            + self.body.quantity
            + self.hands.quantity
            + self.legs.quantity
            + self.feet.quantity
            + self.ears.quantity
            + self.neck.quantity
            + self.wrists.quantity
            + self.right_ring.quantity
            + self.left_ring.quantity
            + self.soul_crystal.quantity
    }

    fn get_slot_mut(&mut self, index: u16) -> &mut Item {
        match index {
            0 => &mut self.main_hand,
            1 => &mut self.off_hand,
            2 => &mut self.head,
            3 => &mut self.body,
            4 => &mut self.hands,
            6 => &mut self.legs,
            7 => &mut self.feet,
            8 => &mut self.ears,
            9 => &mut self.neck,
            10 => &mut self.wrists,
            11 => &mut self.right_ring,
            12 => &mut self.left_ring,
            13 => &mut self.soul_crystal,
            _ => panic!("{} is not a valid src_container_index?!?", index),
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
            _ => panic!("{} is not a valid src_container_index?!?", index),
        }
    }
}
