use binrw::binrw;

use crate::common::ContainerType;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CurrencyInfo {
    pub sequence: u32,
    /// Which container this currency is stored in.
    pub container: ContainerType,
    /// The slot in the container this is updating.
    pub slot: u16,
    /// How much of the currency we're holding.
    #[brw(pad_after = 4)] // not read by the client
    pub quantity: u32,
    /// Index into the Item Excel sheet.
    #[brw(pad_after = 4)] // not read by the client
    pub item_id: u32,
}
