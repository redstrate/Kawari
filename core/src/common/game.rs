//! Useful game types should be kept here. Functions should probably not.

use std::{collections::HashMap, time::Duration};

use binrw::binrw;
use bitflags::bitflags;
use strum_macros::{Display, EnumIter, FromRepr};

use crate::constants::BASE_INVENTORY_ACTION;

/// Maxmimum length of a character's name.
pub const CHAR_NAME_MAX_LENGTH: usize = 32;

/// Maximum length of most (all?) chat messages.
pub const MESSAGE_MAX_LENGTH: usize = 1024;

/// Maximum spawnable amount of actors in the client. Going over this limit crashes the game.
/// This is 99 - which is a weird number - except that the player always takes the first index.
pub const MAX_SPAWNED_ACTORS: usize = 99;

/// Maximum spawnable amount of objects in the client. Going over this limit crashes the game.
pub const MAX_SPAWNED_OBJECTS: usize = 40;

/// The maximum durability of an item.
pub const ITEM_CONDITION_MAX: u16 = 30000;

/// The server's acknowledgement of a shop item being purchased.
pub const INVENTORY_ACTION_ACK_SHOP: u8 = 6;

/// The server's acknowledgement of the client modifying their inventory.
/// In the past, many more values were used according to Sapphire:
/// <https://github.com/SapphireServer/Sapphire/blob/044bff026c01b4cc3a37cbc9b0881fadca3fc477/src/common/Common.h#L83>
pub const INVENTORY_ACTION_ACK_GENERAL: u8 = 7;

/// EObj ID for the "entrance circle" in instanced content.
pub const EOBJ_ENTRANCE_CIRCLE: u32 = 2000182;

/// EObj ID for the "shortcut" in instanced content.
pub const EOBJ_SHORTCUT: u32 = 2000700;

/// EObj ID for the "shortcut" used for explorer mode.
pub const EOBJ_SHORTCUT_EXPLORER_MODE: u32 = 2011343;

/// Time until a dead actor fades away. Estimated from retail.
pub const DEAD_FADE_OUT_TIME: Duration = Duration::from_secs(8);

/// Time until a dead actor despawns after fading away. Estimated from retail.
pub const DEAD_DESPAWN_TIME: Duration = Duration::from_secs(2);

pub struct Attributes {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
}

#[binrw]
#[brw(repr(u32))]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DistanceRange {
    Normal = 0x0,
    Extended = 0x1,
    Maximum = 0x2,
}

// TODO: it would be nice to figure out how these are actually calculated
// From a quick inspection, these are based on zone and increase later the expansion. These are probably encoded in some data somewhere.
pub const DISTANCE_NORMAL: f32 = 100.0;
pub const DISTANCE_EXTENDED: f32 = 300.0;
pub const DISTANCE_MAXIMUM: f32 = 500.0;

