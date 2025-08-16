use std::io::Cursor;

use binrw::{BinRead, binrw};

use crate::{
    common::{read_string, write_string},
    ipc::kawari::CustomIpcSegment,
    packet::encryption::decrypt,
};

use super::{
    CompressionType, compression::decompress, encryption::encrypt, ipc::ReadWriteIpcSegment,
    oodle::OodleNetwork,
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
#[brw(repr = u16)]
#[derive(Debug, PartialEq, Copy, Clone, Default)]
pub enum SegmentType {
    #[default]
    None = 0x0,
    Setup = 0x1,
    Initialize = 0x2,
    // Also known as "UPLAYER"
    Ipc = 0x3,
    KeepAliveRequest = 0x7,
    KeepAliveResponse = 0x8,
    // Also known as "SECSETUP"
    SecuritySetup = 0x9,
    // Also known as "SECINIT"
    SecurityInitialize = 0xA,
    // This isn't in retail!
    KawariIpc = 0xAAAA,
}

#[binrw]
#[brw(import(kind: SegmentType, size: u32, encryption_key: Option<&[u8]>))]
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
        #[br(parse_with = decrypt, args(size, encryption_key))]
        #[bw(write_with = encrypt, args(size, encryption_key))]
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
#[brw(import(encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
pub struct PacketSegment<T: ReadWriteIpcSegment> {
    #[bw(calc = self.calc_size())]
    pub size: u32,
    pub source_actor: u32,
    pub target_actor: u32,
    #[brw(pad_after = 2)] // padding
    pub segment_type: SegmentType,
    #[bw(args(*segment_type, size, encryption_key))]
    #[br(args(segment_type, size, encryption_key))]
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
#[brw(import(oodle: &mut OodleNetwork, encryption_key: Option<&[u8]>))]
#[derive(Debug)]
struct Packet<T: ReadWriteIpcSegment> {
    header: PacketHeader,
    #[bw(args(encryption_key))]
    #[br(parse_with = decompress, args(oodle, &header, encryption_key,))]
    segments: Vec<PacketSegment<T>>,
}

// temporary
/// State needed for each connection between the client & server, containing various things like the compressor and encryption keys.
pub struct PacketState {
    pub client_key: Option<[u8; 16]>,
    pub serverbound_oodle: OodleNetwork,
    pub clientbound_oodle: OodleNetwork,
}

pub fn parse_packet<T: ReadWriteIpcSegment>(
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

            (Vec::new(), ConnectionType::None)
        }
    }
}
