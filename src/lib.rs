//! A server replacement for a certain MMO.

#![allow(clippy::large_enum_variant)]

use std::{collections::HashMap, time::Duration};

use patch::Version;

/// The blowfish implementation used for packet encryption.
pub mod blowfish;

/// Common functions, structures used between all servers.
pub mod common;

/// Config management.
pub mod config;

/// Lobby server-specific code.
#[cfg(all(not(target_family = "wasm"), feature = "server"))]
pub mod lobby;

/// World server-specific code.
#[cfg(all(not(target_family = "wasm"), feature = "server"))]
pub mod world;

/// Everything packet parsing related.
pub mod packet;

/// Logic server-specific code.
#[cfg(all(not(target_family = "wasm"), feature = "server"))]
pub mod login;

/// Patch server-specific code.
pub mod patch;

/// Opcodes, see `resources/opcodes.json`
pub mod opcodes;

/// IPC
pub mod ipc;

/// Inventory and storage management.
pub mod inventory;

/// Used in the encryption key.
const GAME_VERSION: u32 = 7201;

/// The maximum size of our packet buffers, anything bigger than this from the client is truncated.
pub const RECEIVE_BUFFER_SIZE: usize = 0xFFFF;

/// Supported boot version.
pub const SUPPORTED_BOOT_VERSION: Version = Version("2025.07.17.0000.0001");

/// Supported game version.
pub const SUPPORTED_GAME_VERSION: Version = Version("2025.10.13.0000.0000");

/// The maximum supported expansion.
pub const MAX_EXPANSION: usize = 5;

const SUPPORTED_EXPAC_VERSIONS: [(&str, Version); MAX_EXPANSION] = [
    ("ex1", Version("2025.07.25.0000.0000")),
    ("ex2", Version("2025.07.23.0000.0000")),
    ("ex3", Version("2025.10.13.0000.0000")),
    ("ex4", Version("2025.10.13.0000.0000")),
    ("ex5", Version("2025.10.13.0000.0000")),
];

/// Supported expansion versions.
pub fn get_supported_expac_versions() -> HashMap<&'static str, Version<'static>> {
    HashMap::from(SUPPORTED_EXPAC_VERSIONS)
}

/// The size of the unlock bitmask.
pub const UNLOCK_BITMASK_SIZE: usize = 92;

/// The size of the aetheryte unlock bitmask.
// TODO: this can be automatically derived from game data
pub const AETHERYTE_UNLOCK_BITMASK_SIZE: usize = 30;

/// The size of the completed quest bitmask.
pub const COMPLETED_QUEST_BITMASK_SIZE: usize = 747;

/// The size of the unlocked title bitmask.
pub const TITLE_UNLOCK_BITMASK_SIZE: usize = 112;

/// The size of the completed levequest bitmask.
pub const COMPLETED_LEVEQUEST_BITMASK_SIZE: usize = 226;

/// The size of various classjob arrays.
pub const CLASSJOB_ARRAY_SIZE: usize = 32;

/// The size of various raid bitmasks.
pub const RAID_ARRAY_SIZE: usize = 28;

/// The size of various dungeon bitmasks.
pub const DUNGEON_ARRAY_SIZE: usize = 18;

/// The size of various guildhest bitmasks.
pub const GUILDHEST_ARRAY_SIZE: usize = 10;

/// The size of various trial bitmasks.
pub const TRIAL_ARRAY_SIZE: usize = 14;

/// The size of various PvP bitmasks.
pub const PVP_ARRAY_SIZE: usize = 7;

/// The size of the minion bitmask.
pub const MINION_BITMASK_SIZE: usize = 71;

/// The size of the mount bitmask.
pub const MOUNT_BITMASK_SIZE: usize = 41;

/// The size of the active help bitmask.
pub const ACTIVE_HELP_BITMASK_SIZE: usize = 37;

/// The size of the AetherCurrentCompFlgSet bitmask.
pub const AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE: usize = 4;

/// The size of the AetherCurrent bitmask.
pub const AETHER_CURRENT_BITMASK_SIZE: usize = 56;

/// The size of the Orchestrion bitmask.
pub const ORCHESTRION_ROLL_BITMASK_SIZE: usize = 99;

/// The size of the BuddyEquip (Companion Barding) bitmask.
pub const BUDDY_EQUIP_BITMASK_SIZE: usize = 14;

/// The size of the Cutscenes (that has been seen) bitmask.
pub const CUTSCENE_SEEN_BITMASK_SIZE: usize = 173;

/// The size of the Ornament (Accessories) bitmask.
pub const ORNAMENT_BITMASK_SIZE: usize = 7;

/// The size of the Fish caught bitmask.
pub const CAUGHT_FISH_BITMASK_SIZE: usize = 182;

/// The size of the Fish caught with Spearfishing bitmask.
pub const CAUGHT_SPEARFISH_BITMASK_SIZE: usize = 38;

/// The size of the Adventure (Sightseeing) bitmask.
pub const ADVENTURE_BITMASK_SIZE: usize = 43;

/// The size of the Triple Triad cards bitmask.
pub const TRIPLE_TRIAD_CARDS_BITMASK_SIZE: usize = 57;

/// The size of the GlassesStyle bitmask.
pub const GLASSES_STYLES_BITMASK_SIZE: usize = 5;

/// The size of the Chocobo Taxi Stands bitmask.
pub const CHOCOBO_TAXI_STANDS_BITMASK_SIZE: usize = 12;

/// The maximum durability of an item.
pub const ITEM_CONDITION_MAX: u16 = 30000;

// This operation code changes regularly, so update it when needed!
pub const BASE_INVENTORY_ACTION: u32 = 364;

/// The server's acknowledgement of a shop item being purchased.
pub const INVENTORY_ACTION_ACK_SHOP: u8 = 6;

/// The server's acknowledgement of the client modifying their inventory.
/// In the past, many more values were used according to Sapphire:
/// https://github.com/SapphireServer/Sapphire/blob/044bff026c01b4cc3a37cbc9b0881fadca3fc477/src/common/Common.h#L83
pub const INVENTORY_ACTION_ACK_GENERAL: u8 = 7;

// TODO: Where should this be moved to...?
#[repr(u32)]
pub enum LogMessageType {
    ItemBought = 0x697,
    ItemSold = 0x698,
    ItemBoughtBack = 0x699,
}

/// Error messages: TODO: this should probably be moved into its own universal mod/crate?
pub const ERR_INVENTORY_ADD_FAILED: &str =
    "Unable to add item to inventory! Your inventory is full, or this is a bug in Kawari!";

/// Service name for the account management pages. This is used to uniquely identify sessions.
pub const ACCOUNT_MANAGEMENT_SERVICE: &str = "Kawari: Account Management";

/// Service name for game logins. This is used to uniquely identify sessions.
pub const GAME_SERVICE: &str = "Kawari: Game Client";

/// Timeout in seconds before clients are disconnected because of idle network activity.
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(5);
