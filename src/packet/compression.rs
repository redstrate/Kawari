use binrw::{BinWrite, binrw};
use std::io::Cursor;

use binrw::{BinRead, BinResult};

use crate::{
    config::get_config,
    packet::{PacketHeader, PacketSegment},
};

use super::{
    IPC_HEADER_SIZE, PacketState, ReadWriteIpcSegment, ScramblerKeys, SegmentData,
    oodle::OodleNetwork, scramble_packet,
};

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, PartialEq)]
pub enum CompressionType {
    Uncompressed = 0,
    ZLib = 1,
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
        CompressionType::Uncompressed => data,
        CompressionType::ZLib => unimplemented!(),
        CompressionType::Oodle => oodle.decode(data, header.uncompressed_size),
    };

    if header.compression_type == CompressionType::Oodle {
        assert_eq!(
            data.len(),
            header.uncompressed_size as usize,
            "Decompressed data does not match the expected length!"
        );
    }

    let mut cursor = Cursor::new(&data);

    let config = get_config();
    if config.packet_debugging {
        std::fs::write("decompressed.bin", &data).unwrap();
    }

    for _ in 0..header.segment_count {
        let current_position = cursor.position();
        segments.push(PacketSegment::read_options(
            &mut cursor,
            endian,
            (encryption_key,),
        )?);
        let new_position = cursor.position();
        let expected_size = segments.last().unwrap().calc_size() as u64;
        let actual_size = new_position - current_position;

        if expected_size != actual_size {
            tracing::warn!(
                "The segment {:#?} does not match the size in calc_size()! (expected {expected_size} got {actual_size}",
                segments.last()
            );
        }
    }

    Ok(segments)
}

pub(crate) fn compress<T: ReadWriteIpcSegment>(
    state: &mut PacketState,
    compression_type: &CompressionType,
    segments: &[PacketSegment<T>],
    keys: Option<&ScramblerKeys>,
) -> (Vec<u8>, usize) {
    let mut segments_buffer = Vec::new();
    for segment in segments {
        let mut buffer = Vec::new();

        // write to buffer
        {
            let mut cursor = Cursor::new(&mut buffer);

            segment
                .write_le_args(
                    &mut cursor,
                    (state.client_key.as_ref().map(|s: &[u8; 16]| s.as_slice()),),
                )
                .unwrap();
        }

        // obsfucate if needed
        if let Some(keys) = keys {
            if let SegmentData::Ipc { data } = &segment.data {
                let opcode = data.get_opcode();
                let base_key = keys.get_base_key(opcode);

                scramble_packet(
                    data.get_name(),
                    base_key,
                    &mut buffer[IPC_HEADER_SIZE as usize..],
                );
            }
        }

        segments_buffer.append(&mut buffer);
    }

    let segments_buffer_len = segments_buffer.len();

    match compression_type {
        CompressionType::Uncompressed => (segments_buffer, 0),
        CompressionType::ZLib => unimplemented!(),
        CompressionType::Oodle => (
            state.clientbound_oodle.encode(segments_buffer),
            segments_buffer_len,
        ),
    }
}
