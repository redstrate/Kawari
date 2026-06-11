//! Content director related types.

use binrw::binrw;

/// Events are sent by the server (who is acting as the director) to change state.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorEvent {
    /// Changes the festival phases for Ocean Fishing, but probably used for other things.
    /// In Ocean Fishing, seen with params of 13 and 23 (IKDRoute + 1 and something else unknown.)
    #[brw(magic = 2u32)]
    ChangeFestivalPhases,
    /// Shows the Ocean Fishing scoring window, but probably used for other things.
    /// In Ocean Fishing, seen with a param of 19 (IKDRoute probably.)
    #[brw(magic = 3u32)]
    ShowOceanFishingWindow,
    /// Shows the Variant Dungeon vote window, but probably used for other things.
    /// For Variant Dungeons, the first `arg` is how many votes are needed and the second `arg` is what the NPC chose. Has no effect if there is no route associated with the duty (Another Merchant's Tale.)
    #[brw(magic = 0x10000002u32)]
    VariantVoteRoute,
    /// Hides the vote window, but probably used for other things.
    #[brw(magic = 0x10000004u32)]
    HideVariantVoteRoute,
    /// Shows "Duty Commenced", and starts the clock ticking down. `arg` is the number of seconds the duty should last.
    #[brw(magic = 0x40000001u32)]
    DutyCommence,
    /// `arg` is unknown.
    #[brw(magic = 0x40000002u32)]
    DutyCompletedFlyText,
    /// `arg` is unknown.
    #[brw(magic = 0x40000003u32)]
    DutyCompleted,
    /// `arg` is unknown.
    #[brw(magic = 0x40000005u32)]
    PartyWipe,
    /// `arg` is unknown.
    #[brw(magic = 0x40000006u32)]
    DutyRecommence,
    /// Shows "one or more party members have yet to complete this duty" message along with the rewards.
    #[brw(magic = 0x4000000Cu32)]
    DutyFirstTimeCompletionNotice,
    /// Seems to be in response to the Sync trigger. Arg seems to always be 1.
    #[brw(magic = 0x80000000u32)]
    SyncResponse,
    /// Sets the BGM to what's specified in Arg. Index into the BGM Excel sheet.
    #[brw(magic = 0x80000001u32)]
    SetBGM,
    /// Sets the remaining time in the duty. `arg` is the number of seconds.
    #[brw(magic = 0x80000004u32)]
    SetDutyTimeRemaining,
    /// At least used in The Merchant's Tale. First `arg` is the index into InstanceContextTextData.
    #[brw(magic = 0x80000027u32)]
    NpcYell,
    Unknown(u32),
}

/// Triggers are sent by clients to inform the director of their actions.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorTrigger {
    /// Seen while GATEs were spawning.
    #[brw(magic = 0u32)]
    GoldSaucerUnk1,
    /// Seen while GATEs were spawning.
    #[brw(magic = 1u32)]
    GoldSaucerUnk2,
    /// Seen when voting in a Variant Dungeon, but probably used for other things.
    /// For Variant Dungeons, the first `arg` is the route chosen by this player.
    #[brw(magic = 0x10000002u32)]
    VariantVote,
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
