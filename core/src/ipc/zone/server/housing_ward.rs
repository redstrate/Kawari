use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct HousingWardMenuSummaryItem {
    pub plot_price: u32,
    /// Flags? Unknown what they mean, needs research
    pub flags: u32,
    #[brw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

impl HousingWardMenuSummaryItem {
    pub const SIZE: usize = 40;
}
