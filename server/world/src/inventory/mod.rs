use icarus::{ClassJob::ClassJobSheet, Race::RaceSheet};
use kawari::common::{ContainerType, GameData, ItemInfoQuery, ItemOperationKind};
use physis::common::Language;
use serde::{Deserialize, Serialize};

use kawari::ipc::zone::ItemOperation;

mod buyback;
pub use buyback::{BuyBackItem, BuyBackList};

mod equipped;
pub use equipped::EquippedStorage;

mod generic;
pub use generic::GenericStorage;

mod item;
pub use item::Item;

mod storage;
pub use storage::{Storage, get_next_free_slot};

mod currency;
pub use currency::{CurrencyKind, CurrencyStorage};

mod iterators;
pub use iterators::{InventoryIterator, get_container_type};

const MAX_NORMAL_STORAGE: usize = 35;
const MAX_LARGE_STORAGE: usize = 50;

#[derive(Debug)]
pub struct ItemDestinationInfo {
    pub container: ContainerType,
    pub index: u16,
    pub quantity: u32,
}

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
    pub currency: CurrencyStorage,
}

impl Default for Inventory {
    fn default() -> Self {
        // TODO: Set the ContainerType for the others if needed?
        // Right now we only use this for adding items to the main inventory.
        Self {
            equipped: EquippedStorage::default(),
            pages: [
                GenericStorage {
                    kind: ContainerType::Inventory0,
                    ..Default::default()
                },
                GenericStorage {
                    kind: ContainerType::Inventory1,
                    ..Default::default()
                },
                GenericStorage {
                    kind: ContainerType::Inventory2,
                    ..Default::default()
                },
                GenericStorage {
                    kind: ContainerType::Inventory3,
                    ..Default::default()
                },
            ],
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
            currency: CurrencyStorage::default(),
        }
    }
}

impl rusqlite::types::FromSql for Inventory {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(serde_json::from_str(&String::column_result(value)?).unwrap())
    }
}

