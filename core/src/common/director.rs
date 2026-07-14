//! Content director related types.

use binrw::binrw;

/// Events are sent by the server (who is acting as the director) to change state.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorEvent {
    /// Changes the festival phases for Ocean Fishing, but probably used for other things.
    /// In Ocean Fishing, seen with params of 13 and 23 (IKDRoute + 1 and something else unknown.)
    #[brw(magic = 2u32)]
    ChangeFestivalPhases {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Shows the Ocean Fishing scoring window, but probably used for other things.
    /// In Ocean Fishing, seen with a param of 19 (IKDRoute probably.)
    #[brw(magic = 3u32)]
    ShowOceanFishingWindow {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Shows the Variant Dungeon vote window, but probably used for other things.
    #[brw(magic = 0x10000002u32)]
    VariantVoteRoute {
        /// For Variant Dungeons, how many votes are needed
        votes_needed: u32,
        /// For Variant Dungeons, what route the NPC chose.
        npc_route: u32,
    },
    /// Hides the vote window, but probably used for other things.
    #[brw(magic = 0x10000004u32)]
    HideVariantVoteRoute,
    /// Shows "Duty Commenced", and starts the clock ticking down. `arg` is the number of seconds the duty should last.
    #[brw(magic = 0x40000001u32)]
    DutyCommence {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// `arg` is unknown.
    #[brw(magic = 0x40000002u32)]
    DutyCompletedFlyText {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// `arg` is unknown.
    #[brw(magic = 0x40000003u32)]
    DutyCompleted {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// `arg` is unknown.
    #[brw(magic = 0x40000005u32)]
    PartyWipe {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// `arg` is unknown.
    #[brw(magic = 0x40000006u32)]
    DutyRecommence {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Shows "one or more party members have yet to complete this duty" message along with the rewards.
    #[brw(magic = 0x4000000Cu32)]
    DutyFirstTimeCompletionNotice {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Seems to be in response to the Sync trigger. Arg seems to always be 1.
    #[brw(magic = 0x80000000u32)]
    SyncResponse {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Sets the current background music.
    #[brw(magic = 0x80000001u32)]
    SetBGM {
        /// Index into the BGM Excel sheet.
        bgm: u32,
    },
    /// Sets the remaining time in the duty. `arg` is the number of seconds.
    #[brw(magic = 0x80000004u32)]
    SetDutyTimeRemaining {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Updates the content gauge.
    #[brw(magic = 0x8000000Cu32)]
    UpdateContentGauge {
        /// Index into the ContentGauge Excel sheet.
        content_gauge: u32,
        /// Progress of this gauge. From 0 to 10000.
        progress: u32,
    },
    /// At least used in The Merchant's Tale. First `arg` is the index into InstanceContextTextData.
    #[brw(magic = 0x80000027u32)]
    NpcYell {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    Unknown {
        id: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
}

/// Triggers are sent by clients to inform the director of their actions.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorTrigger {
    /// Seen while GATEs were spawning.
    #[brw(magic = 0u32)]
    GoldSaucerUnk1 { unk1: u32, unk2: u32, unk3: u32 },
    /// Seen while GATEs were spawning.
    #[brw(magic = 1u32)]
    GoldSaucerUnk2 { unk1: u32, unk2: u32, unk3: u32 },
    /// Seen when voting in a Variant Dungeon, but probably used for other things.
    #[brw(magic = 0x10000002u32)]
    VariantVote {
        /// For Variant Dungeons, the first `arg` is the route chosen by this player.
        route: u32,
    },
    /// When the player finishes the cutscene, I think.
    #[brw(magic = 0x40000001u32)]
    FinishedCutscene {
        /// Is 174 for Sastasha, I don't know what that means.
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },
    /// When the player requests to summon a striking dummy. `arg` always seems to be 1.
    #[brw(magic = 0x40000006u32)]
    SummonStrikingDummy { unk1: u32, unk2: u32, unk3: u32 },
    /// Unknown purpose.
    #[brw(magic = 0x80000000u32)]
    Sync { unk1: u32, unk2: u32, unk3: u32 },
    Unknown {
        id: u32,
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },
}
