use icarus::{ClassJob::ClassJobSheet, Race::RaceSheet};
use kawari::{
    common::{ContainerType, ItemOperationKind},
    config::get_config,
    ipc::zone::ItemInfo,
};
use serde::{Deserialize, Serialize};

use kawari::ipc::zone::ItemOperation;

mod buyback;
pub use buyback::BuyBackList;

mod equipped;
pub use equipped::{EQUIP_RESTRICTED, EquippedStorage};

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

mod crystals;
pub use crystals::{CrystalKind, CrystalsStorage};

use crate::{GameData, ItemInfoQuery};

use physis::TerritoryIntendedUse;

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
    pub currency: CurrencyStorage,
    pub crystals: CrystalsStorage,
    pub key_items: GenericStorage<MAX_NORMAL_STORAGE>,
}

impl Default for Inventory {
    fn default() -> Self {
        // Right now we only use this for adding items to the main inventory.
        Self {
            equipped: EquippedStorage::default(),
            pages: [
                GenericStorage::new(ContainerType::Inventory0),
                GenericStorage::new(ContainerType::Inventory1),
                GenericStorage::new(ContainerType::Inventory2),
                GenericStorage::new(ContainerType::Inventory3),
            ],
            armoury_main_hand: GenericStorage::new(ContainerType::ArmoryWeapon),
            armoury_head: GenericStorage::new(ContainerType::ArmoryHead),
            armoury_body: GenericStorage::new(ContainerType::ArmoryBody),
            armoury_hands: GenericStorage::new(ContainerType::ArmoryHand),
            armoury_legs: GenericStorage::new(ContainerType::ArmoryLeg),
            armoury_feet: GenericStorage::new(ContainerType::ArmoryFoot),
            armoury_off_hand: GenericStorage::new(ContainerType::ArmoryOffWeapon),
            armoury_earring: GenericStorage::new(ContainerType::ArmoryEarring),
            armoury_necklace: GenericStorage::new(ContainerType::ArmoryNeck),
            armoury_bracelet: GenericStorage::new(ContainerType::ArmoryWrist),
            armoury_rings: GenericStorage::new(ContainerType::ArmoryRing),
            armoury_soul_crystal: GenericStorage::new(ContainerType::ArmorySoulCrystal),
            currency: CurrencyStorage::default(),
            crystals: CrystalsStorage::default(),
            key_items: GenericStorage::new(ContainerType::KeyItems),
        }
    }
}

