use binrw::binrw;

use crate::inventory::ContainerType;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct ItemInfo {
    pub sequence: u32,
    pub unk1: u32,
    pub container: ContainerType,
    pub slot: u16,
    pub quantity: u32,
    pub catalog_id: u32,
    pub reserved_flag: u32,
    pub signature_id: u64,
    pub hq_flag: u8,
    pub unk2: u8,
    pub condition: u16,
    pub spirit_bond: u16,
    pub stain: u16,
    pub glamour_catalog_id: u32,
    pub materia: [u16; 5],
    #[brw(pad_before = 6)]
    pub unk3: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_iteminfo() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("item_info.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let item_info = ItemInfo::read_le(&mut buffer).unwrap();
        assert_eq!(item_info.sequence, 4);
        assert_eq!(item_info.unk1, 0);
        assert_eq!(item_info.container, ContainerType::Equipped);
        assert_eq!(item_info.slot, 11);
        assert_eq!(item_info.quantity, 1);
        assert_eq!(item_info.catalog_id, 4426);
        assert_eq!(item_info.reserved_flag, 0);
        assert_eq!(item_info.signature_id, 0);
        assert_eq!(item_info.hq_flag, 0);
        assert_eq!(item_info.unk2, 0);
        assert_eq!(item_info.condition, 30000);
        assert_eq!(item_info.spirit_bond, 0);
        assert_eq!(item_info.stain, 0);
        assert_eq!(item_info.glamour_catalog_id, 0);
        assert_eq!(item_info.unk3, 0);
    }
}
