use binrw::binrw;

use crate::common::{read_bool_from, write_bool_as};

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct TrackedQuest {
    /// Whether this quest is active (or shows up in the list.)
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub active: bool,
    /// The index of this quest into the active quest list.
    pub quest_index: u8,
}

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct QuestTracker {
    #[brw(pad_after = 14)]
    pub tracked_quests: [TrackedQuest; 5],
}
