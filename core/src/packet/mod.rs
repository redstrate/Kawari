mod parsing;
pub use parsing::{
    ConnectionState, ConnectionType, PacketHeader, PacketSegment, SegmentData, SegmentType,
    parse_packet, parse_packet_header,
};

mod compression;
pub use compression::CompressionType;

mod encryption;
pub use encryption::generate_encryption_key;

mod ipc;
pub use ipc::{
    IPC_HEADER_SIZE, IpcSegment, IpcSegmentHeader, PredefinedOpcode, ReadWriteIpcOpcode,
    ReadWriteIpcSegment, ServerIpcSegmentHeader, ServerlessIpcSegmentHeader,
};

/// Bindings for Oodle network compression.
pub mod oodle;

/// Send packet helpers.
#[cfg(all(not(target_family = "wasm"), feature = "server"))]
mod send_helpers;
#[cfg(all(not(target_family = "wasm"), feature = "server"))]
pub use send_helpers::{send_custom_world_packet, send_keep_alive, send_packet};

mod scrambler;
pub use scrambler::{ScramblerKeyGenerator, ScramblerKeys, scramble_packet};
