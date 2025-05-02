use std::{fs::write, io::Cursor};

use binrw::{BinRead, BinWrite, binrw};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{
    common::{custom_ipc::CustomIpcSegment, read_string, timestamp_msecs, write_string},
    config::get_config,
    oodle::OodleNetwork,
    packet::{compression::compress, encryption::decrypt},
};

use super::{
    CompressionType, compression::decompress, encryption::encrypt, ipc::ReadWriteIpcSegment,
};

#[binrw]
#[brw(repr = u16)]
#[derive(Debug, PartialEq)]
pub enum ConnectionType {
    None = 0x0,
    Zone = 0x1,
    Chat = 0x2,
    Lobby = 0x3,
}

#[binrw]
#[brw(import(size: u32, encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
pub enum SegmentType<T: ReadWriteIpcSegment> {
    // Client->Server Packets
    #[brw(magic = 0x1u32)]
    InitializeSession {
        #[brw(pad_before = 4)] // empty
        #[brw(pad_size_to = 36)]
        #[br(count = 36)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        actor_id: String, // square enix in their infinite wisdom has this as a STRING REPRESENTATION of an integer. what
    },
    #[brw(magic = 0x9u32)]
    InitializeEncryption {
        #[brw(pad_before = 36)] // empty
        #[brw(pad_size_to = 32)]
        #[br(count = 32)]
        #[br(map = read_string)]
        #[bw(ignore)]
        phrase: String,

        #[brw(pad_before = 32)]
        #[brw(pad_after = 512)] // empty
        key: [u8; 4],
    },
    #[brw(magic = 0x3u32)]
    Ipc {
        #[br(parse_with = decrypt, args(size, encryption_key))]
        #[bw(write_with = encrypt, args(size, encryption_key))]
        data: T,
    },
    #[brw(magic = 0x7u32)]
    KeepAlive { id: u32, timestamp: u32 },

    // Server->Client Packets
    #[brw(magic = 0xAu32)]
    InitializationEncryptionResponse {
        #[br(count = 0x280)]
        #[brw(pad_size_to = 640)]
        data: Vec<u8>,
    },
    #[brw(magic = 0x8u32)]
    KeepAliveResponse { id: u32, timestamp: u32 },
    #[brw(magic = 0x2u32)]
    ZoneInitialize {
        player_id: u32,
        #[brw(pad_after = 32)]
        timestamp: u32,
    },

    // Custom Packets
    #[brw(magic = 0xAAAAu32)]
    CustomIpc { data: CustomIpcSegment },
}

#[binrw]
#[derive(Debug)]
pub struct PacketHeader {
    pub unk1: u64,
    pub unk2: u64,
    pub timestamp: u64,
    pub size: u32,
    pub connection_type: ConnectionType,
    pub segment_count: u16,
    pub version: u8, // it's just called this, but unsure what this is actually for?
    pub compression_type: CompressionType,
    pub unk4: u16,
    pub uncompressed_size: u32,
}

#[binrw]
#[brw(import(encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
pub struct PacketSegment<T: ReadWriteIpcSegment> {
    #[bw(calc = self.calc_size())]
    pub size: u32,
    pub source_actor: u32,
    pub target_actor: u32,
    #[brw(args(size, encryption_key))]
    pub segment_type: SegmentType<T>,
}

impl<T: ReadWriteIpcSegment> PacketSegment<T> {
    pub fn calc_size(&self) -> u32 {
        let header = std::mem::size_of::<u32>() * 4;
        header as u32
            + match &self.segment_type {
                SegmentType::InitializeEncryption { .. } => 616,
                SegmentType::InitializationEncryptionResponse { .. } => 640,
                SegmentType::Ipc { data } => data.calc_size(),
                SegmentType::KeepAlive { .. } => 0x8,
                SegmentType::KeepAliveResponse { .. } => 0x8,
                SegmentType::ZoneInitialize { .. } => 40,
                SegmentType::InitializeSession { .. } => 40,
                SegmentType::CustomIpc { data } => data.calc_size(),
            }
    }
}

#[binrw]
#[brw(import(oodle: &mut OodleNetwork, encryption_key: Option<&[u8]>))]
#[derive(Debug)]
struct Packet<T: ReadWriteIpcSegment> {
    header: PacketHeader,
    #[bw(args(encryption_key))]
    #[br(parse_with = decompress, args(oodle, &header, encryption_key,))]
    segments: Vec<PacketSegment<T>>,
}

