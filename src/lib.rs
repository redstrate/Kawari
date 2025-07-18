//! A server replacement for a certain MMO.

#![allow(clippy::large_enum_variant)]

use std::collections::HashMap;

use patch::Version;

/// The blowfish implementation used for packet encryption.
pub mod blowfish;

/// Common functions, structures used between all servers.
pub mod common;

/// Config management.
pub mod config;

/// Lobby server-specific code.
#[cfg(not(target_family = "wasm"))]
pub mod lobby;

/// World server-specific code.
#[cfg(not(target_family = "wasm"))]
pub mod world;

/// Everything packet parsing related.
pub mod packet;

/// Logic server-specific code.
#[cfg(not(target_family = "wasm"))]
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
const GAME_VERSION: u16 = 7000;

pub const RECEIVE_BUFFER_SIZE: usize = 32000;

/// Supported boot version.
pub const SUPPORTED_BOOT_VERSION: Version = Version("2025.05.01.0000.0001");

/// Supported game version.
pub const SUPPORTED_GAME_VERSION: Version = Version("2025.06.28.0000.0000");

const SUPPORTED_EXPAC_VERSIONS: [(&str, Version); 5] = [
    ("ex1", Version("2025.05.01.0000.0000")),
    ("ex2", Version("2025.05.09.0000.0000")),
    ("ex3", Version("2025.06.28.0000.0000")),
    ("ex4", Version("2025.06.28.0000.0000")),
    ("ex5", Version("2025.06.28.0000.0000")),
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
pub const COMPLETED_QUEST_BITMASK_SIZE: usize = 691;

/// The size of the unlocked title bitmask.
pub const TITLE_UNLOCK_BITMASK_SIZE: usize = 112;

/// The size of the completed levequest bitmask.
pub const COMPLETED_LEVEQUEST_BITMASK_SIZE: usize = 226;

/// The size of various classjob arrays.
pub const CLASSJOB_ARRAY_SIZE: usize = 32;

/// The maximum durability of an item.
pub const ITEM_CONDITION_MAX: u16 = 30000;

// This operation code change regularly, so update it when needed!
pub const BASE_INVENTORY_ACTION: u8 = 144;

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
