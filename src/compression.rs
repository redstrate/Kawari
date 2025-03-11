use std::fs::write;
use std::io::Cursor;

use binrw::{BinRead, BinResult};

use crate::{
    oodle::{FFXIVOodle, Oodle},
    packet::{PacketHeader, PacketSegment},
};

#[binrw::parser(reader, endian)]
pub(crate) fn decompress(
    oodle: &mut FFXIVOodle,
    header: &PacketHeader,
    encryption_key: Option<&[u8]>,
) -> BinResult<Vec<PacketSegment>> {
    let mut segments = Vec::new();

    let size = header.size as usize - std::mem::size_of::<PacketHeader>();

    println!(
        "known packet size: {} but decompressing {} bytes",
        header.size, size
    );

    let mut data = vec![0; size];
    reader.read_exact(&mut data).unwrap();

    write("compressed.bin", &data).unwrap();

    let data = match header.compressed {
        crate::packet::CompressionType::Uncompressed => data,
        crate::packet::CompressionType::Oodle => oodle.decode(data, header.oodle_decompressed_size),
    };

    write("decompressed.bin", &data).unwrap();

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
