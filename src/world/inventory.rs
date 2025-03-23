use physis::{
    common::{Language, Platform},
    gamedata::GameData,
};

use crate::config::get_config;

#[derive(Default, Copy, Clone)]
pub struct Item {
    pub quantity: u32,
    pub id: u32,
}

impl Item {
    pub fn new(quantity: u32, id: u32) -> Self {
        Self { quantity, id }
    }
}

#[derive(Default, Clone, Copy)]
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
}

pub struct Inventory {
    pub equipped: EquippedContainer,
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
}