impl Inventory {
    /// Equip the starting items for a given classjob
    pub fn equip_classjob_items(&mut self, classjob_id: u16, game_data: &mut GameData) {
        let config = get_config();
        let sheet =
            ClassJobSheet::read_from(&mut game_data.resource, config.world.language()).unwrap();
        let row = sheet.row(classjob_id as u32).unwrap();

        let main_hand_id = row.ItemStartingWeaponMainHand() as u32;
        self.equipped.main_hand = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(main_hand_id))
                .unwrap(),
            1,
        );

        // TODO: don't hardcode
        self.equipped.ears = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1b))
                .unwrap(),
            1,
        );
        self.equipped.neck = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1a))
                .unwrap(),
            1,
        );
        self.equipped.wrists = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1c))
                .unwrap(),
            1,
        );
        // FIXME: I think this is actually based on a choice in the opening, and also defined in OpeningSystemDefine Excel sheet:
        self.equipped.right_ring = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(0x114a))
                .unwrap(),
            1,
        );
        self.equipped.left_ring = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(0x3b1d))
                .unwrap(),
            1,
        );
    }

    /// Equip the starting items for a given race
    pub fn equip_racial_items(&mut self, race_id: u8, gender: u8, game_data: &mut GameData) {
        let config = get_config();
        let sheet = RaceSheet::read_from(&mut game_data.resource, config.world.language()).unwrap();
        let row = sheet.row(race_id as u32).unwrap();

        let ids = if gender == 0 {
            [
                row.RSEMBody() as u32,
                row.RSEMHands() as u32,
                row.RSEMLegs() as u32,
                row.RSEMFeet() as u32,
            ]
        } else {
            [
                row.RSEFBody() as u32,
                row.RSEFHands() as u32,
                row.RSEFLegs() as u32,
                row.RSEFFeet() as u32,
            ]
        };

        self.equipped.body = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(ids[0]))
                .unwrap(),
            1,
        );
        self.equipped.hands = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(ids[1]))
                .unwrap(),
            1,
        );
        self.equipped.legs = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(ids[2]))
                .unwrap(),
            1,
        );
        self.equipped.feet = Item::new(
            &game_data
                .get_item_info(ItemInfoQuery::ById(ids[3]))
                .unwrap(),
            1,
        );
    }

    /// Helper functions to reduce boilerplate
    pub fn get_item_mut(
        &mut self,
        storage_id: ContainerType,
        storage_index: u16,
    ) -> Option<&mut Item> {
        if let Some(container) = self.get_container_mut(&storage_id) {
            return Some(container.get_slot_mut(storage_index));
        }

        None
    }

    pub fn get_item(&self, storage_id: ContainerType, storage_index: u16) -> Option<Item> {
        if let Some(container) = self.get_container(storage_id) {
            return Some(*container.get_slot(storage_index));
        }

        None
    }

    pub fn process_action(&mut self, action: &ItemOperation) {
        match action.operation_type {
            ItemOperationKind::Discard => {
                if let Some(src_item) =
                    self.get_item_mut(action.src_storage_id, action.src_container_index)
                {
                    *src_item = Item::default();
                }
            }
            ItemOperationKind::CombineStack => {
                let src_item;
                {
                    if let Some(original_item) =
                        self.get_item_mut(action.src_storage_id, action.src_container_index)
                    {
                        src_item = *original_item;
                        *original_item = Item::default();
                    } else {
                        return;
                    }
                }

                if let Some(dst_item) =
                    self.get_item_mut(action.dst_storage_id, action.dst_container_index)
                {
                    // TODO: We ought to check the max stack size for a given item id and disallow overflow
                    dst_item.quantity += src_item.quantity;
                }
            }
            ItemOperationKind::SplitStack => {
                let mut src_item;
                {
                    let Some(original_item) =
                        self.get_item_mut(action.src_storage_id, action.src_container_index)
                    else {
                        tracing::warn!(
                            "Client sent a bogus storage id: {}! Rejecting item operation!",
                            action.src_storage_id
                        );
                        return;
                    };
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

                if let Some(dst_item) =
                    self.get_item_mut(action.dst_storage_id, action.dst_container_index)
                {
                    dst_item.clone_from(&src_item);
                }
            }
            ItemOperationKind::Exchange | ItemOperationKind::Move => {
                let src_item;

                // Clear existing item so add in next free slot checks work.
                {
                    if let Some(src_slot) =
                        self.get_item_mut(action.src_storage_id, action.src_container_index)
                    {
                        src_item = *src_slot;
                        src_slot.quantity = 0;
                    } else {
                        return;
                    }
                }

                // move src item into dst slot
                if let Some(dst_slot) =
                    self.get_item_mut(action.dst_storage_id, action.dst_container_index)
                {
                    let dst_item = *dst_slot;
                    dst_slot.clone_from(&src_item);

                    // move dst item into src slot
                    if let Some(src_slot) =
                        self.get_item_mut(action.src_storage_id, action.src_container_index)
                    {
                        src_slot.clone_from(&dst_item);
                    }
                }
            }
            _ => todo!(),
        }
    }

    fn add_in_empty_slot(&mut self, item: Item) -> Option<ItemInfo> {
        for page in &mut self.pages {
            for (slot_index, slot) in page.slots.iter_mut().enumerate() {
                if slot.quantity == 0 {
                    slot.clone_from(&item);
                    return Some(ItemInfo {
                        slot: slot_index as u16,
                        container: page.kind,
                        ..(*slot).into()
                    });
                }
            }
        }
        None
    }

    pub fn add_in_next_free_slot(&mut self, item: Item) -> Option<ItemInfo> {
        if item.stack_size > 1 {
            for page in &mut self.pages {
                for (slot_index, slot) in page.slots.iter_mut().enumerate() {
                    if slot.item_id == item.item_id
                        && slot.quantity + item.quantity <= item.stack_size
                    {
                        slot.quantity += item.quantity;
                        return Some(ItemInfo {
                            slot: slot_index as u16,
                            container: page.kind,
                            ..(*slot).into()
                        });
                    }
                }
            }
        }

        // If we didn't find any stacks, or the item isn't stackable, try again to find an empty inventory slot.
        self.add_in_empty_slot(item)
    }

    pub fn add_in_next_free_armory_slot(&self, equip_index: u16) -> Option<ItemInfo> {
        let container_type = ContainerType::from_equip_slot(equip_index as u8);

        if let Some(container) = self.get_container(container_type) {
            for i in 0..container.max_slots() {
                if container.get_slot(i as u16).quantity == 0 {
                    return Some(ItemInfo {
                        slot: i as u16,
                        container: container_type,
                        ..(*container.get_slot(i as u16)).into()
                    });
                }
            }
        }

        None
    }

    pub fn add_in_slot(&mut self, item: Item, container_type: &ContainerType, index: u16) {
        let Some(container) = self.get_container_mut(container_type) else {
            return;
        };

        let slot = container.get_slot_mut(index);
        slot.clone_from(&item);
    }

    fn get_container_mut(&mut self, container_type: &ContainerType) -> Option<&mut dyn Storage> {
        match container_type {
            ContainerType::Inventory0 => Some(&mut self.pages[0]),
            ContainerType::Inventory1 => Some(&mut self.pages[1]),
            ContainerType::Inventory2 => Some(&mut self.pages[2]),
            ContainerType::Inventory3 => Some(&mut self.pages[3]),
            ContainerType::Equipped => Some(&mut self.equipped),
            ContainerType::Currency => Some(&mut self.currency),
            ContainerType::Crystals => Some(&mut self.crystals),
            ContainerType::ArmoryOffWeapon => Some(&mut self.armoury_off_hand),
            ContainerType::ArmoryHead => Some(&mut self.armoury_head),
            ContainerType::ArmoryBody => Some(&mut self.armoury_body),
            ContainerType::ArmoryHand => Some(&mut self.armoury_hands),
            ContainerType::ArmoryLeg => Some(&mut self.armoury_legs),
            ContainerType::ArmoryFoot => Some(&mut self.armoury_feet),
            ContainerType::ArmoryEarring => Some(&mut self.armoury_earring),
            ContainerType::ArmoryNeck => Some(&mut self.armoury_necklace),
            ContainerType::ArmoryWrist => Some(&mut self.armoury_bracelet),
            ContainerType::ArmoryRing => Some(&mut self.armoury_rings),
            ContainerType::ArmorySoulCrystal => Some(&mut self.armoury_soul_crystal),
            ContainerType::ArmoryWeapon => Some(&mut self.armoury_main_hand),
            ContainerType::KeyItems => Some(&mut self.key_items),
            _ => None,
        }
    }

    pub fn get_container(&self, container_type: ContainerType) -> Option<&dyn Storage> {
        match container_type {
            ContainerType::Inventory0 => Some(&self.pages[0]),
            ContainerType::Inventory1 => Some(&self.pages[1]),
            ContainerType::Inventory2 => Some(&self.pages[2]),
            ContainerType::Inventory3 => Some(&self.pages[3]),
            ContainerType::Equipped => Some(&self.equipped),
            ContainerType::Currency => Some(&self.currency),
            ContainerType::Crystals => Some(&self.crystals),
            ContainerType::ArmoryOffWeapon => Some(&self.armoury_off_hand),
            ContainerType::ArmoryHead => Some(&self.armoury_head),
            ContainerType::ArmoryBody => Some(&self.armoury_body),
            ContainerType::ArmoryHand => Some(&self.armoury_hands),
            ContainerType::ArmoryLeg => Some(&self.armoury_legs),
            ContainerType::ArmoryFoot => Some(&self.armoury_feet),
            ContainerType::ArmoryEarring => Some(&self.armoury_earring),
            ContainerType::ArmoryNeck => Some(&self.armoury_necklace),
            ContainerType::ArmoryWrist => Some(&self.armoury_bracelet),
            ContainerType::ArmoryRing => Some(&self.armoury_rings),
            ContainerType::ArmorySoulCrystal => Some(&self.armoury_soul_crystal),
            ContainerType::ArmoryWeapon => Some(&self.armoury_main_hand),
            ContainerType::KeyItems => Some(&self.key_items),
            _ => None,
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

    fn prepare_items_in_container(container: &mut impl Storage, data: &mut GameData) {
        for index in 0..container.max_slots() {
            let item = container.get_slot_mut(index as u16);

            if item.is_empty_slot() {
                continue;
            }

            if let Some(info) = data.get_item_info(ItemInfoQuery::ById(item.item_id)) {
                *item = Item {
                    quantity: item.quantity,
                    item_id: item.item_id,
                    crafter_content_id: item.crafter_content_id,
                    item_flags: item.item_flags,
                    condition: item.condition,
                    spiritbond_or_collectability: item.spiritbond_or_collectability,
                    glamour_id: item.glamour_id,
                    materia: item.materia,
                    materia_grades: item.materia_grades,
                    stains: item.stains,
                    ..info.into()
                };
            }
        }
    }

    pub fn prepare_player_inventory(inventory: &mut Inventory, data: &mut GameData) {
        // TODO: implement iter_mut for Inventory so all of this can be reduced down
        for index in 0..inventory.pages.len() {
            Self::prepare_items_in_container(&mut inventory.pages[index], data);
        }

        Self::prepare_items_in_container(&mut inventory.equipped, data);
        Self::prepare_items_in_container(&mut inventory.armoury_main_hand, data);
        Self::prepare_items_in_container(&mut inventory.armoury_body, data);
        Self::prepare_items_in_container(&mut inventory.armoury_hands, data);
        Self::prepare_items_in_container(&mut inventory.armoury_legs, data);
        Self::prepare_items_in_container(&mut inventory.armoury_feet, data);
        Self::prepare_items_in_container(&mut inventory.armoury_off_hand, data);
        Self::prepare_items_in_container(&mut inventory.armoury_earring, data);
        Self::prepare_items_in_container(&mut inventory.armoury_necklace, data);
        Self::prepare_items_in_container(&mut inventory.armoury_bracelet, data);
        Self::prepare_items_in_container(&mut inventory.armoury_rings, data);
        Self::prepare_items_in_container(&mut inventory.key_items, data);
        // Skip soul crystals
    }

    /// Equips the given soul crystal and places the old one (if any) back into the Armoury Chest.
    pub fn equip_soul_crystal(&mut self, id: u32) {
        // NOTE: This has to match client behavior exactly! See ItemOperation code for more details.

        // If we already have it equipped, do nothing.
        if self.equipped.soul_crystal.item_id == id && self.equipped.soul_crystal.quantity > 0 {
            return;
        }

        for i in 0..self.armoury_soul_crystal.max_slots() as u16 {
            // Find the soul crystal in the Armoury Chest.
            let armoury_slot = self.armoury_soul_crystal.get_slot(i);
            if armoury_slot.item_id == id && armoury_slot.quantity > 0 {
                // Perform the swap.
                let operation = ItemOperation {
                    operation_type: ItemOperationKind::Exchange,
                    src_storage_id: ContainerType::ArmorySoulCrystal,
                    src_container_index: i,
                    dst_storage_id: ContainerType::Equipped,
                    dst_container_index: 13,
                    ..Default::default()
                };
                self.process_action(&operation);

                return;
            }
        }
    }

    /// Checks if the soul crystal exists in your inventory.
    pub fn has_soul_crystal(&mut self, id: u32) -> bool {
        // TODO: can you move the soul crystal somewhere else?

        for i in 0..self.armoury_soul_crystal.max_slots() as u16 {
            // Find the soul crystal in the Armoury Chest.
            let armoury_slot = self.armoury_soul_crystal.get_slot(i);
            if armoury_slot.item_id == id && armoury_slot.quantity > 0 {
                return true;
            }
        }

        false
    }

    /// Puts the equipment in `slot` back into the Armoury Chest.
    pub fn unequip_equipment(&mut self, slot: u16) {
        // NOTE: This has to match client behavior exactly! See ItemOperation code for more details.

        // If we already have nothing, do nothing.
        if self.equipped.get_slot(slot).quantity == 0 {
            return;
        }

        let destination_info = self.add_in_next_free_armory_slot(slot).unwrap();
        self.add_in_slot(
            *self.equipped.get_slot(slot),
            &destination_info.container,
            destination_info.slot,
        );

        *self.equipped.get_slot_mut(slot) = Item::default();
    }
}

/// Represents a single housing plot's collective inventory, both inside and out.
// TODO: This will need to adjustments in 7.5x
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HousingInventory {
    //pub plot_size: kawari::ipc::zone::PlotSize,
    pub interior: Vec<GenericStorage<MAX_LARGE_STORAGE>>,
    pub interior_storeroom: Vec<GenericStorage<MAX_LARGE_STORAGE>>,
    // TODO: Unclear if we need to emulate this
    pub interior_appearance: Vec<GenericStorage<MAX_LARGE_STORAGE>>,
    pub exterior: Vec<GenericStorage<MAX_LARGE_STORAGE>>,
    pub exterior_storeroom: Vec<GenericStorage<MAX_LARGE_STORAGE>>,
    // TODO: Unclear if we need to emulate this
    pub exterior_appearance: Vec<GenericStorage<MAX_LARGE_STORAGE>>,
}

impl HousingInventory {
    pub const INT_STORAGE_APT_FC: usize = 2;
    pub const INT_STORAGE_SMALL: usize = 3;
    pub const INT_STORAGE_MEDIUM: usize = 4;
    pub const INT_STORAGE_MANSION: usize = 8;
}

// By default, be an apartment/fc chamber.
impl Default for HousingInventory {
    fn default() -> Self {
        Self {
            // plot_size: kawari::ipc::zone::PlotSize::Small,
            interior: vec![
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems1,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems2,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems3,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems4,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems5,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems6,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems7,
                ),
                GenericStorage::<MAX_LARGE_STORAGE>::new(
                    ContainerType::HousingInteriorPlacedItems8,
                ),
            ],
            interior_storeroom: vec![
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom1),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom2),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom3),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom4),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom5),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom6),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom7),
                GenericStorage::<MAX_LARGE_STORAGE>::new(ContainerType::HousingInteriorStoreroom8),
            ],
            interior_appearance: vec![GenericStorage::<MAX_LARGE_STORAGE>::new(
                ContainerType::HousingExteriorAppearance,
            )],

            exterior: vec![GenericStorage::<MAX_LARGE_STORAGE>::new(
                ContainerType::HousingExteriorPlacedItems,
            )],
            exterior_storeroom: vec![GenericStorage::<MAX_LARGE_STORAGE>::new(
                ContainerType::HousingExteriorPlacedItems,
            )],
            exterior_appearance: vec![GenericStorage::<MAX_LARGE_STORAGE>::new(
                ContainerType::HousingExteriorAppearance,
            )],
        }
    }
}

