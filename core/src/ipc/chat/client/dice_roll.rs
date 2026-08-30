use crate::ipc::chat::ChatChannel;
use binrw::binrw;

#[binrw]
#[derive(Clone, Copy, Debug, Default)]
pub struct ChatDiceRollData {
    #[brw(pad_after = 7)] // Seemingly empty/padding
    pub unk1: u8, // Always 1?
    /// The destination ChatChannel this dice roll should be broadcasted to.
    pub community_id: ChatChannel,
    #[brw(pad_after = 7)] // Seemingly empty/padding
    pub unk2: u8, // Always 3?
    #[brw(pad_after = 7)] // Seemingly empty/padding
    pub unk3: u8, // Might be an index of some sort, but unsure
    /// The number of sides on the die.
    #[brw(pad_after = 14)] // Seemingly empty/padding
    pub num_sides: u16,
}
