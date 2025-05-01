use physis::common::Language;
use serde::{Deserialize, Serialize};

use crate::common::GameData;

use super::ipc::{ContainerType, InventoryModify};

// TODO: rename to storage?
pub trait Container {
    fn num_items(&self) -> u32;
    fn get_slot_mut(&mut self, index: u16) -> &mut Item;
    fn get_slot(&self, index: u16) -> &Item;
}

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

    // only for the iterator, so it can skip over it
    pub belt: Item,
}

impl Container for EquippedContainer {
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

impl<'a> IntoIterator for &'a EquippedContainer {
    type Item = &'a Item;
    type IntoIter = EquippedContainerIterator<'a>;
    fn into_iter(self) -> EquippedContainerIterator<'a> {
        EquippedContainerIterator {
            equipped: self,
            curr: 0,
        }
    }
}

pub struct EquippedContainerIterator<'a> {
    equipped: &'a EquippedContainer,
    curr: u16,
}

impl<'a> Iterator for EquippedContainerIterator<'a> {
    type Item = &'a Item;

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr;
        self.curr += 1;

        if self.curr >= 14 {
            return None;
        }

        Some(self.equipped.get_slot(curr))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InventoryPage {
    pub slots: Vec<Item>,
}

impl InventoryPage {
    fn default() -> Self {
        Self {
            slots: vec![Item::default(); 35],
        }
    }
}

impl Container for InventoryPage {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Inventory {
    pub equipped: EquippedContainer,
    pub pages: [InventoryPage; 4],
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            equipped: EquippedContainer::default(),
            pages: std::array::from_fn(|_| InventoryPage::default()),
        }
    }
}

impl Inventory {
    /// Equip the starting items for a given race
    pub fn equip_racial_items(&mut self, race_id: u8, gender: u8, game_data: &mut GameData) {
        let exh = game_data.game_data.read_excel_sheet_header("Race").unwrap();
        let exd = game_data
            .game_data
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
        if action.operation_type == 571 {
            // discard
            let src_container = self.get_container(&action.src_storage_id);
            let src_slot = src_container.get_slot_mut(action.src_container_index);
            *src_slot = Item::default();
        } else {
            // NOTE: only swaps items for now

            let src_item;
            // get the source item
            {
                let src_container = self.get_container(&action.src_storage_id);
                let src_slot = src_container.get_slot_mut(action.src_container_index);
                src_item = *src_slot;
            }

            let dst_item;
            // move into dst item
            {
                let dst_container = self.get_container(&action.dst_storage_id);
                let dst_slot = dst_container.get_slot_mut(action.dst_container_index);

                dst_item = *dst_slot;
                dst_slot.clone_from(&src_item);
            }

            // move dst item into src slot
            {
                let src_container = self.get_container(&action.src_storage_id);
                let src_slot = src_container.get_slot_mut(action.src_container_index);
                src_slot.clone_from(&dst_item);
            }
        }
    }

    pub fn add_in_next_free_slot(&mut self, item: Item) {
        for page in &mut self.pages {
            for slot in &mut page.slots {
                if slot.quantity == 0 {
                    slot.clone_from(&item);
                    return;
                }
            }
        }
    }

    fn get_container(&mut self, container_type: &ContainerType) -> &mut dyn Container {
        match container_type {
            ContainerType::Inventory0 => &mut self.pages[0],
            ContainerType::Inventory1 => &mut self.pages[1],
            ContainerType::Inventory2 => &mut self.pages[2],
            ContainerType::Inventory3 => &mut self.pages[3],
            ContainerType::Equipped => &mut self.equipped,
            ContainerType::ArmouryBody => todo!(),
        }
    }

    pub fn get_main_weapon_id(&self, game_data: &mut GameData) -> u64 {
        game_data
            .get_primary_model_id(self.equipped.main_hand.id)
            .unwrap_or(0)
    }

    pub fn get_model_ids(&self, game_data: &mut GameData) -> [u32; 10] {
        [
            game_data
                .get_primary_model_id(self.equipped.head.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.body.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.hands.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.legs.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.feet.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.ears.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.neck.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.wrists.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.left_ring.id)
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.right_ring.id)
                .unwrap_or(0) as u32,
        ]
    }
}
