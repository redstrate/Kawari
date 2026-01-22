//! ## Server Opcodes
#![doc = include_str!("../../doc_serverlobbyipctype.md")]
//! ## Client Opcodes
#![doc = include_str!("../../doc_clientlobbyipctype.md")]

mod client;
pub use client::*;

mod server;
pub use server::*;

pub mod chara_make;
