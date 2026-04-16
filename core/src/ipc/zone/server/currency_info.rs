use binrw::binrw;

use crate::common::ContainerType;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct CurrencyInfo {
    pub sequence: u32,
    /// Which container this currency is stored in.
    pub container: ContainerType,
    /// The slot in the container this is updating.
    pub slot: u16,
    /// How much of the currency we're holding.
    pub quantity: u32,
    pub unk1: u32,
    /// Index into the Item Excel sheet.
    pub catalog_id: u32,
    pub unk2: u32,
}
