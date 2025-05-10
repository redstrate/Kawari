use icarus::Race::RaceSheet;
use physis::common::Language;
use serde::{Deserialize, Serialize};

use crate::common::GameData;

use crate::ipc::zone::ItemOperation;

mod equipped;
pub use equipped::EquippedStorage;

mod generic;
pub use generic::GenericStorage;

mod item;
pub use item::Item;

mod storage;
pub use storage::{ContainerType, Storage};

const MAX_NORMAL_STORAGE: usize = 35;
const MAX_LARGE_STORAGE: usize = 50;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Inventory {
    pub equipped: EquippedStorage,
    pub pages: [GenericStorage<MAX_NORMAL_STORAGE>; 4],
    pub armoury_main_hand: GenericStorage<MAX_LARGE_STORAGE>,
    pub armoury_head: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_body: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_hands: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_legs: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_feet: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_off_hand: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_earring: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_necklace: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_bracelet: GenericStorage<MAX_NORMAL_STORAGE>,
    pub armoury_rings: GenericStorage<MAX_LARGE_STORAGE>,
    pub armoury_soul_crystal: GenericStorage<MAX_NORMAL_STORAGE>,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            equipped: EquippedStorage::default(),
            pages: std::array::from_fn(|_| GenericStorage::default()),
            armoury_main_hand: GenericStorage::default(),
            armoury_head: GenericStorage::default(),
            armoury_body: GenericStorage::default(),
            armoury_hands: GenericStorage::default(),
            armoury_legs: GenericStorage::default(),
            armoury_feet: GenericStorage::default(),
            armoury_off_hand: GenericStorage::default(),
            armoury_earring: GenericStorage::default(),
            armoury_necklace: GenericStorage::default(),
            armoury_bracelet: GenericStorage::default(),
            armoury_rings: GenericStorage::default(),
            armoury_soul_crystal: GenericStorage::default(),
        }
    }
}

impl<'a> IntoIterator for &'a Inventory {
    type Item = (ContainerType, &'a dyn Storage);
    type IntoIter = InventoryIterator<'a>;
    fn into_iter(self) -> InventoryIterator<'a> {
        InventoryIterator {
            inventory: self,
            curr: 0,
        }
    }
}

pub struct InventoryIterator<'a> {
    inventory: &'a Inventory,
    curr: u16,
}

impl<'a> Iterator for InventoryIterator<'a> {
    type Item = (ContainerType, &'a dyn Storage);

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr;
        self.curr += 1;

        if curr >= 17 {
            return None;
        }

        let container_type = match curr {
            // inventory
            0 => ContainerType::Inventory0,
            1 => ContainerType::Inventory1,
            2 => ContainerType::Inventory2,
            3 => ContainerType::Inventory3,

            // armory
            4 => ContainerType::ArmoryOffWeapon,
            5 => ContainerType::ArmoryHead,
            6 => ContainerType::ArmoryBody,
            7 => ContainerType::ArmoryHand,
            8 => ContainerType::ArmoryLeg,
            9 => ContainerType::ArmoryFoot,
            10 => ContainerType::ArmoryEarring,
            11 => ContainerType::ArmoryNeck,
            12 => ContainerType::ArmoryWrist,
            13 => ContainerType::ArmoryRing,
            14 => ContainerType::ArmorySoulCrystal,
            15 => ContainerType::ArmoryWeapon,

            // equipped
            16 => ContainerType::Equipped,
            _ => panic!("Inventory iterator invalid!"),
        };

        Some((
            container_type,
            self.inventory.get_container(&container_type),
        ))
    }
}

