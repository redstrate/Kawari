use binrw::binrw;

#[binrw]
#[repr(u8)]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum WaymarkPlacementMode {
    #[default]
    Removed = 0,
    Placed = 1,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default)]
pub struct WaymarkPreset {
    #[br(count = 104)]
    #[bw(pad_size_to = 104)]
    unk: Vec<u8>,
}
