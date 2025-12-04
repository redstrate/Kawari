use binrw::binrw;

use crate::common::EquipDisplayFlag;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct Config {
    #[brw(pad_after = 6)]
    pub display_flag: EquipDisplayFlag,
}
