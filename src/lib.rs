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

/// The size of various classjob arrays.
pub const CLASSJOB_ARRAY_SIZE: usize = 32;

/// The maximum durability of an item.
pub const ITEM_CONDITION_MAX: u16 = 30000;

// These operation codes/types change regularly, so update them when needed!

/// The operation opcode/type when discarding an item from the inventory.
pub const INVENTORY_ACTION_DISCARD: u8 = 145;

/// The operation opcode/type when moving an item to an emtpy slot in the inventory.
pub const INVENTORY_ACTION_MOVE: u8 = 146;

/// The operation opcode/type when moving an item to a slot occupied by another in the inventory.
pub const INVENTORY_ACTION_EXCHANGE: u8 = 147;

/// The operation opcode/type when splitting stacks of identical items.
pub const INVENTORY_ACTION_SPLIT_STACK: u8 = 148;

/// The operation opcode/type when combining stacks of identical items.
pub const INVENTORY_ACTION_COMBINE_STACK: u8 = 150;

/// The server's acknowledgement of a shop item being purchased.
pub const INVENTORY_ACTION_ACK_SHOP: u8 = 6;

/// The server's acknowledgement of the client modifying their inventory.
/// In the past, many more values were used according to Sapphire:
/// https://github.com/SapphireServer/Sapphire/blob/044bff026c01b4cc3a37cbc9b0881fadca3fc477/src/common/Common.h#L83
pub const INVENTORY_ACTION_ACK_GENERAL: u8 = 7;
