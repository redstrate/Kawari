use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Copy, Default)]
pub struct House {
    pub plot_size: u8,
    pub status: u8,
    #[brw(pad_after = 1)]
    pub flags: u8,
    pub fc_id: u32,
    pub fc_crest_id: u32,
    pub fc_crest_id1: u32,
    pub pattern_ids: [u16; 8],
    pub colors: [u8; 8],
}

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct HouseList {
    pub land_id: u16,
    pub ward: u16,
    pub territory_type_id: u16,
    pub world_id: u16,

    #[brw(pad_after = 4)] // seems empty
    pub subdivision: u32,

    pub houses: [House; 30],
}