fn dump(msg: &str, data: &[u8]) {
    write("packet.bin", data).unwrap();
    tracing::warn!("{msg} Dumped to packet.bin.");
}

pub async fn send_packet<T: ReadWriteIpcSegment>(
    socket: &mut TcpStream,
    state: &mut PacketState,
    connection_type: ConnectionType,
    compression_type: CompressionType,
    segments: &[PacketSegment<T>],
) {
    let (data, uncompressed_size) = compress(state, &compression_type, segments);
    let size = std::mem::size_of::<PacketHeader>() + data.len();

    let header = PacketHeader {
        unk1: 0xE2465DFF41A05252, // wtf?
        unk2: 0x75C4997B4D642A7F, // wtf? x2
        timestamp: timestamp_msecs(),
        size: size as u32,
        connection_type,
        segment_count: segments.len() as u16,
        version: 0,
        compression_type,
        unk4: 0,
        uncompressed_size: uncompressed_size as u32,
    };

    let mut cursor = Cursor::new(Vec::new());
    header.write_le(&mut cursor).unwrap();
    std::io::Write::write_all(&mut cursor, &data).unwrap();

    let buffer = cursor.into_inner();

    socket.write_all(&buffer).await.unwrap();
}

// temporary
/// State needed for each connection between the client & server, containing various things like the compressor and encryption keys.
pub struct PacketState {
    pub client_key: Option<[u8; 16]>,
    pub serverbound_oodle: OodleNetwork,
    pub clientbound_oodle: OodleNetwork,
}

pub async fn parse_packet<T: ReadWriteIpcSegment>(
    data: &[u8],
    state: &mut PacketState,
) -> (Vec<PacketSegment<T>>, ConnectionType) {
    let mut cursor = Cursor::new(data);

    match Packet::read_le_args(
        &mut cursor,
        (
            &mut state.serverbound_oodle,
            state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),
        ),
    ) {
        Ok(packet) => (packet.segments, packet.header.connection_type),
        Err(err) => {
            tracing::error!("{err}");

            let config = get_config();
            if config.packet_debugging {
                dump("Failed to parse packet!", data);
            }

            (Vec::new(), ConnectionType::None)
        }
    }
}

pub async fn send_keep_alive<T: ReadWriteIpcSegment>(
    socket: &mut TcpStream,
    state: &mut PacketState,
    connection_type: ConnectionType,
    id: u32,
    timestamp: u32,
) {
    let response_packet: PacketSegment<T> = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::KeepAliveResponse { id, timestamp },
    };
    send_packet(
        socket,
        state,
        connection_type,
        CompressionType::Uncompressed,
        &[response_packet],
    )
    .await;
}

#[cfg(test)]
mod tests {
    use crate::packet::IpcSegment;

    use super::*;

    /// Ensure that the packet size as reported matches up with what we write
    #[test]
    fn test_packet_sizes() {
        #[binrw]
        #[brw(repr = u16)]
        #[derive(Clone, PartialEq, Debug)]
        enum ClientLobbyIpcType {
            Dummy = 0x1,
        }

        #[binrw]
        #[br(import(_magic: &ClientLobbyIpcType))]
        #[derive(Debug, Clone)]
        enum ClientLobbyIpcData {
            Dummy(),
        }

        type ClientLobbyIpcSegment = IpcSegment<ClientLobbyIpcType, ClientLobbyIpcData>;

        impl ReadWriteIpcSegment for ClientLobbyIpcSegment {
            fn calc_size(&self) -> u32 {
                todo!()
            }
        }

        let packet_types = [
            SegmentType::InitializeEncryption {
                phrase: String::new(),
                key: [0; 4],
            },
            SegmentType::InitializationEncryptionResponse { data: Vec::new() },
            SegmentType::KeepAlive {
                id: 0,
                timestamp: 0,
            },
            SegmentType::KeepAliveResponse {
                id: 0,
                timestamp: 0,
            },
        ];

        for packet in &packet_types {
            let mut cursor = Cursor::new(Vec::new());

            let packet_segment: PacketSegment<ClientLobbyIpcSegment> = PacketSegment {
                source_actor: 0,
                target_actor: 0,
                segment_type: packet.clone(),
            };
            packet_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(buffer.len(), packet_segment.calc_size() as usize);
        }
    }
}
