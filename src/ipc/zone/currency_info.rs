use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct CurrencyInfo {
    pub sequence: u32,
    pub container: u16,
    pub slot: u16,
    pub quantity: u32,
    pub unk1: u32,
    pub catalog_id: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
}
