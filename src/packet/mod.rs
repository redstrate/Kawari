mod parsing;
use parsing::PacketHeader;
pub use parsing::{
    ConnectionType, PacketSegment, PacketState, SegmentType, parse_packet, send_keep_alive,
    send_packet,
};

mod compression;
pub use compression::CompressionType;

mod encryption;
pub use encryption::generate_encryption_key;

mod ipc;
pub use ipc::{IpcSegment, ReadWriteIpcSegment};
