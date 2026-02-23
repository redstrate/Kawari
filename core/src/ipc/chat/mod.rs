//! ## Server Opcodes
#![doc = include_str!("../../doc_serverchatipctype.md")]
//! ## Client Opcodes
#![doc = include_str!("../../doc_clientchatipctype.md")]

mod server;
pub use server::*;

mod client;
pub use client::*;

mod common;
pub use common::*;
