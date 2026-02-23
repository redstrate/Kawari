use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveQuest {
    pub id: u16,
    #[brw(pad_after = 1)] // padding
    pub sequence: u8,
    pub flags: u8,
    #[brw(pad_after = 5)] // padding
    pub bitflags: [u8; 6],
}

impl ActiveQuest {
    pub const SIZE: usize = 16;
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct QuestActiveList {
    #[br(count = 30)]
    #[brw(pad_size_to = 30 * ActiveQuest::SIZE)]
    pub quests: Vec<ActiveQuest>,
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn active_quest_size() {
        ensure_size::<ActiveQuest, { ActiveQuest::SIZE }>();
    }
}
