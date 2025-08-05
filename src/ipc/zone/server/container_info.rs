use binrw::binrw;

use crate::inventory::ContainerType;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct ContainerInfo {
    pub sequence: u32,
    pub num_items: u32,
    #[brw(pad_size_to = 4)]
    pub container: ContainerType,
    pub start_or_finish: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_containerinfo() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/container_info.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let container_info = ContainerInfo::read_le(&mut buffer).unwrap();
        assert_eq!(container_info.sequence, 1);
        assert_eq!(container_info.num_items, 0);
        assert_eq!(container_info.container, ContainerType::Inventory1);
        assert_eq!(container_info.start_or_finish, 0);
    }
}
