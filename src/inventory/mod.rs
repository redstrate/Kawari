use crate::common::{GameData, ItemInfoQuery};
use binrw::binrw;
use icarus::{ClassJob::ClassJobSheet, Race::RaceSheet};
use physis::common::Language;
use serde::{Deserialize, Serialize};

use crate::ipc::zone::ItemOperation;

mod buyback;
pub use buyback::{BuyBackItem, BuyBackList};

mod equipped;
pub use equipped::EquippedStorage;

mod generic;
pub use generic::GenericStorage;

mod item;
pub use item::Item;

mod storage;
pub use storage::{ContainerType, Storage};

mod currency;
pub use currency::CurrencyKind;
pub use currency::CurrencyStorage;

use crate::{
    INVENTORY_ACTION_COMBINE_STACK, INVENTORY_ACTION_DISCARD, INVENTORY_ACTION_EXCHANGE,
    INVENTORY_ACTION_MOVE, INVENTORY_ACTION_SPLIT_STACK, INVENTORY_ACTION_UPDATE_CURRENCY,
};

const MAX_NORMAL_STORAGE: usize = 35;
const MAX_LARGE_STORAGE: usize = 50;

#[binrw]
#[derive(Debug, Clone, Default, Copy, PartialEq)]
#[brw(repr = u8)]
#[repr(u8)]
pub enum ItemOperationKind {
    /// The operation opcode/type when updating the currency storage.
    UpdateCurrency = INVENTORY_ACTION_UPDATE_CURRENCY,
    /// The operation opcode/type when discarding an item from the inventory.
    Discard = INVENTORY_ACTION_DISCARD,
    #[default]
    /// The operation opcode/type when moving an item to an emtpy slot in the inventory.
    Move = INVENTORY_ACTION_MOVE,
    /// The operation opcode/type when moving an item to a slot occupied by another in the inventory.
    Exchange = INVENTORY_ACTION_EXCHANGE,
    /// The operation opcode/type when splitting stacks of identical items.
    SplitStack = INVENTORY_ACTION_SPLIT_STACK,
    /// The operation opcode/type when combining stacks of identical items.
    CombineStack = INVENTORY_ACTION_COMBINE_STACK,
}

impl TryFrom<u8> for ItemOperationKind {
    type Error = ();
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            x if x == ItemOperationKind::Discard as u8 => Ok(ItemOperationKind::Discard),
            x if x == ItemOperationKind::Move as u8 => Ok(ItemOperationKind::Move),
            x if x == ItemOperationKind::Exchange as u8 => Ok(ItemOperationKind::Exchange),
            x if x == ItemOperationKind::SplitStack as u8 => Ok(ItemOperationKind::SplitStack),
            x if x == ItemOperationKind::CombineStack as u8 => Ok(ItemOperationKind::CombineStack),
            _ => Err(()),
        }
    }
}

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
        let mut pages = std::array::from_fn(|_| GenericStorage::default());
        pages[0].kind = ContainerType::Inventory0;
        pages[1].kind = ContainerType::Inventory1;
        pages[2].kind = ContainerType::Inventory2;
        pages[3].kind = ContainerType::Inventory3;
        Self {
            equipped: EquippedStorage::default(),
            pages,
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

#[cfg(not(target_family = "wasm"))]
impl rusqlite::types::FromSql for Inventory {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(serde_json::from_str(&String::column_result(value)?).unwrap())
    }
}

pub struct InventoryIterator<'a> {
    inventory: &'a Inventory,
    curr: u16,
}

pub fn get_container_type(container_index: u32) -> Option<ContainerType> {
    match container_index {
        // inventory
        0 => Some(ContainerType::Inventory0),
        1 => Some(ContainerType::Inventory1),
        2 => Some(ContainerType::Inventory2),
        3 => Some(ContainerType::Inventory3),

        // armory
        4 => Some(ContainerType::ArmoryOffWeapon),
        5 => Some(ContainerType::ArmoryHead),
        6 => Some(ContainerType::ArmoryBody),
        7 => Some(ContainerType::ArmoryHand),
        8 => Some(ContainerType::ArmoryLeg),
        9 => Some(ContainerType::ArmoryFoot),
        10 => Some(ContainerType::ArmoryEarring),
        11 => Some(ContainerType::ArmoryNeck),
        12 => Some(ContainerType::ArmoryWrist),
        13 => Some(ContainerType::ArmoryRing),
        14 => Some(ContainerType::ArmorySoulCrystal),
        15 => Some(ContainerType::ArmoryWeapon),

        // equipped
        16 => Some(ContainerType::Equipped),

        // currency
        17 => Some(ContainerType::Currency),
        _ => panic!(
            "Inventory iterator invalid or the client sent a very weird packet! {container_index}"
        ),
    }
}

impl<'a> Iterator for InventoryIterator<'a> {
    type Item = (ContainerType, &'a dyn Storage);

    fn next(&mut self) -> Option<Self::Item> {
        let curr = self.curr;
        self.curr += 1;

        if curr >= 18 {
            return None;
        }

        let container_type = get_container_type(curr as u32).unwrap();

        Some((
            container_type,
            self.inventory.get_container(&container_type),
        ))
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

        let ids: Vec<u32> = if gender == 0 {
            vec![
                *row.RSEMBody().into_i32().unwrap() as u32,
                *row.RSEMHands().into_i32().unwrap() as u32,
                *row.RSEMLegs().into_i32().unwrap() as u32,
                *row.RSEMFeet().into_i32().unwrap() as u32,
            ]
        } else {
            vec![
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
    fn get_item_mut(&mut self, storage_id: ContainerType, storage_index: u16) -> &mut Item {
        let container = self.get_container_mut(&storage_id);
        container.get_slot_mut(storage_index)
    }

    pub fn get_item(&self, storage_id: ContainerType, storage_index: u16) -> Item {
        let container = self.get_container(&storage_id);
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
            _ => todo!(),
        }
    }

    fn get_container(&self, container_type: &ContainerType) -> &dyn Storage {
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
            _ => todo!(),
        }
    }

    pub fn get_main_weapon_id(&self, game_data: &mut GameData) -> u64 {
        game_data
            .get_primary_model_id(self.equipped.main_hand.apparent_id())
            .unwrap_or(0)
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
