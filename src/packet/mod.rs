mod parsing;
pub use parsing::{
    ConnectionType, PacketHeader, PacketSegment, PacketState, SegmentData, SegmentType,
    parse_packet,
};

mod compression;
pub use compression::CompressionType;

mod encryption;
pub use encryption::generate_encryption_key;

mod ipc;
pub use ipc::{IpcSegment, ReadWriteIpcSegment};

/// Bindings for Oodle network compression.
pub mod oodle;

/// Send packet helpers.
#[cfg(not(target_family = "wasm"))]
mod send_helpers;
#[cfg(not(target_family = "wasm"))]
pub use send_helpers::{send_keep_alive, send_packet};
