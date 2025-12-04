use binrw::binrw;

use crate::common::{ContainerType, ItemOperationKind};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ItemOperation {
    pub context_id: u32,
    pub operation_type: ItemOperationKind,

    pub src_actor_id: u32,
    #[brw(pad_size_to = 4)]
    pub src_storage_id: ContainerType,
    pub src_container_index: u16,
    #[brw(pad_before = 2)]
    pub src_stack: u32,
    pub src_catalog_id: u32,

    pub dst_actor_id: u32,
    #[brw(pad_size_to = 4)]
    pub dst_storage_id: ContainerType,
    pub dst_container_index: u16,
    #[brw(pad_before = 2)]
    pub dst_stack: u32,
    pub dst_catalog_id: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::client_zone_tests_dir;

    use super::*;

    #[test]
    fn read_inventory_modify() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(client_zone_tests_dir!("inventory_modify.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let modify_inventory = ItemOperation::read_le(&mut buffer).unwrap();
        assert_eq!(modify_inventory.context_id, 0x10000000);
        assert_eq!(modify_inventory.operation_type, ItemOperationKind::Move);
        assert_eq!(modify_inventory.src_actor_id, 0);
        assert_eq!(modify_inventory.src_storage_id, ContainerType::Equipped);
        assert_eq!(modify_inventory.src_container_index, 3);
        assert_eq!(modify_inventory.src_stack, 1);
        assert_eq!(modify_inventory.src_catalog_id, 0);
        assert_eq!(modify_inventory.dst_actor_id, 0);
        assert_eq!(modify_inventory.dst_storage_id, ContainerType::ArmoryBody);
        assert_eq!(modify_inventory.dst_container_index, 0);
        assert_eq!(modify_inventory.dst_stack, 0);
        assert_eq!(modify_inventory.dst_catalog_id, 0);
    }
}
