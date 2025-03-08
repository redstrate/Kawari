use std::fs::write;

pub fn parse_packet(data: &[u8]) {
    write("packet.bin", data);
    panic!("Unknown packet! Dumping to packet.bin.");
}