impl Inventory {
    /// Equip the starting items for a given race
    pub fn equip_racial_items(&mut self, race_id: u8, gender: u8, game_data: &mut GameData) {
        let sheet = RaceSheet::read_from(&mut game_data.game_data, Language::English).unwrap();
        let row = sheet.get_row(race_id as u32).unwrap();

        if gender == 0 {
            self.equipped.body = Item::new(1, *row.RSEMBody().into_i32().unwrap() as u32);
            self.equipped.hands = Item::new(1, *row.RSEMHands().into_i32().unwrap() as u32);
            self.equipped.legs = Item::new(1, *row.RSEMLegs().into_i32().unwrap() as u32);
            self.equipped.feet = Item::new(1, *row.RSEMFeet().into_i32().unwrap() as u32);
        } else {
            self.equipped.body = Item::new(1, *row.RSEFBody().into_i32().unwrap() as u32);
            self.equipped.hands = Item::new(1, *row.RSEFHands().into_i32().unwrap() as u32);
            self.equipped.legs = Item::new(1, *row.RSEFLegs().into_i32().unwrap() as u32);
            self.equipped.feet = Item::new(1, *row.RSEFFeet().into_i32().unwrap() as u32);
        }

        // TODO: don't hardcode
        self.equipped.main_hand = Item::new(1, 0x00000641);
        self.equipped.ears = Item::new(1, 0x00003b1b);
        self.equipped.neck = Item::new(1, 0x00003b1a);
        self.equipped.wrists = Item::new(1, 0x00003b1c);
        self.equipped.right_ring = Item::new(1, 0x0000114a);
        self.equipped.left_ring = Item::new(1, 0x00003b1d);
    }

    pub fn process_action(&mut self, action: &ItemOperation) {
        if action.operation_type == 78 {
            // discard
            let src_container = self.get_container_mut(&action.src_storage_id);
            let src_slot = src_container.get_slot_mut(action.src_container_index);
            *src_slot = Item::default();
        } else {
            // NOTE: only swaps items for now

            let src_item;
            // get the source item
            {
                let src_container = self.get_container_mut(&action.src_storage_id);
                let src_slot = src_container.get_slot_mut(action.src_container_index);
                src_item = *src_slot;
            }

            let dst_item;
            // move into dst item
            {
                let dst_container = self.get_container_mut(&action.dst_storage_id);
                let dst_slot = dst_container.get_slot_mut(action.dst_container_index);

                dst_item = *dst_slot;
                dst_slot.clone_from(&src_item);
            }

            // move dst item into src slot
            {
                let src_container = self.get_container_mut(&action.src_storage_id);
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

    fn get_container_mut(&mut self, container_type: &ContainerType) -> &mut dyn Storage {
        match container_type {
            ContainerType::Inventory0 => &mut self.pages[0],
            ContainerType::Inventory1 => &mut self.pages[1],
            ContainerType::Inventory2 => &mut self.pages[2],
            ContainerType::Inventory3 => &mut self.pages[3],
            ContainerType::Equipped => &mut self.equipped,
            ContainerType::ArmoryOffWeapon => &mut self.armoury_off_hand,
            ContainerType::ArmoryHead => &mut self.armoury_head,
            ContainerType::ArmoryBody => &mut self.armoury_body,
            ContainerType::ArmoryHand => &mut self.armoury_hands,
            ContainerType::ArmoryLeg => &mut self.armoury_legs,
            ContainerType::ArmoryFoot => &mut self.armoury_feet,
            ContainerType::ArmoryEarring => &mut self.armoury_earring,
            ContainerType::ArmoryNeck => &mut self.armoury_necklace,
            ContainerType::ArmoryWrist => &mut self.armoury_bracelet,
            ContainerType::ArmoryRing => &mut self.armoury_rings,
            ContainerType::ArmorySoulCrystal => &mut self.armoury_soul_crystal,
            ContainerType::ArmoryWeapon => &mut self.armoury_main_hand,
        }
    }

    fn get_container(&self, container_type: &ContainerType) -> &dyn Storage {
        match container_type {
            ContainerType::Inventory0 => &self.pages[0],
            ContainerType::Inventory1 => &self.pages[1],
            ContainerType::Inventory2 => &self.pages[2],
            ContainerType::Inventory3 => &self.pages[3],
            ContainerType::Equipped => &self.equipped,
            ContainerType::ArmoryOffWeapon => &self.armoury_off_hand,
            ContainerType::ArmoryHead => &self.armoury_head,
            ContainerType::ArmoryBody => &self.armoury_body,
            ContainerType::ArmoryHand => &self.armoury_hands,
            ContainerType::ArmoryLeg => &self.armoury_legs,
            ContainerType::ArmoryFoot => &self.armoury_feet,
            ContainerType::ArmoryEarring => &self.armoury_earring,
            ContainerType::ArmoryNeck => &self.armoury_necklace,
            ContainerType::ArmoryWrist => &self.armoury_bracelet,
            ContainerType::ArmoryRing => &self.armoury_rings,
            ContainerType::ArmorySoulCrystal => &self.armoury_soul_crystal,
            ContainerType::ArmoryWeapon => &self.armoury_main_hand,
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
