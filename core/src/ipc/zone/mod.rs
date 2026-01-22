//! ## Server Opcodes
#![doc = include_str!("../../doc_serverzoneipctype.md")]
//! ## Client Opcodes
#![doc = include_str!("../../doc_clientzoneipctype.md")]

mod client;
pub use client::*;

mod common_emote;

mod common_strategy_board;
pub use common_strategy_board::*;

mod common_waymark;
pub use common_waymark::*;

mod black_list;
mod config;
mod social_list;

mod online_status;
pub use online_status::*;

mod party_misc;
pub use party_misc::*;

mod server;
pub use server::*;

mod search_info;
pub use search_info::*;
