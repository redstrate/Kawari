use std::io::Cursor;

use binrw::{BinRead, binrw};

use crate::{
    common::{read_string, write_string},
    ipc::kawari::CustomIpcSegment,
    packet::encryption::decrypt,
};

use super::{
    CompressionType, ScramblerKeys, compression::decompress, encryption::encrypt,
    ipc::ReadWriteIpcSegment, oodle::OodleNetwork,
};

#[binrw]
#[brw(repr = u16)]
#[derive(Debug, PartialEq)]
pub enum ConnectionType {
    /// An invalid connection.
    None = 0x0,
    /// The zone connection.
    Zone = 0x1,
    /// The chat connection.
    Chat = 0x2,
    /// The lobby connection.
    Lobby = 0x3,
    /// A custom internal connection for use between Kawari servers.
    KawariIpc = 0xAAAA,
}

#[binrw]
#[brw(repr = u16)]
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum SegmentType {
    /// An invalid segment.
    #[default]
    None = 0x0,
    /// Used to tell the server to setup a connection.
    Setup = 0x1,
    /// Used to tell the client that the server has setup their connection and it's ready to use.
    Initialize = 0x2,
    /// Used for everything interesting, e.g. game actions. Has it's own completely separate opcodes and structure.
    Ipc = 0x3,
    /// Sent to begin keep alives and needs a KeepAliveResponse.
    KeepAliveRequest = 0x7,
    /// Sent to respond to KeepAliveRequest.
    KeepAliveResponse = 0x8,
    /// Sent to the server which gives it the required phrase and key to communicate with the client.
    SecuritySetup = 0x9,
    /// Sent to the client to initialize and confirm the encryption key.
    SecurityInitialize = 0xA,
    /// Segment used internally in Kawari for IPC.
    KawariIpc = 0xAAAA,
}

#[binrw]
#[brw(import(kind: SegmentType, size: u32, state: &ConnectionState))]
#[derive(Debug, Clone)]
pub enum SegmentData<T: ReadWriteIpcSegment> {
    #[br(pre_assert(kind == SegmentType::None))]
    None(),
    #[br(pre_assert(kind == SegmentType::Setup))]
    Setup {
        #[brw(pad_before = 4)] // empty
        #[brw(pad_size_to = 36)]
        #[br(count = 36)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        actor_id: String, // square enix in their infinite wisdom has this as a STRING REPRESENTATION of an integer. what
    },
    #[br(pre_assert(kind == SegmentType::Initialize))]
    Initialize {
        actor_id: u32,
        #[brw(pad_after = 32)]
        timestamp: u32,
    },
    #[br(pre_assert(kind == SegmentType::SecuritySetup))]
    SecuritySetup {
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
    #[br(pre_assert(kind == SegmentType::Ipc))]
    Ipc(
        #[br(parse_with = decrypt, args(size, state))]
        #[bw(write_with = encrypt, args(size, state))]
        T,
    ),
    #[br(pre_assert(kind == SegmentType::KeepAliveRequest))]
    KeepAliveRequest { id: u32, timestamp: u32 },
    #[br(pre_assert(kind == SegmentType::SecurityInitialize))]
    SecurityInitialize {
        #[br(count = 0x280)]
        #[brw(pad_size_to = 640)]
        data: Vec<u8>,
    },
    #[br(pre_assert(kind == SegmentType::KeepAliveResponse))]
    KeepAliveResponse { id: u32, timestamp: u32 },

    #[br(pre_assert(kind == SegmentType::KawariIpc))]
    KawariIpc(
        #[br(args(&0))] // this being zero is okay, custom ipc segments don't use the size arg
        CustomIpcSegment,
    ),
}

impl<T: ReadWriteIpcSegment> Default for SegmentData<T> {
    fn default() -> Self {
        Self::None()
    }
}

#[binrw]
#[derive(Debug)]
pub struct PacketHeader {
    // unknown purpose
    pub prefix: [u8; 16],
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
#[brw(import(state: &ConnectionState))]
#[derive(Debug, Clone)]
pub struct PacketSegment<T: ReadWriteIpcSegment> {
    #[bw(calc = self.calc_size())]
    pub size: u32,
    pub source_actor: u32,
    pub target_actor: u32,
    #[brw(pad_after = 2)] // padding
    pub segment_type: SegmentType,
    #[bw(args(*segment_type, size, state))]
    #[br(args(segment_type, size, state))]
    #[br(err_context("segment size = {}", size))]
    pub data: SegmentData<T>,
}

impl<T: ReadWriteIpcSegment> Default for PacketSegment<T> {
    fn default() -> Self {
        Self {
            source_actor: 0,
            target_actor: 0,
            segment_type: SegmentType::default(),
            data: SegmentData::default(),
        }
    }
}

impl<T: ReadWriteIpcSegment> PacketSegment<T> {
    pub fn calc_size(&self) -> u32 {
        let header = std::mem::size_of::<u32>() * 4;
        header as u32
            + match &self.data {
                SegmentData::None() => 0,
                SegmentData::SecuritySetup { .. } => 616,
                SegmentData::SecurityInitialize { .. } => 640,
                SegmentData::Ipc(data) => data.calc_size(),
                SegmentData::KeepAliveRequest { .. } => 0x8,
                SegmentData::KeepAliveResponse { .. } => 0x8,
                SegmentData::Initialize { .. } => 40,
                SegmentData::Setup { .. } => 40,
                SegmentData::KawariIpc(data) => data.calc_size(),
            }
    }
}

#[binrw]
#[brw(import(state: &mut ConnectionState))]
#[derive(Debug)]
struct Packet<T: ReadWriteIpcSegment> {
    header: PacketHeader,
    #[bw(args(state))]
    #[br(parse_with = decompress, args(&header, state,))]
    segments: Vec<PacketSegment<T>>,
}

/// State needed for each connection between the client & server, containing various things like the compressor or encryption keys.
pub enum ConnectionState {
    /// Used for stateless connections.
    None,
    /// Used for the Lobby connection.
    Lobby { client_key: [u8; 16] },
    /// Used for the Zone connection.
    Zone {
        serverbound_oodle: OodleNetwork,
        clientbound_oodle: OodleNetwork,
        scrambler_keys: Option<ScramblerKeys>,
    },
}

pub fn parse_packet_header(data: &[u8]) -> PacketHeader {
    let mut cursor = Cursor::new(data);

    match PacketHeader::read_le_args(&mut cursor, ()) {
        Ok(header) => header,
        Err(err) => {
            tracing::error!("{err}");

            PacketHeader {
                prefix: [0u8; 16],
                timestamp: 0,
                size: 0,
                connection_type: ConnectionType::None,
                segment_count: 0,
                version: 0,
                compression_type: CompressionType::Uncompressed,
                unk4: 0,
                uncompressed_size: 0,
            }
        }
    }
}

pub fn parse_packet<T: ReadWriteIpcSegment>(
    data: &[u8],
    state: &mut ConnectionState,
) -> (Vec<PacketSegment<T>>, ConnectionType) {
    let mut cursor = Cursor::new(data);

    match Packet::read_le_args(&mut cursor, (state,)) {
        Ok(packet) => (packet.segments, packet.header.connection_type),
        Err(err) => {
            tracing::error!("{err}");

            (Vec::new(), ConnectionType::None)
        }
    }
}
