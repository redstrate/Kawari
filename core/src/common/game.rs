//! Useful game types should be kept here. Functions should probably not.

use binrw::binrw;
use bitflags::bitflags;
use strum_macros::{Display, EnumIter, FromRepr};

use crate::constants::BASE_INVENTORY_ACTION;

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

/// Maximum length of most (all?) chat messages.
pub const MESSAGE_MAX_LENGTH: usize = 1024;

/// The maximum durability of an item.
pub const ITEM_CONDITION_MAX: u16 = 30000;

/// The server's acknowledgement of a shop item being purchased.
pub const INVENTORY_ACTION_ACK_SHOP: u8 = 6;

/// The server's acknowledgement of the client modifying their inventory.
/// In the past, many more values were used according to Sapphire:
/// <https://github.com/SapphireServer/Sapphire/blob/044bff026c01b4cc3a37cbc9b0881fadca3fc477/src/common/Common.h#L83>
pub const INVENTORY_ACTION_ACK_GENERAL: u8 = 7;

pub struct Attributes {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
}

#[binrw]
#[brw(repr(u32))]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DistanceRange {
    Normal = 0x0,
    Extended = 0x1,
    Maximum = 0x2,
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EquipDisplayFlag(pub u16);

impl std::fmt::Debug for EquipDisplayFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

bitflags! {
    impl EquipDisplayFlag : u16 {
        const NONE = 0x00;
        const HIDE_LEGACY_MARK = 0x04;
        const HIDE_HEAD = 0x01;
        const HIDE_WEAPON = 0x02;
        const UNK1 = 0x04;
        const UNK2 = 0x08;
        const UNK3 = 0x10;
        const UNK4 = 0x20;
        const CLOSE_VISOR = 0x40;
        const HIDE_EARS = 0x80;
    }
}

impl Default for EquipDisplayFlag {
    fn default() -> Self {
        Self::NONE
    }
}

/// The client sends this to inform the server (and other clients) about the animation its player is performing while moving.
/// Multiple can be set at once, e.g. Strafing and walking at the same time.
// TODO: Why does RUNNING display as a comma in PacketAnalyzer?
#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct MoveAnimationType(u8);

bitflags! {
    impl MoveAnimationType : u8 {
        /// The player is running.
        const RUNNING = 0x00;
        /// Unknown: seems to be the start of the regular run animation and loops the first few frames endlessly.
        const UNKNOWN = 0x01;
        /// The player is walking or landing from a jump/fall (MoveAnimationState::ENTER_COLLISION is set).
        const WALKING_OR_LANDING = 0x02;
        /// The player is strafing.
        const STRAFING = 0x04;
        /// The player is being knocked back by an attack or some other force.
        const KNOCKBACK = 0x08;
        /// The player is jumping.
        const JUMPING = 0x10;
        /// The player has begun falling after jumping.
        const FALLING = 0x20;
    }
}

impl Default for MoveAnimationType {
    fn default() -> Self {
        Self::RUNNING
    }
}

impl std::fmt::Debug for MoveAnimationType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

/// The client sends this to inform the server about its player's current state when moving around.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveAnimationState {
    #[default]
    /// No special state is in play.
    None = 0,
    /// The player fell off something, or they began jumping.
    LeavingCollision = 1,
    /// The player landed back on the ground.
    EnteringCollision = 2,
    /// The player reached the apex of their jump, and began to fall.
    StartFalling = 4,
}

/// The client sends this to inform the server about its player's current state when jumping.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JumpState {
    /// The player is descending back to the ground, or isn't jumping at all.
    #[default]
    NoneOrFalling = 0,
    /// The player is ascending to the apex of the jump.
    Ascending = 16,
}

/// The server responds with these values to set the correct speed when informing other clients about how quickly to animate the movements.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MoveAnimationSpeed {
    Walking = 20,
    #[default]
    Running = 60,
    Jogging = 72,
    Sprinting = 78,
}