impl Inventory {
    /// Equip the starting items for a given classjob
    pub fn equip_classjob_items(&mut self, classjob_id: u16, game_data: &mut GameData) {
        let sheet = ClassJobSheet::read_from(&mut game_data.resource, Language::English).unwrap();
        let row = sheet.get_row(classjob_id as u32).unwrap();

        let main_hand_id = *row.ItemStartingWeapon().into_i32().unwrap() as u32;
        self.equipped.main_hand = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(main_hand_id))
                .unwrap(),
            1,
        );

        // TODO: don't hardcode
        self.equipped.ears = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1b))
                .unwrap(),
            1,
        );
        self.equipped.neck = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1a))
                .unwrap(),
            1,
        );
        self.equipped.wrists = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1c))
                .unwrap(),
            1,
        );
        // FIXME: I think this is actually based on a choice in the opening, and also defined in OpeningSystemDefine Excel sheet:
        self.equipped.right_ring = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(0x114a))
                .unwrap(),
            1,
        );
        self.equipped.left_ring = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1d))
                .unwrap(),
            1,
        );
    }

    /// Equip the starting items for a given race
    pub fn equip_racial_items(&mut self, race_id: u8, gender: u8, game_data: &mut GameData) {
        let sheet = RaceSheet::read_from(&mut game_data.resource, Language::English).unwrap();
        let row = sheet.get_row(race_id as u32).unwrap();

        let ids = if gender == 0 {
            [
                *row.RSEMBody().into_i32().unwrap() as u32,
                *row.RSEMHands().into_i32().unwrap() as u32,
                *row.RSEMLegs().into_i32().unwrap() as u32,
                *row.RSEMFeet().into_i32().unwrap() as u32,
            ]
        } else {
            [
                *row.RSEFBody().into_i32().unwrap() as u32,
                *row.RSEFHands().into_i32().unwrap() as u32,
                *row.RSEFLegs().into_i32().unwrap() as u32,
                *row.RSEFFeet().into_i32().unwrap() as u32,
            ]
        };

        self.equipped.body = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(ids[0]))
                .unwrap(),
            1,
        );
        self.equipped.hands = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(ids[1]))
                .unwrap(),
            1,
        );
        self.equipped.legs = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(ids[2]))
                .unwrap(),
            1,
        );
        self.equipped.feet = Item::new(
            game_data
                .get_item_info(ItemInfoQuery::ById(ids[3]))
                .unwrap(),
            1,
        );
    }

    /// Helper functions to reduce boilerplate
    pub fn get_item_mut(&mut self, storage_id: ContainerType, storage_index: u16) -> &mut Item {
        let container = self.get_container_mut(&storage_id);
        container.get_slot_mut(storage_index)
    }

    pub fn get_item(&self, storage_id: ContainerType, storage_index: u16) -> Item {
        if storage_id == ContainerType::Invalid {
            return Item::default();
        }

        let container = self.get_container(storage_id);
        *container.get_slot(storage_index)
    }

    pub fn process_action(&mut self, action: &ItemOperation) {
        match action.operation_type {
            ItemOperationKind::Discard => {
                let src_item = self.get_item_mut(action.src_storage_id, action.src_container_index);
                *src_item = Item::default();
            }
            ItemOperationKind::CombineStack => {
                let src_item;
                {
                    let original_item =
                        self.get_item_mut(action.src_storage_id, action.src_container_index);
                    src_item = *original_item;
                    *original_item = Item::default();
                }

                let dst_item = self.get_item_mut(action.dst_storage_id, action.dst_container_index);
                // TODO: We ought to check the max stack size for a given item id and disallow overflow
                dst_item.quantity += src_item.quantity;
            }
            ItemOperationKind::SplitStack => {
                let mut src_item;
                {
                    let original_item =
                        self.get_item_mut(action.src_storage_id, action.src_container_index);
                    if original_item.quantity >= action.dst_stack {
                        original_item.quantity -= action.dst_stack;
                        src_item = *original_item;
                        src_item.quantity = action.dst_stack
                    } else {
                        tracing::warn!(
                            "Client sent a bogus split amount: {}! Rejecting item operation!",
                            action.dst_stack
                        );
                        return;
                    }
                }

                let dst_item = self.get_item_mut(action.dst_storage_id, action.dst_container_index);
                dst_item.clone_from(&src_item);
            }
            ItemOperationKind::Exchange | ItemOperationKind::Move => {
                let src_item = self.get_item(action.src_storage_id, action.src_container_index);

                // move src item into dst slot
                let dst_slot = self.get_item_mut(action.dst_storage_id, action.dst_container_index);
                let dst_item = *dst_slot;
                dst_slot.clone_from(&src_item);

                // move dst item into src slot
                let src_slot = self.get_item_mut(action.src_storage_id, action.src_container_index);
                src_slot.clone_from(&dst_item);
            }
            _ => todo!(),
        }
    }

    fn add_in_empty_slot(&mut self, item: Item) -> Option<ItemDestinationInfo> {
        for page in &mut self.pages {
            for (slot_index, slot) in page.slots.iter_mut().enumerate() {
                if slot.quantity == 0 {
                    slot.clone_from(&item);
                    return Some(ItemDestinationInfo {
                        container: page.kind,
                        index: slot_index as u16,
                        quantity: item.quantity,
                    });
                }
            }
        }
        None
    }

    pub fn add_in_next_free_slot(&mut self, item: Item) -> Option<ItemDestinationInfo> {
        if item.stack_size > 1 {
            for page in &mut self.pages {
                for (slot_index, slot) in page.slots.iter_mut().enumerate() {
                    if slot.id == item.id && slot.quantity + item.quantity <= item.stack_size {
                        slot.quantity += item.quantity;
                        return Some(ItemDestinationInfo {
                            container: page.kind,
                            index: slot_index as u16,
                            quantity: slot.quantity,
                        });
                    }
                }
            }
        }

        // If we didn't find any stacks, or the item isn't stackable, try again to find an empty inventory slot.
        self.add_in_empty_slot(item)
    }

    pub fn add_in_slot(&mut self, item: Item, container_type: &ContainerType, index: u16) {
        let container = self.get_container_mut(container_type);
        let slot = container.get_slot_mut(index);
        slot.clone_from(&item);
    }

    fn get_container_mut(&mut self, container_type: &ContainerType) -> &mut dyn Storage {
        match container_type {
            ContainerType::Inventory0 => &mut self.pages[0],
            ContainerType::Inventory1 => &mut self.pages[1],
            ContainerType::Inventory2 => &mut self.pages[2],
            ContainerType::Inventory3 => &mut self.pages[3],
            ContainerType::Equipped => &mut self.equipped,
            ContainerType::Currency => &mut self.currency,
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
            _ => unimplemented!(),
        }
    }

    pub fn get_container(&self, container_type: ContainerType) -> &dyn Storage {
        match container_type {
            ContainerType::Inventory0 => &self.pages[0],
            ContainerType::Inventory1 => &self.pages[1],
            ContainerType::Inventory2 => &self.pages[2],
            ContainerType::Inventory3 => &self.pages[3],
            ContainerType::Equipped => &self.equipped,
            ContainerType::Currency => &self.currency,
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
            _ => unimplemented!(),
        }
    }

    pub fn get_main_weapon_id(&self, game_data: &mut GameData) -> u64 {
        game_data
            .get_primary_model_id(self.equipped.main_hand.apparent_id())
            .unwrap_or(0)
    }

    pub fn get_sub_weapon_id(&self, game_data: &mut GameData) -> u64 {
        // Use sub model from main hand (if available), e.g. quivers for bows. Otherwise fall back to off-hand, e.g. shields.
        if let Some(model) = game_data.get_sub_model_id(self.equipped.main_hand.apparent_id()) {
            model
        } else {
            game_data
                .get_primary_model_id(self.equipped.off_hand.apparent_id())
                .unwrap_or(0)
        }
    }

    pub fn get_model_ids(&self, game_data: &mut GameData) -> [u32; 10] {
        [
            game_data
                .get_primary_model_id(self.equipped.head.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.body.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.hands.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.legs.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.feet.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.ears.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.neck.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.wrists.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.left_ring.apparent_id())
                .unwrap_or(0) as u32,
            game_data
                .get_primary_model_id(self.equipped.right_ring.apparent_id())
                .unwrap_or(0) as u32,
        ]
    }
}
