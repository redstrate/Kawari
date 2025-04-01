use physis::{
    common::{Language, Platform},
    gamedata::GameData,
};
use serde::{Deserialize, Serialize};

use crate::config::get_config;

use super::ipc::{ContainerType, InventoryModify};

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

#[derive(Default, Clone, Copy, Deserialize, Serialize, Debug)]
pub struct EquippedContainer {
    pub main_hand: Item,
    pub off_hand: Item,
    pub head: Item,
    pub body: Item,
    pub hands: Item,
    pub legs: Item,
    pub feet: Item,
    pub ears: Item,
    pub neck: Item,
    pub wrists: Item,
    pub right_ring: Item,
    pub left_ring: Item,
    pub soul_crystal: Item,
}

impl EquippedContainer {
    pub fn num_items(&self) -> u32 {
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

    pub fn get_slot(&mut self, index: u16) -> &mut Item {
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
            _ => panic!("Not a valid src_container_index?!?"),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct Inventory {
    pub equipped: EquippedContainer,
    pub extra_slot: Item, // WIP for inventory pages
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    pub fn new() -> Self {
        Self {
            equipped: EquippedContainer::default(),
            extra_slot: Item::default(),
        }
    }

    /// Equip the starting items for a given race
    pub fn equip_racial_items(&mut self, race_id: u8, gender: u8) {
        let config = get_config();

        let mut game_data =
            GameData::from_existing(Platform::Win32, &config.game_location).unwrap();

        let exh = game_data.read_excel_sheet_header("Race").unwrap();
        let exd = game_data
            .read_excel_sheet("Race", &exh, Language::English, 0)
            .unwrap();

        let world_row = &exd.read_row(&exh, race_id as u32).unwrap()[0];

        let get_column = |column_index: usize| {
            let physis::exd::ColumnData::Int32(item_id) = &world_row.data[column_index] else {
                panic!("Unexpected type!");
            };

            *item_id
        };

        if gender == 0 {
            self.equipped.body = Item::new(1, get_column(2) as u32);
            self.equipped.hands = Item::new(1, get_column(3) as u32);
            self.equipped.legs = Item::new(1, get_column(4) as u32);
            self.equipped.feet = Item::new(1, get_column(5) as u32);
        } else {
            self.equipped.body = Item::new(1, get_column(6) as u32);
            self.equipped.hands = Item::new(1, get_column(7) as u32);
            self.equipped.legs = Item::new(1, get_column(8) as u32);
            self.equipped.feet = Item::new(1, get_column(9) as u32);
        }

        // TODO: don't hardcode
        self.equipped.main_hand = Item::new(1, 0x00000641);
        self.equipped.ears = Item::new(1, 0x00003b1b);
        self.equipped.neck = Item::new(1, 0x00003b1a);
        self.equipped.wrists = Item::new(1, 0x00003b1c);
        self.equipped.right_ring = Item::new(1, 0x0000114a);
        self.equipped.left_ring = Item::new(1, 0x00003b1d);
    }

    pub fn process_action(&mut self, action: &InventoryModify) {
        // equipped
        if action.src_storage_id == ContainerType::Equipped {
            let src_slot = self.equipped.get_slot(action.src_container_index);

            // it only unequips for now, doesn't move the item
            *src_slot = Item::default();
        } else if action.src_storage_id == ContainerType::Inventory0 {
            let dst_slot = self.equipped.get_slot(action.dst_container_index);

            *dst_slot = self.extra_slot;
        }
    }
}
