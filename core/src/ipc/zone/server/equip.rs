use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct Equip {
    pub main_weapon_id: u64,
    pub sub_weapon_id: u64,
    pub crest_enable: u8,
    pub classjob_id: u8,
    pub pattern_invalid: u16,
    pub model_ids: [u32; 10],
    /// These appear to be some sort of display flags? They're continually included as long as the gear in question remains equipped.
    /// Associated with headgear: set to 0x41 for some headgear when equipped, 0 when unequipped, observed when equipping Songbird Hat; 0x41 doesn't seem to be related to visor toggling or hiding earrings.
    pub headgear_unk: u8,
    #[brw(pad_after = 10)]
    /// Associated with chest gear: set to 0x01 when equipped, 0 when unequipped. Observed when equipping Martial Artist's Sleeveless Vest, a piece which hides necklaces.
    pub chestgear_unk: u8,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_containerinfo() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("equip.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let equip = Equip::read_le(&mut buffer).unwrap();
        assert_eq!(equip.main_weapon_id, 4297785545);
        assert_eq!(equip.sub_weapon_id, 0);
        assert_eq!(equip.crest_enable, 0);
        assert_eq!(equip.pattern_invalid, 1);
        assert_eq!(equip.model_ids[0], 0);
        assert_eq!(equip.model_ids[1], 0);
        assert_eq!(equip.model_ids[2], 131156);
        assert_eq!(equip.model_ids[3], 131156);
        assert_eq!(equip.model_ids[4], 131156);
        assert_eq!(equip.model_ids[5], 131073);
        assert_eq!(equip.model_ids[6], 131073);
        assert_eq!(equip.model_ids[7], 131073);
        assert_eq!(equip.model_ids[8], 0);
        assert_eq!(equip.model_ids[9], 131073);
    }
}