/// Gets the distance given a `DistanceRange`.
pub fn get_distance_range(range: DistanceRange) -> f32 {
    match range {
        DistanceRange::Normal => DISTANCE_NORMAL,
        DistanceRange::Extended => DISTANCE_EXTENDED,
        DistanceRange::Maximum => DISTANCE_MAXIMUM,
    }
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
    /// See Quest Excel sheet.
    Quest = 1,
    /// See Warp Excel sheet.
    Warp = 2,
    /// Unknown purpose.
    GatheringPoint = 3,
    /// See GilShop Excel sheet.
    Shop = 4,
    /// See Aetheryte Excel sheet (event_id & 0xFFF.)
    Aetheryte = 5,
    /// See GuildleveAssignment Excel sheet.
    GuildLeveAssignment = 6,
    /// See DefaultTalk Excel sheet.
    DefaultTalk = 9,
    /// Unknown purpose.
    Craft = 10,
    /// See CustomTalk Excel sheet.
    CustomTalk = 11,
    /// Unknown purpose.
    CompanyLeveOfficer = 12,
    /// See ArrayEventHandler sheet.
    Array = 13,
    /// See CraftLeve Excel sheet.
    CraftLeveClient = 14,
    /// Unknown purpose.
    GimmickAccessor = 15,
    /// See GimmickBill Excel sheet (event_id & 0xFFF.)
    GimmickBill = 16,
    /// See GimmickRect Excel sheet (event_id & 0xFFF.)
    GimmickRect = 17,
    /// See ChocoboTaxiStand Excel sheet.
    ChocoboTaxiStand = 18,
    /// See Opening Excel sheet.
    Opening = 19,
    /// Used for housing?
    ExitRange = 20,
    /// Unknown purpose.
    Fishing = 21,
    /// See GCShop Excel sheet.
    GrandCompanyShop = 22,
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
    /// Unknown purpose.
    DeepDungeon = 28,
    /// See InstanceContentGuide sheet.
    InstanceContentGuide = 29,
    /// See HousingAethernet sheet.
    HousingAethernet = 30,
    // See SwitchTalk Excel sheet.
    SwitchTalk = 31,
    /// Unknown purpose.
    MobHunt = 32,
    /// See Adventure Excel sheet.
    Adventure = 33,
    /// Unknown purpose.
    DailyQuestSupply = 34,
    /// See TripleTriad Excel sheet.
    TripleTriad = 35,
    /// See GoldSaucerArcadeMachine Excel sheet.
    GoldSaucerArcadeMachine = 36,
    /// Unknown purpose.
    LotteryDaily = 37,
    /// Unknown purpose.
    LotteryWeekly = 38,
    /// Unknown purpose.
    RaceChocoboRegistrar = 39,
    /// Unknown purpose.
    GoldSaucerTalk = 41,
    /// See FccShop Excel sheet.
    FreeCompanyCreditShop = 42,
    /// See AetherCurrent Excel sheet.
    AetherCurrent = 43,
    /// See ContentEntry Excel sheet.
    ContentEntry = 44,
    /// Unknown purpose.
    Verminion = 45,
    /// Unknown purpose.
    SkyIslandEntrance = 46,
    /// See DpsChallengeOfficer Excel sheet.
    DpsChallengeOfficer = 47,
    /// Unknown purpose.
    BeginnerTrainingOfficer = 48,
    /// Unknown purpose.
    RetainerBuyback = 49,
    /// See TopicSelect Excel sheet.
    TopicSelect = 50,
    /// See LotteryExchangeShop Excel sheet.
    LotteryExchangeShop = 52,
    /// See DisposalShop Excel sheet.
    DisposalShop = 53,
    /// See PreHandler Excel sheet.
    PreHandler = 54,
    /// See Description Excel sheet.
    Description = 55,
    /// Unknown purpose.
    HwdDev = 56,
    /// Unknown purpose.
    Materialize = 57,
    /// See InclusionShop Excel sheet.
    InclusionShop = 58,
    /// See CollectablesShop Excel sheet.
    CollectablesShop = 59,
    /// Unknown purpose.
    MJIPasture = 60,
    /// See EventPathMove Excel sheet.
    EventPathMove = 61,
    /// Unknown purpose.
    ReactionEvent = 62,
    /// Used for the Solution Nine teleporter pads, for example. See EventGimmickPathMove Excel sheet.
    EventGimmickPathMove = 64,
    /// See EventMountGimmickPathMove Excel sheet.
    EventMountGimmickPathMove = 65,
}

