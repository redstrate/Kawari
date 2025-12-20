use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateClassInfo {
    pub class_id: u8,
    #[brw(pad_before = 1)] // should be empty
    pub current_level: u16,
    pub class_level: u16,
    pub synced_level: u16,
    pub current_exp: i32,
    pub rested_exp: u32,
}
