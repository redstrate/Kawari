use binrw::binrw;

use crate::common::ContainerType;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct DyeInformation {
    #[brw(pad_size_to = 4)]
    pub target_container: ContainerType,
    pub target_slot: u32,
    #[brw(pad_size_to = 4)]
    pub dye1_container: ContainerType,
    #[brw(pad_size_to = 4)]
    pub dye2_container: ContainerType,
    pub unk1: u32,
    pub dye1: u8,
    pub dye2: u8,
    pub unk2: u16,
}