impl HousingInventory {
    pub fn add_in_empty_slot(
        &mut self,
        item: Item,
        desired_pages: DesiredHousingInventoryPages,
    ) -> Option<ItemInfo> {
        let desired_pages = match desired_pages {
            DesiredHousingInventoryPages::Interior => &mut self.interior,
            DesiredHousingInventoryPages::InteriorStoreroom => &mut self.interior_storeroom,
            DesiredHousingInventoryPages::Exterior => {
                if !self.exterior.is_empty() {
                    &mut self.exterior
                } else {
                    return None;
                }
            }
            DesiredHousingInventoryPages::ExteriorStoreroom => {
                if !self.exterior_storeroom.is_empty() {
                    &mut self.exterior_storeroom
                } else {
                    return None;
                }
            }
            DesiredHousingInventoryPages::None => {
                return None;
            }
        };

        for page in desired_pages {
            for (slot_index, slot) in page.slots.iter_mut().enumerate() {
                if slot.quantity == 0 {
                    slot.clone_from(&item);
                    return Some(ItemInfo {
                        slot: slot_index as u16,
                        container: page.kind,
                        ..(*slot).into()
                    });
                }
            }
        }
        None
    }

    pub fn get_desired_pages_from_intendeduse(
        &self,
        intended_use: TerritoryIntendedUse,
        storeroom: bool,
    ) -> DesiredHousingInventoryPages {
        match intended_use {
            TerritoryIntendedUse::HousingOutdoor => {
                if storeroom {
                    DesiredHousingInventoryPages::ExteriorStoreroom
                } else {
                    DesiredHousingInventoryPages::Exterior
                }
            }
            TerritoryIntendedUse::HousingIndoor => {
                if storeroom {
                    DesiredHousingInventoryPages::InteriorStoreroom
                } else {
                    DesiredHousingInventoryPages::Interior
                }
            }
            _ => DesiredHousingInventoryPages::None,
        }
    }