#[cfg(feature = "server")]
impl mlua::IntoLua for EventHandlerType {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
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
#[derive(Debug, Clone, Default, Copy, PartialEq, FromRepr)]
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

// TODO: Where should this be moved to...?
#[repr(u32)]
pub enum LogMessageType {
    ItemBought = 0x697,
    ItemSold = 0x698,
    ItemBoughtBack = 0x699,
}

/// Names for rows in the Excel sheet of the same name.
/// See <https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Enums/TerritoryIntendedUse.cs>.
#[repr(u8)]
#[derive(FromRepr)]
pub enum TerritoryIntendedUse {
    /// Towns such as Limsa Lominsa.
    Town = 0,
    /// Open world zones such as everything out of towns.
    OpenWorld = 1,
    /// Inn rooms.
    Inn = 2,
    /// Dungeon zones and other misc duties like Air Force One.
    Dungeon = 3,
    /// Jail zones like Mordion Gaol.
    Jail = 5,
    /// Copies of Towns that are only during the opening.
    OpeningArea = 6,
    /// Rarely seen "lobby zones", such as the Phantom Village for Occult Crescent.
    LobbyArea = 7,
    /// Zones used in Alliance Raids.
    AllianceRaid = 8,
    /// Used for (pre-Endwalker?) quest battles.
    OpenWorldInstanceBattle = 9,
    /// Trial battles.
    Trial = 10,
    Unk100 = 11,
    Unk110 = 12,
    HousingOutdoor = 13,
    HousingIndoor = 14,
    SoloOverworldInstance = 15,
    /// Fighting arenas for raids like.
    Raid1 = 16,
    /// Ditto?
    Raid2 = 17,
    /// Zones used for Frontline PvP.
    Frontline = 18,
    Unk120 = 19,
    ChocoboRacing = 20,
    /// Used for the only Ishgard Restoration zone, the Firamament.
    IshgardRestoration = 21,
    /// The Sanctum of the Twelve zone used for weddings.
    Wedding = 22,
    /// Gold Saucer zones.
    GoldSaucer = 23,
    /// ???
    ExploratoryMissions = 26,
    /// Used for the Hall of Novice tutorials.
    HallOfTheNovice = 27,
    /// Zones used for Crystalline Conflict PvP.
    CrystallineConflict = 28,
    /// Used for events like Solo Duties.
    SoloDuty = 29,
    /// The barracks zones of grand companies.
    FreeCompanyGarrison = 30,
    /// Zones used for Deep Dugeons, e.g. Palace of the Dead.
    DeepDungeon = 31,
    /// Used for zones only accessible seasonally, like Starlight Halls.
    Seasonal = 32,
    /// Treasure dungeons like Vault Oneiron.
    TreasureDungeon = 33,
    /// ???
    SeasonalInstancedArea = 34,
    /// ???
    TripleTriadBattleHall = 35,
    /// Used for raids like The Cloud of Darkness (Chaotic).
    ChaoticRaid = 36,
    /// ???
    CrystallineConflictCustomMatch = 37,
    /// Also used for Starlight Halls(?)
    PrivateEventArea = 40,
    /// Eureka zones.
    Eureka = 41,
    Unk2 = 42,
    Unk3 = 43,
    /// Leap of Faith zones.
    LeapOfFaith = 44,
    /// ???
    MaskedCarnival = 45,
    /// Zones used for Ocean Fishing.
    OceanFishing = 46,
    Unk7 = 47,
    Unk8 = 48,
    /// Island Sanctuary zones.
    IslandSanctuary = 49,
    Unk10 = 50,
    Unk11 = 51,
    Unk12 = 52,
    Unk13 = 53,
    Unk14 = 54,
    Unk15 = 55,
    Elysion = 56,
    /// Criterion Dungeons zones.
    CriterionDungeon = 57,
    /// Savage Criterion Dungeons zones.
    SavageCriterionDungeon = 58,
    /// Bean containment zones.
    Blunderville = 59,
    /// Cosmic Exploration zones.
    CosmicExploration = 60,
    /// Occult Crescent zones.
    OccultCrescent = 61,
    Unk22 = 62,
}

// From FFXIVClientStructs
// This is actually indexes of InstanceContentType, but we want nice names.
#[derive(Debug, FromRepr)]
#[repr(u8)]
pub enum InstanceContentType {
    Raid = 1,
    Dungeon = 2,
    Guildhests = 3,
    Trial = 4,
    CrystallineConflict = 5,
    Frontlines = 6,
    QuestBattle = 7,
    BeginnerTraining = 8,
    DeepDungeon = 9,
    TreasureHuntDungeon = 10,
    SeasonalDungeon = 11,
    RivalWing = 12,
    MaskedCarnivale = 13,
    Mahjong = 14,
    GoldSaucer = 15,
    OceanFishing = 16,
    UnrealTrial = 17,
    TripleTriad = 18,
    VariantDungeon = 19,
    CriterionDungeon = 20,
}

// TODO: see if this can be extrapolated from game data
const AETHER_CURRENT_COMP_FLG_SET_TO_SCREENIMAGE: [(u32, u32); 31] = [
    // HW
    (1, 328), // Coerthas Western Highlands
    (2, 329), // The Dravanian Forelands
    (3, 330), // The Dravanian Hinterlands
    (4, 331), // The Churning Mists
    (5, 332), // The Sea of Clouds
    (6, 333), // Azys Lla
    // StB
    (7, 511),  // The Fringes
    (8, 514),  // The Ruby Sea
    (9, 512),  // The Peaks
    (10, 515), // Yanxia
    (11, 513), // The Lochs
    (12, 516), // The Azim Steppe
    // ShB
    (13, 762), // Lakeland
    (14, 763), // Amh Araeng
    (15, 764), // Il Mheg
    (16, 765), // Kholusia
    (17, 766), // The Rak'tika Greatwood
    (18, 767), // The Tempest
    // TODO: maybe Mor Dhona's ScreenImage is the "Flying Unlocked" seen at the end of "The Ultimate Weapon" (end of ARR MSQ)? Need a confirmation.
    (19, 0), // Mor Dhona
    // EW
    (20, 1016), // Labyrinthos
    (21, 1017), // Thavnair
    (22, 1018), // Garlemald
    (23, 1019), // Mare Lamentorum
    (24, 1021), // Elpis
    (25, 1020), // Ultima Thule
    // DT
    (26, 1269), // Urqopacha
    (27, 1270), // Kozama'uka
    (28, 1271), // Yak T'el
    (29, 1272), // Shaaloani
    (30, 1273), // Heritage Found
    (31, 1274), // Living Memory
];

pub fn get_aether_current_comp_flg_set_to_screenimage() -> HashMap<u32, u32> {
    HashMap::from(AETHER_CURRENT_COMP_FLG_SET_TO_SCREENIMAGE)
}

#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct InvisibilityFlags(u8);

bitflags! {
    impl InvisibilityFlags: u8 {
        const VISIBLE = 0;
        const UNK1 = 1;
        const UNK2 = 2;
        const UNK3 = 4;
    }
}

impl Default for InvisibilityFlags {
    fn default() -> Self {
        InvisibilityFlags::VISIBLE
    }
}

impl std::fmt::Debug for InvisibilityFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

// TODO: should we include the param here?
#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[derive(Debug, Clone, Default, PartialEq)]
pub enum CharacterMode {
    /// Has no effect, never used.
    None = 0,
    /// Normal behavior. Param is always 0.
    #[default]
    Normal = 1,
    /// Changes the nameplate color, and shows the return message. Param unknown.
    Dead = 2,
    /// Currently looping an emote. Param is the index into the EmoteMode Excel sheet.
    EmoteLoop = 3,
    /// Unknown purpose. Param is always 0.
    Mounted = 4,
    /// Currently crafting, blocks certain actions. Param is always 0.
    Crafting = 5,
    /// Currently gathering, blocks certain actions. Param unknown.
    Gathering = 6,
    /// Unknown purpose. Param unknown.
    MateriaMelding = 7,
    /// Unknown purpose. Param unknown.
    AnimationLock = 8,
    /// Currently carrying an object. Param is a index into the Carry Excel sheet.
    Carrying = 9,
    /// Riding in someone else's mount. Param is the seat number.
    RidingPillion = 10,
    /// Unknown purpose. Param is a index into the EmoteMode sheet.
    InPositionLoop = 11,
    /// Unknown purpose. Param unknown.
    RaceChocobo = 12,
    /// Displays the "playing Triple Triad" animation. Param unknown.
    TripleTriad = 13,
    /// Unknown purpose. Param unknown.
    LordOfVerminion = 14,
    /// Unknown purpose. Param unknown. But it makes your character disappear?!
    Unknown1 = 15,
    /// Playing an instrument. Param is a index into the Perform Excel sheet.
    Performance = 16,
}
