use std::{fs::write, io::Cursor};

use binrw::{BinRead, binrw, helpers::until_eof};

pub(crate) fn read_bool_from<T: std::convert::From<u8> + std::cmp::PartialEq>(x: T) -> bool {
    x == T::from(1u8)
}

pub(crate) fn write_bool_as<T: std::convert::From<u8>>(x: &bool) -> T {
    if *x { T::from(1u8) } else { T::from(0u8) }
}

pub(crate) fn read_string(byte_stream: Vec<u8>) -> String {
    let str = String::from_utf8(byte_stream).unwrap();
    str.trim_matches(char::from(0)).to_string() // trim \0 from the end of strings
}

#[binrw]
#[brw(repr = u16)]
#[derive(Debug)]
enum ConnectionType {
    Lobby = 0x3,
}

#[binrw]
#[derive(Debug)]
enum SegmentType {
    #[brw(magic = 0x9u32)]
    InitializeEncryption {
        #[br(pad_before = 36)] // empty
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        phrase: String,

        #[br(pad_after = 512)] // empty
        key: u32,
    },
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
#[derive(Debug)]
struct PacketSegment {
    size: u32,
    source_actor: u32,
    target_actor: u32,
    segment_type: SegmentType,
}

#[binrw]
#[derive(Debug)]
struct Packet {
    header: PacketHeader,
    #[br(count = header.segment_count)]
    segments: Vec<PacketSegment>,
}

fn dump(msg: &str, data: &[u8]) {
    write("packet.bin", data);
    panic!("{msg} Dumped to packet.bin.");
}

pub fn parse_packet(data: &[u8]) {
    let mut cursor = Cursor::new(data);

    if let Ok(packet) = Packet::read_le(&mut cursor) {
        println!("{:#?}", packet);

        if packet.header.size as usize != data.len() {
            dump(
                "Packet size mismatch between what we're given and the header!",
                data,
            );
        }

        dump("nothing", data);
    } else {
        dump("Failed to parse packet!", data);
    }
}