    fn get_container_mut(&mut self, container_type: &ContainerType) -> Option<&mut dyn Storage> {
        match container_type {
            ContainerType::HousingInteriorPlacedItems1 => Some(&mut self.interior[0]),
            ContainerType::HousingInteriorPlacedItems2 => Some(&mut self.interior[1]),
            ContainerType::HousingInteriorPlacedItems3 => Some(&mut self.interior[2]),
            ContainerType::HousingInteriorPlacedItems4 => Some(&mut self.interior[3]),
            ContainerType::HousingInteriorPlacedItems5 => Some(&mut self.interior[4]),
            ContainerType::HousingInteriorPlacedItems6 => Some(&mut self.interior[5]),
            ContainerType::HousingInteriorPlacedItems7 => Some(&mut self.interior[6]),
            ContainerType::HousingInteriorPlacedItems8 => Some(&mut self.interior[7]),

            ContainerType::HousingInteriorStoreroom1 => Some(&mut self.interior_storeroom[0]),
            ContainerType::HousingInteriorStoreroom2 => Some(&mut self.interior_storeroom[1]),
            ContainerType::HousingInteriorStoreroom3 => Some(&mut self.interior_storeroom[2]),
            ContainerType::HousingInteriorStoreroom4 => Some(&mut self.interior_storeroom[3]),
            ContainerType::HousingInteriorStoreroom5 => Some(&mut self.interior_storeroom[4]),
            ContainerType::HousingInteriorStoreroom6 => Some(&mut self.interior_storeroom[5]),
            ContainerType::HousingInteriorStoreroom7 => Some(&mut self.interior_storeroom[6]),
            ContainerType::HousingInteriorStoreroom8 => Some(&mut self.interior_storeroom[7]),

            ContainerType::HousingInteriorAppearance => Some(&mut self.interior_appearance[0]),
            ContainerType::HousingExteriorAppearance => Some(&mut self.exterior_appearance[0]),

            ContainerType::HousingExteriorPlacedItems => Some(&mut self.exterior[0]),
            ContainerType::HousingExteriorStoreroom => Some(&mut self.exterior_storeroom[0]),
            _ => None,
        }
    }

