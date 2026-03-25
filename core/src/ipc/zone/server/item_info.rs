use binrw::binrw;

use crate::common::ContainerType;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct ItemInfo {
    /// Starts from zero and increases by one for each of these packets during this gameplay session
    #[brw(pad_after = 4)] // unused
    pub sequence: u32,
    /// What container this item is placed in.
    pub container: ContainerType,
    /// What slot in the container this item is placed in.
    pub slot: u16,
    /// How many of this item occupies it's slot.
    pub quantity: u32,
    /// Index into the Item Excel sheet.
    #[brw(pad_after = 4)] // unused
    pub item_id: u32,
    /// The player who crafted this item.
    pub crafter_content_id: u64,
    #[brw(pad_after = 1)] // unused
    /// Unknown flags.
    pub item_flags: u8,
    /// The condition of this item from 0 to 30000.
    pub condition: u16,
    #[brw(pad_after = 2)] // unused
    /// Spiritbond or collectability stat.
    pub spiritbond_or_collectability: u16,
    /// If not zero, what Item this is glamoured to.
    pub glamour_id: u32,
    /// The materia melded into this item.
    pub materia: [u16; 5],
    /// The grade of said materia.
    pub materia_grades: [u8; 5],
    #[brw(pad_after = 3)] // unused
    /// Dye information?
    pub stains: [u8; 2],
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
        assert_eq!(item_info.container, ContainerType::Equipped);
        assert_eq!(item_info.slot, 11);
        assert_eq!(item_info.quantity, 1);
        assert_eq!(item_info.item_id, 4426);
        assert_eq!(item_info.crafter_content_id, 0);
        assert_eq!(item_info.item_flags, 0);
        assert_eq!(item_info.condition, 30000);
        assert_eq!(item_info.spiritbond_or_collectability, 0);
        assert_eq!(item_info.glamour_id, 0);
        assert_eq!(item_info.materia, [0; 5]);
        assert_eq!(item_info.materia_grades, [0; 5]);
        assert_eq!(item_info.stains, [0; 2]);
    }
}
