use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateClassInfo {
    pub class_id: u16,
    pub unknown: u8,
    pub is_specialist: u8,
    pub synced_level: u16,
    pub class_level: u16,
    pub role_actions: [u32; 2],
}
