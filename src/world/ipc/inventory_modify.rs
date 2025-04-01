use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct InventoryModify {
    context_id: u32,
    operation_type: u8,
    #[brw(pad_before = 3)]
    src_actor_id: u32,
    src_storage_id: u32,
    src_container_index: i16,
    #[brw(pad_before = 4)]
    src_stack: u32,
    src_catalog_id: u32,
    dst_actor_id: u32,
    dst_storage_id: u32,
    dst_container_index: i16,
    #[brw(pad_before = 2)]
    dst_stack: u32,
    dst_catalog_id: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_inventory_modify() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/inventory_modify.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let modify_inventory = InventoryModify::read_le(&mut buffer).unwrap();
        assert_eq!(modify_inventory.context_id, 0x10000002);
        assert_eq!(modify_inventory.operation_type, 70);
        assert_eq!(modify_inventory.src_actor_id, 0);
        assert_eq!(modify_inventory.src_storage_id, 1000);
        assert_eq!(modify_inventory.src_container_index, 4);
        assert_eq!(modify_inventory.src_stack, 0);
        assert_eq!(modify_inventory.src_catalog_id, 0);
        assert_eq!(modify_inventory.dst_actor_id, 209911808);
        assert_eq!(modify_inventory.dst_storage_id, 0);
        assert_eq!(modify_inventory.dst_container_index, 96);
        assert_eq!(modify_inventory.dst_stack, 0);
        assert_eq!(modify_inventory.dst_catalog_id, 4194304);
    }
}