/// This allows us (and probably the client as well) to determine which event belongs to each sheet, or type of NPC.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Display, EnumIter, FromRepr)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum EventHandlerType {
    /// See Quests Excel sheet.
    Quests = 1,
    /// See Warp Excel sheet.
    Warp = 2,
    /// See GilShop Excel sheet.
    GilShop = 4,
    /// See Aetheryte Excel sheet.
    Aetheryte = 5,
    /// See GuildleveAssignment Excel sheet.
    GuildLeveAssignment = 6,
    /// See DefaultTalk Excel sheet.
    DefaultTalk = 9,
    /// See CustomTalk Excel sheet.
    CustomTalk = 11,
    /// See CraftLeve Excel sheet.
    CraftLevel = 14,
    /// See ChocoboTaxiStand Excel sheet.
    ChocoboTaxiStand = 18,
    /// See Opening Excel sheet.
    Opening = 19,
    /// Used for housing.
    ExitRange = 20,
    /// See GCShop Excel sheet.
    GcShop = 22,
    /// See GuildOrderGuide Excel sheet.
    GuildOrderGuide = 23,
    /// See GuildOrderOfficer Excel sheet.
    GuildOrderOfficer = 24,
    /// See ContentNpc Excel sheet.
    ContentNpc = 25,
    /// See Story Excel sheet.
    Story = 26,
    /// See SpecialShop Excel sheet.
    SpecialShop = 27,
    // See SwitchTalk Excel sheet.
    SwitchTalk = 31,
    /// See TripleTriad Excel sheet.
    TripleTriad = 35,
    /// See GoldSaucerArcadeMachine Excel sheet.
    GoldSaucerArcadeMachine = 36,
    /// See FccShop Excel sheet.
    FccShop = 42,
    /// See DpsChallengeOfficer Excel sheet.
    DpsChallengeOfficer = 47,
    /// See TopicSelect Excel sheet.
    TopicSelect = 50,
    /// See LotteryExchangeShop Excel sheet.
    LotteryExchangeShop = 52,
    /// See DisposalShop Excel sheet.
    DisposalShop = 53,
    /// See PreHandler Excel sheet.
    PreHandler = 54,
    /// Unknown, but seen when talking to the Frontline Attendant.
    UnkPVP = 55,
    /// See InclusionShop Excel sheet.
    InclusionShop = 58,
    /// See CollectablesShop Excel sheet.
    CollectablesShop = 59,
    /// See EventPathMove Excel sheet.
    EventPathMove = 61,
    /// These are used for the Solution Nine teleporter pads, for example. See EventGimmickPathMove Excel sheet.
    EventGimmickPathMove = 64,
}

#[cfg(feature = "server")]
impl mlua::IntoLua for EventHandlerType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

impl TryFrom<u32> for EventHandlerType {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::from_repr(value).ok_or(())
    }
}

/// Which language the client indicates as its primary language.
/// Not to be confused with physis::common::Language.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum ClientLanguage {
    #[default]
    Japanese = 0,
    English = 1,
    German = 2,
    French = 3,
}

