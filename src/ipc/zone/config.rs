use binrw::binrw;

use super::DisplayFlag;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct Config {
    #[brw(pad_after = 6)]
    pub display_flag: DisplayFlag,
}
