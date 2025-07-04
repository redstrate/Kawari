use std::io::Cursor;

use binrw::BinWrite;
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::common::timestamp_msecs;

use super::{
    CompressionType, ConnectionType, PacketHeader, PacketSegment, PacketState, ReadWriteIpcSegment,
    ScramblerKeys, SegmentData, SegmentType, compression::compress,
};

pub async fn send_packet<T: ReadWriteIpcSegment>(
    socket: &mut TcpStream,
    state: &mut PacketState,
    connection_type: ConnectionType,
    compression_type: CompressionType,
    segments: &[PacketSegment<T>],
    keys: Option<&ScramblerKeys>,
) {
    let (data, uncompressed_size) = compress(state, &compression_type, segments, keys);
    let size = std::mem::size_of::<PacketHeader>() + data.len();

    let header = PacketHeader {
        prefix: [0; 16],
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

    if let Err(e) = socket.write_all(&buffer).await {
        tracing::warn!("Failed to send packet: {e}");
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
        None,
    )
    .await;
}
