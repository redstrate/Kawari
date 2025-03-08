use std::{fs::write, io::Cursor};

use binrw::{binrw, BinRead};

pub(crate) fn read_bool_from<T: std::convert::From<u8> + std::cmp::PartialEq>(x: T) -> bool {
    x == T::from(1u8)
}

pub(crate) fn write_bool_as<T: std::convert::From<u8>>(x: &bool) -> T {
    if *x { T::from(1u8) } else { T::from(0u8) }
}

#[binrw]
#[brw(repr = u16)]
#[derive(Debug)]
enum ConnectionType {
    Lobby = 0x3,
}

#[binrw]
#[derive(Debug)]
struct PacketHeader {
    unk1: u64,
    unk2: u64,
    timestamp: u64,
    size: u32,
    connection_type: ConnectionType,
    count: u16,
    unk3: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    compressed: bool,
    unk4: u32,
}

fn dump(msg: &str, data: &[u8]) {
    write("packet.bin", data);
    panic!("{msg} Dumped to packet.bin.");
}

pub fn parse_packet(data: &[u8]) {
    let mut cursor = Cursor::new(data);

    if let Ok(packet) = PacketHeader::read_le(&mut cursor) {
        println!("{:#?}", packet);

        if packet.size as usize != data.len() {
            dump("Packet size mismatch between what we're given and the header!", data);
        }
    } else {
        dump("Failed to parse packet!", data);
    }
}
