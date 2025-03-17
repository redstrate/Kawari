use binrw::{BinWrite, binrw};
use std::io::Cursor;

use binrw::{BinRead, BinResult};

use crate::{
    oodle::OodleNetwork,
    packet::{PacketHeader, PacketSegment},
};

use super::{PacketState, ReadWriteIpcSegment};

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, PartialEq)]
pub enum CompressionType {
    Uncompressed = 0,
    Oodle = 2,
}

#[binrw::parser(reader, endian)]
pub(crate) fn decompress<T: ReadWriteIpcSegment>(
    oodle: &mut OodleNetwork,
    header: &PacketHeader,
    encryption_key: Option<&[u8]>,
) -> BinResult<Vec<PacketSegment<T>>> {
    let mut segments = Vec::new();

    let size = header.size as usize - std::mem::size_of::<PacketHeader>();

    let mut data = vec![0; size];
    reader.read_exact(&mut data).unwrap();

    let data = match header.compression_type {
        crate::packet::CompressionType::Uncompressed => data,
        crate::packet::CompressionType::Oodle => oodle.decode(data, header.uncompressed_size),
    };

    if header.compression_type == crate::packet::CompressionType::Oodle {
        assert_eq!(
            data.len(),
            header.uncompressed_size as usize,
            "Decompressed data does not match the expected length!"
        );
    }

    let mut cursor = Cursor::new(&data);

    for _ in 0..header.segment_count {
        segments.push(PacketSegment::read_options(
            &mut cursor,
            endian,
            (encryption_key,),
        )?);
    }

    Ok(segments)
}

pub(crate) fn compress<T: ReadWriteIpcSegment>(
    state: &mut PacketState,
    compression_type: &CompressionType,
    segments: &[PacketSegment<T>],
) -> (Vec<u8>, usize) {
    let mut segments_buffer = Cursor::new(Vec::new());
    for segment in segments {
        segment
            .write_le_args(
                &mut segments_buffer,
                (state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),),
            )
            .unwrap();
    }

    let segments_buffer = segments_buffer.into_inner();
    let segments_buffer_len = segments_buffer.len();

    match compression_type {
        CompressionType::Uncompressed => (segments_buffer, 0),
        CompressionType::Oodle => (
            state.clientbound_oodle.encode(segments_buffer),
            segments_buffer_len,
        ),
    }
}
