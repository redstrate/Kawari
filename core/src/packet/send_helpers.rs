use std::io::{self, Cursor, ErrorKind};

use binrw::BinWrite;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    common::RECEIVE_BUFFER_SIZE, common::timestamp_msecs, config::get_config,
    ipc::kawari::CustomIpcSegment,
};

use super::{
    CompressionType, ConnectionState, ConnectionType, PacketHeader, PacketSegment,
    ReadWriteIpcSegment, SegmentData, SegmentType, compression::compress, parse_packet,
    parse_packet_header,
};

const PACKET_HEADER_SIZE: usize = std::mem::size_of::<PacketHeader>();
const PACKET_SIZE_OFFSET: usize = 24;
const CONNECTION_TYPE_OFFSET: usize = 28;
const SEGMENT_COUNT_OFFSET: usize = 30;
const COMPRESSION_TYPE_OFFSET: usize = 33;

fn packet_size_from_header(header: &PacketHeader) -> io::Result<usize> {
    let size = header.size as usize;
    if !(PACKET_HEADER_SIZE..=RECEIVE_BUFFER_SIZE).contains(&size) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "invalid packet size {size}; expected {PACKET_HEADER_SIZE}..={RECEIVE_BUFFER_SIZE}"
            ),
        ));
    }

    Ok(size)
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap())
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}

fn packet_size_from_raw_header(header: &[u8]) -> io::Result<usize> {
    if header.len() < PACKET_HEADER_SIZE {
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "packet header is incomplete",
        ));
    }

    let size = read_u32_le(header, PACKET_SIZE_OFFSET) as usize;
    if !(PACKET_HEADER_SIZE..=RECEIVE_BUFFER_SIZE).contains(&size) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!(
                "invalid packet size {size}; expected {PACKET_HEADER_SIZE}..={RECEIVE_BUFFER_SIZE}"
            ),
        ));
    }

    let connection_type = read_u16_le(header, CONNECTION_TYPE_OFFSET);
    if !matches!(connection_type, 0x0 | 0x1 | 0x2 | 0x3 | 0xAAAA) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid connection type {connection_type:#06x}"),
        ));
    }

    let segment_count = read_u16_le(header, SEGMENT_COUNT_OFFSET);
    if segment_count == 0 {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "packet contains zero segments",
        ));
    }

    let compression_type = header[COMPRESSION_TYPE_OFFSET];
    if !matches!(compression_type, 0x0 | 0x1 | 0x2) {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("invalid compression type {compression_type:#04x}"),
        ));
    }

    Ok(size)
}

fn find_next_packet_header(buffer: &[u8]) -> Option<usize> {
    if buffer.len() < PACKET_HEADER_SIZE {
        return None;
    }

    (1..=buffer.len() - PACKET_HEADER_SIZE)
        .find(|&offset| packet_size_from_raw_header(&buffer[offset..]).is_ok())
}

/// Reassembles FFXIV packets from TCP reads. A single socket read can contain a partial packet or
/// multiple packets; parsing anything except an exact packet corrupts Oodle's packet history.
#[derive(Debug, Default)]
pub struct PacketReadBuffer {
    pending: Vec<u8>,
}

impl PacketReadBuffer {
    pub fn push(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        self.pending.extend_from_slice(data);

        let mut packets = Vec::new();
        loop {
            if self.pending.len() < PACKET_HEADER_SIZE {
                break;
            }

            let packet_size = match packet_size_from_raw_header(&self.pending[..PACKET_HEADER_SIZE])
            {
                Ok(packet_size) => packet_size,
                Err(err) => {
                    if let Some(offset) = find_next_packet_header(&self.pending) {
                        tracing::warn!(
                            pending_len = self.pending.len(),
                            drop_len = offset,
                            "Dropping {offset} buffered bytes before the next plausible packet header after reading an invalid packet header: {err}"
                        );
                        self.pending.drain(..offset);
                        continue;
                    }

                    tracing::warn!(
                        pending_len = self.pending.len(),
                        "Dropping {} buffered bytes after reading an invalid packet header: {err}",
                        self.pending.len()
                    );
                    self.pending.clear();
                    break;
                }
            };

            if self.pending.len() < packet_size {
                break;
            }

            packets.push(self.pending.drain(..packet_size).collect());
        }

        packets
    }
}

pub async fn read_packet(socket: &mut TcpStream) -> io::Result<Option<Vec<u8>>> {
    let mut packet = vec![0; PACKET_HEADER_SIZE];
    match socket.read_exact(&mut packet).await {
        Ok(_) => {}
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err),
    }

    let header = parse_packet_header(&packet);
    let packet_size = packet_size_from_header(&header)?;
    packet.resize(packet_size, 0);
    socket.read_exact(&mut packet[PACKET_HEADER_SIZE..]).await?;

    Ok(Some(packet))
}

pub async fn send_packet<T: ReadWriteIpcSegment>(
    socket: &mut TcpStream,
    state: &mut ConnectionState,
    connection_type: ConnectionType,
    compression_type: CompressionType,
    segments: &[PacketSegment<T>],
) {
    let (data, uncompressed_size) = compress(state, &compression_type, segments);
    let size = std::mem::size_of::<PacketHeader>() + data.len();

    let header = PacketHeader {
        timestamp: timestamp_msecs(),
        size: size as u32,
        connection_type,
        segment_count: segments.len() as u16,
        compression_type,
        uncompressed_size: uncompressed_size as u32,
        ..Default::default()
    };

    let mut cursor = Cursor::new(Vec::with_capacity(size));
    header.write_le(&mut cursor).unwrap();
    std::io::Write::write_all(&mut cursor, &data).unwrap();

    let buffer = cursor.into_inner();
    assert!(buffer.len() < RECEIVE_BUFFER_SIZE);

    if let Err(e) = socket.write_all(&buffer).await {
        tracing::warn!("Failed to send packet: {e}");
    }
}

pub async fn send_keep_alive<T: ReadWriteIpcSegment>(
    socket: &mut TcpStream,
    state: &mut ConnectionState,
    connection_type: ConnectionType,
    id: u32,
    timestamp: u32,
) {
    let response_packet: PacketSegment<T> = PacketSegment {
        segment_type: SegmentType::KeepAliveResponse,
        data: SegmentData::KeepAliveResponse { id, timestamp },
        ..Default::default()
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

/// Sends a custom IPC packet to the world server, meant for private server-to-server communication.
/// Returns the first custom IPC segment returned.
pub async fn send_custom_world_packet(segment: CustomIpcSegment) -> Option<CustomIpcSegment> {
    let config = get_config();

    let addr = config.world.get_public_socketaddr();

    let mut stream = TcpStream::connect(addr).await.ok()?;

    let mut packet_state = ConnectionState::None;

    let segment: PacketSegment<CustomIpcSegment> = PacketSegment {
        segment_type: SegmentType::KawariIpc,
        data: SegmentData::KawariIpc(segment),
        ..Default::default()
    };

    send_packet(
        &mut stream,
        &mut packet_state,
        ConnectionType::KawariIpc,
        CompressionType::Uncompressed,
        &[segment],
    )
    .await;

    // read response
    if let Some(packet) = read_packet(&mut stream)
        .await
        .expect("Failed to read data!")
    {
        let segments = parse_packet::<CustomIpcSegment>(&packet, &mut packet_state);

        return match &segments[0].data {
            SegmentData::KawariIpc(data) => Some(data.clone()),
            _ => None,
        };
    }

    None
}
