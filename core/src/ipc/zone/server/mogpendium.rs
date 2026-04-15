use binrw::binrw;
use bitflags::bitflags;

// Represents the progress of the *current* mogpendium
#[binrw]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mogpendium {
    unk1: u32,
    /// Always 0x18, Possible discriminator
    unk2: u32,
    /// Always 0x20, Possible discriminator
    unk3: u32,
    /// Unique and stable per character, observed in range of 1810000000 - 3000000000
    /// Seems to be somewhat related to the owning character `content_id`, which exposes which server the character was originally created on
    /// Eg. characters created on any US datacenter have ranges of 3000000000, while characters created on any EU datacenter have ranges of 1810000000.
    /// This remains visible even when the character was transferred between datacenter regions years ago.
    /// My initial thought was that these 4 bytes represent each weekly objective,
    /// but they show no identical weekly challenge between characters that have the same byte at the same offset.
    id_or_seed: u32,
    /// Completion flags for weekly objectives
    weekly_objective_flags: MogpendiumCompletionFlags,
    unk4: [u8; 16],
    /// Completion flags for minimog challenges
    minimog_challenge_flags: MogpendiumCompletionFlags,
    /// TODO: The progress count for the ultimog challenge is somewhere in there, I just don't care about treasure dungeons :(
    unk5: [u8; 36],
    /// Holds the progress count for the first minimog objective of the *current* week
    minimog_objective_1_progress: u32,
    /// Holds the progress count for the second minimog objective of the *current* week
    minimog_objective_2_progress: u32,
    unk6: [u8; 60],
}

impl Default for Mogpendium {
    fn default() -> Self {
        Self {
            unk1: Default::default(),
            unk2: Default::default(),
            unk3: Default::default(),
            id_or_seed: Default::default(),
            weekly_objective_flags: Default::default(),
            unk4: Default::default(),
            minimog_challenge_flags: Default::default(),
            unk5: [0; 36],
            minimog_objective_1_progress: Default::default(),
            minimog_objective_2_progress: Default::default(),
            unk6: [0; 60],
        }
    }
}

/// Represents the completion state for a complete set of Mogpendium challenges.
/// If a bitfield is `0`, it is either expired (past week) or incomplete (current week).
#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MogpendiumCompletionFlags(u32);

bitflags! {
    impl MogpendiumCompletionFlags: u32 {
        const NONE = 0;
        const W1_COMPLETE = 1 << 0;
        const W1_CLAIMED  = 1 << 1;
        const W2_COMPLETE = 1 << 2;
        const W2_CLAIMED  = 1 << 3;
        const W3_COMPLETE = 1 << 4;
        const W3_CLAIMED  = 1 << 5;
        const W4_COMPLETE = 1 << 6;
        const W4_CLAIMED  = 1 << 7;
    }
}

impl std::fmt::Debug for MogpendiumCompletionFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

impl Default for MogpendiumCompletionFlags {
    fn default() -> Self {
        MogpendiumCompletionFlags::NONE
    }
}
