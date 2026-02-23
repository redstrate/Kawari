//! A server replacement for a certain MMO.

/// Common functions, structures used in parsing and also useful for the servers.
pub mod common;

/// Config management.
#[cfg(feature = "server")]
pub mod config;

/// Everything packet parsing related.
pub mod packet;

#[rustfmt::skip]
#[doc(hidden)]
pub mod constants;
#[rustfmt::skip]
#[doc(hidden)]
pub mod opcodes;

/// IPC
pub mod ipc;
