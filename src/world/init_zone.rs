use binrw::binrw;

use super::Position;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct InitZone {
    pub server_id: u16,
    pub zone_id: u16,
    pub zone_index: u16,
    pub content_finder_condition_id: u16,
    pub layer_set_id: u32,
    pub layout_id: u32,
    pub weather_id: u16, // index into Weather sheet probably?
    pub unk_really: u16,
    pub unk_bitmask1: u8,
    pub unk_bitmask2: u8,
    pub unk1: u8,
    pub unk2: u32,
    pub festival_id: u16,
    pub additional_festival_id: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: [u32; 4],
    pub unk7: [u32; 3],
    pub unk8_9: [u8; 9],
    pub position: Position,
    pub unk8: [u32; 4],
    pub unk9: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_init_zone() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/init_zone.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let init_zone = InitZone::read_le(&mut buffer).unwrap();
        assert_eq!(init_zone.zone_id, 182);
        assert_eq!(init_zone.weather_id, 2);
        assert_eq!(init_zone.position.x, 40.519722);
        assert_eq!(init_zone.position.y, 4.0);
        assert_eq!(init_zone.position.z, -150.33124);
    }
}
