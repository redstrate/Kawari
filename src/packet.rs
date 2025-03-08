use std::{fs::write, io::Cursor, time::{SystemTime, UNIX_EPOCH}};

use binrw::{binrw, helpers::until_eof, BinRead, BinWrite};
use physis::blowfish::Blowfish;
use tokio::{io::{AsyncWriteExt, WriteHalf}, net::TcpStream};

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
#[derive(Debug, Clone)]
enum SegmentType {
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
    #[brw(magic = 0x0Au32)]
    InitializationEncryptionResponse {
        #[br(count = 0x280)]
        data: Vec<u8>
    }
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
#[derive(Debug, Clone)]
struct PacketSegment {
    #[bw(calc = self.calc_size())]
    size: u32,
    source_actor: u32,
    target_actor: u32,
    segment_type: SegmentType,
}

impl PacketSegment {
    fn calc_size(&self) -> u32 {
        let header = std::mem::size_of::<u32>() * 4;
        return header as u32 + match &self.segment_type {
            SegmentType::InitializeEncryption { .. } => 616,
            SegmentType::InitializationEncryptionResponse { .. } => 640,
        };
    }
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

async fn send_packet(socket: &mut WriteHalf<TcpStream>, segments: &[PacketSegment]) {
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
        unk1: 0,
        unk2: 0,
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
    packet.write_le(&mut cursor);

    let buffer = cursor.into_inner();

    tracing::info!("Wrote response packet to outpacket.bin");
    write("outpacket.bin", &buffer);

    socket
        .write(&buffer)
        .await
        .expect("Failed to write packet!");
}

pub async fn parse_packet(socket: &mut WriteHalf<TcpStream>, data: &[u8]) {
    let mut cursor = Cursor::new(data);


    match Packet::read_le(&mut cursor) {
        Ok(packet) => {
            println!("{:#?}", packet);

            if packet.header.size as usize != data.len() {
                dump(
                    "Packet size mismatch between what we're given and the header!",
                     data,
                );
            }

            for segment in &packet.segments {
                match &segment.segment_type {
                    SegmentType::InitializeEncryption { phrase, key } => {
                        // Generate an encryption key for this client
                        let client_key = generate_encryption_key(key, phrase);

                        let blowfish = Blowfish::new(&client_key);
                        let mut data = blowfish.encrypt(&0xE0003C2Au32.to_le_bytes()).unwrap();
                        data.resize(0x280, 0);

                        let response_packet = PacketSegment {
                            source_actor: 0,
                            target_actor: 0,
                            segment_type: SegmentType::InitializationEncryptionResponse {
                                data
                            },
                        };
                        send_packet(socket, &[response_packet]).await;
                    },
                    SegmentType::InitializationEncryptionResponse { .. } => panic!("The server is recieving a response packet!"),
                }
            }

        },
        Err(err) => {
            println!("{err}");
            dump("Failed to parse packet!", data);
        },
    }
}

const GAME_VERSION: u16 = 7000;

pub fn generate_encryption_key(key: &[u8], phrase: &str) -> [u8; 16] {
    let mut base_key = vec![0x78, 0x56, 0x34, 0x12];
    base_key.extend_from_slice(&key);
    base_key.extend_from_slice(&GAME_VERSION.to_le_bytes());
    base_key.extend_from_slice(&[0; 2]); // padding (possibly for game version?)
    base_key.extend_from_slice(&phrase.as_bytes());

    md5::compute(&base_key).0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key() {
        let key = generate_encryption_key([0x00, 0x00, 0x00, 0x00], "foobar");
        assert_eq!(key, [169, 78, 235, 31, 57, 151, 26, 74, 250, 196, 1, 120, 206, 173, 202, 48]);
    }
}
