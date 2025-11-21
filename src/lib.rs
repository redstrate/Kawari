//! A server replacement for a certain MMO.

#![allow(clippy::large_enum_variant)]

use std::time::Duration;

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
#[cfg(all(not(target_family = "wasm"), feature = "server"))]
pub mod patch;

pub mod constants;
#[doc(hidden)]
mod opcodes;

/// IPC
pub mod ipc;

/// Inventory and storage management.
pub mod inventory;

/// The maximum size of our packet buffers, anything bigger than this from the client is truncated.
pub const RECEIVE_BUFFER_SIZE: usize = 0xFFFF;

/// The maximum durability of an item.
pub const ITEM_CONDITION_MAX: u16 = 30000;

/// The server's acknowledgement of a shop item being purchased.
pub const INVENTORY_ACTION_ACK_SHOP: u8 = 6;

/// The server's acknowledgement of the client modifying their inventory.
/// In the past, many more values were used according to Sapphire:
/// <https://github.com/SapphireServer/Sapphire/blob/044bff026c01b4cc3a37cbc9b0881fadca3fc477/src/common/Common.h#L83>
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