    pub fn get_container(&self, container_type: ContainerType) -> Option<&dyn Storage> {
        match container_type {
            ContainerType::HousingInteriorPlacedItems1 => Some(&self.interior[0]),
            ContainerType::HousingInteriorPlacedItems2 => Some(&self.interior[1]),
            ContainerType::HousingInteriorPlacedItems3 => Some(&self.interior[2]),
            ContainerType::HousingInteriorPlacedItems4 => Some(&self.interior[3]),
            ContainerType::HousingInteriorPlacedItems5 => Some(&self.interior[4]),
            ContainerType::HousingInteriorPlacedItems6 => Some(&self.interior[5]),
            ContainerType::HousingInteriorPlacedItems7 => Some(&self.interior[6]),
            ContainerType::HousingInteriorPlacedItems8 => Some(&self.interior[7]),

            ContainerType::HousingInteriorStoreroom1 => Some(&self.interior_storeroom[0]),
            ContainerType::HousingInteriorStoreroom2 => Some(&self.interior_storeroom[1]),
            ContainerType::HousingInteriorStoreroom3 => Some(&self.interior_storeroom[2]),
            ContainerType::HousingInteriorStoreroom4 => Some(&self.interior_storeroom[3]),
            ContainerType::HousingInteriorStoreroom5 => Some(&self.interior_storeroom[4]),
            ContainerType::HousingInteriorStoreroom6 => Some(&self.interior_storeroom[5]),
            ContainerType::HousingInteriorStoreroom7 => Some(&self.interior_storeroom[6]),
            ContainerType::HousingInteriorStoreroom8 => Some(&self.interior_storeroom[7]),

            ContainerType::HousingInteriorAppearance => Some(&self.interior_appearance[0]),
            ContainerType::HousingExteriorAppearance => Some(&self.exterior_appearance[0]),

            ContainerType::HousingExteriorPlacedItems => Some(&self.exterior[0]),
            ContainerType::HousingExteriorStoreroom => Some(&self.exterior_storeroom[0]),
            _ => None,
        }
    }

    /// Helper functions to reduce boilerplate
    pub fn get_item_mut(
        &mut self,
        storage_id: ContainerType,
        storage_index: u16,
    ) -> Option<&mut Item> {
        if let Some(container) = self.get_container_mut(&storage_id) {
            return Some(container.get_slot_mut(storage_index));
        }

        None
    }

    pub fn get_item(&self, storage_id: ContainerType, storage_index: u16) -> Option<Item> {
        if let Some(container) = self.get_container(storage_id) {
            return Some(*container.get_slot(storage_index));
        }

        None
    }
}

/// Used to decide which set of housing inventory pages to send.
#[derive(Debug, Copy, Clone)]
pub enum DesiredHousingInventoryPages {
    None,
    Exterior,
    ExteriorStoreroom,
    Interior,
    InteriorStoreroom,
}
