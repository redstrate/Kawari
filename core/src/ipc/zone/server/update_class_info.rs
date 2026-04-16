use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct UpdateClassInfo {
    /// Index into the ClassJob Excel sheet.
    pub class_id: u8,
    /// The level of the current class.
    #[brw(pad_before = 1)] // should be empty
    pub current_level: u16,
    /// Seems to always be identical to `current_level`.
    pub class_level: u16,
    /// If not zero, informs the client to display this as a "synced level" with an icon.
    pub synced_level: u16,
    /// Amount of gained EXP for the `current_level`.
    pub current_exp: i32,
    /// Amount of rested EXP.
    pub rested_exp: u32,
}
