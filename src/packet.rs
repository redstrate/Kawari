use std::{
    fs::write,
    io::Cursor,
    time::{SystemTime, UNIX_EPOCH},
};

use binrw::{BinRead, BinWrite, binrw};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
};

use crate::{
    common::{read_bool_from, read_string, write_bool_as},
    encryption::{decrypt, encrypt},
    ipc::IPCSegment,
};

#[binrw]
#[brw(repr = u16)]
#[derive(Debug)]
enum ConnectionType {
    None = 0x0,
    Zone = 0x1,
    Chat = 0x2,
    Lobby = 0x3,
}

#[binrw]
#[brw(import(size: u32, encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
pub enum SegmentType {
    // Client->Server Packets
    #[brw(magic = 0x9u32)]
    InitializeEncryption {
        #[brw(pad_before = 36)] // empty
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        phrase: String,

        #[brw(pad_after = 512)] // empty
        key: [u8; 4],
    },
    #[brw(magic = 0x3u32)]
    Ipc {
        #[br(parse_with = decrypt, args(size, encryption_key))]
        #[bw(write_with = encrypt, args(size, encryption_key))]
        data: IPCSegment,
    },
    #[brw(magic = 0x7u32)]
    KeepAlive { id: u32, timestamp: u32 },

    // Server->Client Packets
    #[brw(magic = 0x0Au32)]
    InitializationEncryptionResponse {
        #[br(count = 0x280)]
        data: Vec<u8>,
    },
    #[brw(magic = 0x08u32)]
    KeepAliveResponse { id: u32, timestamp: u32 },
}

#[binrw]
#[derive(Debug)]
struct PacketHeader {
    unk1: u64,
    unk2: u64,
    timestamp: u64,
    size: u32,
    connection_type: ConnectionType,
    segment_count: u16,
    unk3: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    compressed: bool,
    unk4: u16,
    unk5: u32, // iolite says the size after oodle decompression
}

#[binrw]
#[brw(import(encryption_key: Option<&[u8]>))]
#[derive(Debug, Clone)]
pub struct PacketSegment {
    #[bw(calc = self.calc_size())]
    pub size: u32,
    pub source_actor: u32,
    pub target_actor: u32,
    #[brw(args(size, encryption_key))]
    pub segment_type: SegmentType,
}

impl PacketSegment {
    fn calc_size(&self) -> u32 {
        let header = std::mem::size_of::<u32>() * 4;
        header as u32
            + match &self.segment_type {
                SegmentType::InitializeEncryption { .. } => 616,
                SegmentType::InitializationEncryptionResponse { .. } => 640,
                SegmentType::Ipc { data } => data.calc_size(),
                SegmentType::KeepAlive { .. } => todo!(),
                SegmentType::KeepAliveResponse { .. } => 0x8,
            }
    }
}

#[binrw]
#[brw(import(encryption_key: Option<&[u8]>))]
#[derive(Debug)]
struct Packet {
    header: PacketHeader,
    #[br(count = header.segment_count, args { inner: (encryption_key,) })]
    #[bw(args(encryption_key))]
    segments: Vec<PacketSegment>,
}

fn dump(msg: &str, data: &[u8]) {
    write("packet.bin", data).unwrap();
    panic!("{msg} Dumped to packet.bin.");
}

pub async fn send_packet(
    socket: &mut WriteHalf<TcpStream>,
    segments: &[PacketSegment],
    state: &State,
) {
    let timestamp: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Failed to get UNIX timestamp!")
        .as_millis()
        .try_into()
        .unwrap();

    let mut total_segment_size = 0;
    for segment in segments {
        total_segment_size += segment.calc_size();
    }

    let header = PacketHeader {
        unk1: 0xE2465DFF41A05252, // wtf?
        unk2: 0x75C4997B4D642A7F, // wtf? x2
        timestamp,
        size: std::mem::size_of::<PacketHeader>() as u32 + total_segment_size,
        connection_type: ConnectionType::Lobby,
        segment_count: segments.len() as u16,
        unk3: 0,
        compressed: false,
        unk4: 0,
        unk5: 0,
    };

    let packet = Packet {
        header,
        segments: segments.to_vec(),
    };

    let mut cursor = Cursor::new(Vec::new());
    packet
        .write_le_args(
            &mut cursor,
            (state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),),
        )
        .unwrap();

    let buffer = cursor.into_inner();

    tracing::info!("Wrote response packet to outpacket.bin");
    write("outpacket.bin", &buffer).unwrap();

    socket
        .write_all(&buffer)
        .await
        .expect("Failed to write packet!");
}

// temporary
pub struct State {
    pub client_key: Option<[u8; 16]>,
}

pub async fn parse_packet(data: &[u8], state: &mut State) -> Vec<PacketSegment> {
    let mut cursor = Cursor::new(data);

    match Packet::read_le_args(
        &mut cursor,
        (state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),),
    ) {
        Ok(packet) => {
            println!("{:#?}", packet);

            if packet.header.size as usize != data.len() {
                dump(
                    "Packet size mismatch between what we're given and the header!",
                    data,
                );
            }

            packet.segments
        }
        Err(err) => {
            println!("{err}");
            dump("Failed to parse packet!", data);

            Vec::new()
        }
    }
}

pub async fn send_keep_alive(
    socket: &mut WriteHalf<TcpStream>,
    state: &State,
    id: u32,
    timestamp: u32,
) {
    let response_packet = PacketSegment {
        source_actor: 0,
        target_actor: 0,
        segment_type: SegmentType::KeepAliveResponse { id, timestamp },
    };
    send_packet(socket, &[response_packet], state).await;
}
