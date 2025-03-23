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

impl Inventory {
    pub fn new() -> Self {
        Self {
            equipped: EquippedContainer::default(),
        }
    }

    /// Equip the starting items for a given race
    pub fn equip_racial_items(&mut self, race_id: u8) {
        // TODO: don't hardcode
        self.equipped.main_hand = Item::new(1, 0x00000641);
        self.equipped.body = Item::new(1, 0x00000ba8);
        self.equipped.hands = Item::new(1, 0x00000dc1);
        self.equipped.legs = Item::new(1, 0x00000ce1);
        self.equipped.feet = Item::new(1, 0x00000ea7);
        self.equipped.ears = Item::new(1, 0x00003b1b);
        self.equipped.neck = Item::new(1, 0x00003b1a);
        self.equipped.wrists = Item::new(1, 0x00003b1c);
        self.equipped.right_ring = Item::new(1, 0x0000114a);
        self.equipped.left_ring = Item::new(1, 0x00003b1d);
    }
}
