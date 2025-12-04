//! A server replacement for a certain MMO.

#![allow(clippy::large_enum_variant)]

/// The blowfish implementation used for packet encryption.
pub mod blowfish;

/// Common functions, structures used between all servers.
pub mod common;

/// Config management.
pub mod config;

/// Everything packet parsing related.
pub mod packet;

pub mod constants;
pub mod opcodes;

/// IPC
pub mod ipc;
