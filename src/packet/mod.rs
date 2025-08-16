mod parsing;
pub use parsing::{
    ConnectionState, ConnectionType, PacketHeader, PacketSegment, SegmentData, SegmentType,
    parse_packet,
};

mod compression;
pub use compression::CompressionType;

mod encryption;
pub use encryption::generate_encryption_key;

mod ipc;
pub use ipc::{IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcOpcode, ReadWriteIpcSegment};

/// Bindings for Oodle network compression.
pub mod oodle;

/// Send packet helpers.
#[cfg(not(target_family = "wasm"))]
mod send_helpers;
#[cfg(not(target_family = "wasm"))]
pub use send_helpers::{send_custom_world_packet, send_keep_alive, send_packet};

mod scrambler;
pub use scrambler::{
    OBFUSCATION_ENABLED_MODE, ScramblerKeyGenerator, ScramblerKeys, scramble_packet,
};
