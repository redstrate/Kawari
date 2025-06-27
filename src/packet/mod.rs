mod parsing;
pub use parsing::{
    ConnectionType, PacketHeader, PacketSegment, PacketState, SegmentData, SegmentType,
    parse_packet, send_keep_alive, send_packet,
};

mod compression;
pub use compression::CompressionType;

mod encryption;
pub use encryption::generate_encryption_key;

mod ipc;
pub use ipc::{IpcSegment, ReadWriteIpcSegment};

/// Bindings for Oodle network compression.
pub mod oodle;
