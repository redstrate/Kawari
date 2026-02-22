//! Content director related types.

use binrw::binrw;

#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorEvent {
    /// Shows "Duty Commenced", and starts the clock ticking down. `arg` is the number of seconds the duty should last.
    #[brw(magic = 0x40000001u32)]
    DutyCommence,
    /// Seems to be in response to the Sync trigger. Arg seems to always be 1.
    #[brw(magic = 0x80000000u32)]
    SyncResponse,
    /// Sets the BGM to what's specified in Arg. Index into the BGM Excel sheet.
    #[brw(magic = 0x80000001u32)]
    SetBGM,
    /// Sets the remaining time in the duty. `arg` is the number of seconds.
    #[brw(magic = 0x80000004u32)]
    SetDutyTimeRemaining,
    Unknown(u32),
}

#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorTrigger {
    /// When the player finishes the cutscene, I think. `arg` is 174 for Sastasha, I don't know what that means.'
    #[brw(magic = 0x40000001u32)]
    FinishedCutscene,
    /// When the player requests to summon a striking dummy. `arg` always seems to be 1.
    #[brw(magic = 0x40000006u32)]
    SummonStrikingDummy,
    /// Unknown purpose.
    #[brw(magic = 0x80000000u32)]
    Sync,
    Unknown(u32),
}