// When adding a new container type, make sure to add it to InventoryIterator
#[binrw]
#[brw(little)]
#[brw(repr = u16)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ContainerType {
    #[default]
    Inventory0 = 0,
    Inventory1 = 1,
    Inventory2 = 2,
    Inventory3 = 3,

    Equipped = 1000,

    Currency = 2000,
    Crystals = 2001,
    MailEdit = 2002,
    Mail = 2003,
    KeyItems = 2004,
    HandIn = 2005,
    Unk1 = 2006,
    BlockedItems = 2007,
    Unk2 = 2008,
    Examine = 2009,
    Reclaim = 2010,
    HousingExteriorAppearanceEdit = 2011,
    HousingInteriorAppearanceEdit = 2012,
    ReconstructionBuyback = 2013,

    ArmoryOffWeapon = 3200,
    ArmoryHead = 3201,
    ArmoryBody = 3202,
    ArmoryHand = 3203,
    ArmoryWaist = 3204,
    ArmoryLeg = 3205,
    ArmoryFoot = 3206,
    ArmoryEarring = 3207,
    ArmoryNeck = 3208,
    ArmoryWrist = 3209,
    ArmoryRing = 3300,
    ArmorySoulCrystal = 3400,
    ArmoryWeapon = 3500,

    SaddleBag1 = 4000,
    SaddleBag2 = 4001,
    PremiumSaddleBag1 = 4100,
    PremiumSaddleBag2 = 4101,

    Cosmopouch1 = 5000,
    Cosmopouch2 = 5001,

    Invalid = 9999,

    RetainerPage1 = 10000,
    RetainerPage2 = 10001,
    RetainerPage3 = 10002,
    RetainerPage4 = 10003,
    RetainerPage5 = 10004,
    RetainerPage6 = 10005,
    RetainerPage7 = 10006,
    RetainerEquippedItems = 11000,
    RetainerGil = 12000,
    RetainerCrystals = 12001,
    RetainerMarket = 12002,

    FreeCompanyPage1 = 20000,
    FreeCompanyPage2 = 20001,
    FreeCompanyPage3 = 20002,
    FreeCompanyPage4 = 20003,
    FreeCompanyPage5 = 20004,
    FreeCompanyGil = 22000,
    FreeCompanyCrystals = 22001,

    HousingExteriorAppearance = 25000,
    HousingExteriorPlacedItems = 25001,
    HousingInteriorAppearance = 25002,
    HousingInteriorPlacedItems1 = 25003,
    HousingInteriorPlacedItems2 = 25004,
    HousingInteriorPlacedItems3 = 25005,
    HousingInteriorPlacedItems4 = 25006,
    HousingInteriorPlacedItems5 = 25007,
    HousingInteriorPlacedItems6 = 25008,
    HousingInteriorPlacedItems7 = 25009,
    HousingInteriorPlacedItems8 = 25010,

    HousingExteriorStoreroom = 27000,
    HousingInteriorStoreroom1 = 27001,
    HousingInteriorStoreroom2 = 27002,
    HousingInteriorStoreroom3 = 27003,
    HousingInteriorStoreroom4 = 27004,
    HousingInteriorStoreroom5 = 27005,
    HousingInteriorStoreroom6 = 27006,
    HousingInteriorStoreroom7 = 27007,
    HousingInteriorStoreroom8 = 27008,

    DiscardingItemSentinel = 65535,
}

#[binrw]
#[derive(Debug, Clone, Default, Copy, PartialEq)]
#[brw(repr = u32)]
#[repr(u32)]
pub enum ItemOperationKind {
    /// The operation opcode/type when the server wants the client to create a storage. Seen during login to create the HandIn storage.
    CreateStorage = BASE_INVENTORY_ACTION,
    /// The operation opcode/type when an item is created in the inventory. Seen during currency shop transactions, and likely elsewhere.
    Create = BASE_INVENTORY_ACTION + 4,
    /// The operation opcode/type when updating the inventory. Seen during all shop transactions when updating currency, and elsewhere.
    Update = BASE_INVENTORY_ACTION + 5,
    /// The operation opcode/type when discarding an item from the inventory. Seen when discarding an item from the inventory, and during gilshop sell transactions.
    Discard = BASE_INVENTORY_ACTION + 6,
    #[default]
    /// The operation opcode/type when moving an item to an emtpy slot in the inventory.
    Move = BASE_INVENTORY_ACTION + 7,
    /// The operation opcode/type when moving an item to a slot occupied by another in the inventory.
    Exchange = BASE_INVENTORY_ACTION + 8,
    /// The operation opcode/type when splitting stacks of identical items.
    SplitStack = BASE_INVENTORY_ACTION + 9,
    /// The operation opcode/type when combining stacks of identical items.
    CombineStack = BASE_INVENTORY_ACTION + 11,
    /// The operation opcode/type when the server wants the client to equip the mannequin (character sheet/try on preview model?). Seen during login and probably elsewhere.
    EquipMannequin = BASE_INVENTORY_ACTION + 18,
}

impl TryFrom<u32> for ItemOperationKind {
    type Error = ();
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            x if x == ItemOperationKind::Discard as u32 => Ok(ItemOperationKind::Discard),
            x if x == ItemOperationKind::Move as u32 => Ok(ItemOperationKind::Move),
            x if x == ItemOperationKind::Exchange as u32 => Ok(ItemOperationKind::Exchange),
            x if x == ItemOperationKind::SplitStack as u32 => Ok(ItemOperationKind::SplitStack),
            x if x == ItemOperationKind::CombineStack as u32 => Ok(ItemOperationKind::CombineStack),
            _ => Err(()),
        }
    }
}

// TODO: Where should this be moved to...?
#[repr(u32)]
pub enum LogMessageType {
    ItemBought = 0x697,
    ItemSold = 0x698,
    ItemBoughtBack = 0x699,
}
