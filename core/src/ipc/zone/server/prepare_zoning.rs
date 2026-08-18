use binrw::binrw;
use bitflags::bitflags;

use crate::common::WarpType;

#[binrw]
#[derive(Clone, Copy, Eq, PartialEq, Default)]
pub struct PrepareZoningFlag(u8);

bitflags! {
    impl PrepareZoningFlag: u8 {
        /// If set, the text that usually indicates the territory name is not shown.
        const HIDE_TERRITORY_NAME = 0x1;
        const UNK2 = 0x2; // Seen while teleporting, water->air. Also during resurrections.
        const UNK4 = 0x4; // Seen while going from portal->water, water->air
        /// If set, the companion does not play a visible and loud despawn animation.
        const PRESERVE_COMPANION = 0x8;
        const UNK16 = 0x10;
    }
}

impl std::fmt::Debug for PrepareZoningFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[derive(Debug, Clone, Default)]
#[binrw]
pub struct PrepareZoning {
    /// If non-zero, prints this log message to the chat. Index into the LogMessage Excel sheet.
    pub log_message: u32,
    /// Affects what's displayed on the loading screen. Index into the TerritoryType Excel sheet.
    pub territory_type_id: u16,
    /// If non-zero, begins playing this VFX. Index into the VFX Excel sheet.
    pub vfx_id: u16,
    /// If non-zero, uses this VFX as the loading screen background. Index into the VFX Excel sheet.
    pub loading_screen_vfx_id: u16,
    /// Must match what is used in ActorSetPos (if applicable) otherwise weird stuff like EnterTerritoryEvent is sent by the client again.
    pub warp_type: WarpType,
    /// If set to one, the character is hidden.
    /// This is not a boolean because technically there is a "third" mode for values 1, >2 but I'm not sure what they do or if they're even used by retail.
    /// Mode 2 is used by resurrections.
    pub hide_character: u8,
    /// Seems to always be set to 1, but mostly unused by the client. If set to 0xFF (255) then the screen never fades out.
    pub fade_out_delay: u8,
    /// Miscellaneous flags.
    #[brw(pad_after = 2)] // not read by the client it seems
    pub flags: PrepareZoningFlag,
}
