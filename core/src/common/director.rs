//! Content director related types.

use crate::common::{read_bool_from, write_bool_as};
use binrw::binrw;

// TODO: this may not apply to MassivePcContentDirector! Needs more research for that one.
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

    // Below are all of the events specific to InstanceContentDirector.
    // TODO: move out of this enum, and create one for PublicContentDirector etc.
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

    // Below is all of the events specific to ContentDirectors.
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
    /// Sets some field in ContentDirector, unsure how this is used.
    #[brw(magic = 0x80000002u32)]
    UnknownDirector2 { param: u32 },
    /// Sets some field in ContentDirector, unsure how this is used.
    #[brw(magic = 0x80000003u32)]
    UnknownDirector3 { param: u32 },
    /// Sets the remaining time in the duty. `arg` is the number of seconds.
    #[brw(magic = 0x80000004u32)]
    SetDutyTimeRemaining {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Sets some field in ContentDirector, unsure how this is used.
    #[brw(magic = 0x80000005u32)]
    UnknownDirector5 { param: u32 },
    /// Sets some field in ContentDirector, unsure how this is used.
    #[brw(magic = 0x80000006u32)]
    UnknownDirector6 { param: u32 },
    /// Does something duty action manager related.
    #[brw(magic = 0x80000007u32)]
    UnknownDirector7 {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Activates a new QTE event.
    #[brw(magic = 0x80000008u32)]
    QTEActivate { arg1: u32 },
    /// Deactivates the current QTE event.
    #[brw(magic = 0x80000009u32)]
    QTEDeactivate {},
    /// Does something related to QTEs.
    #[brw(magic = 0x8000000Au32)]
    UnkQTEA {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        arg1: bool,
    },
    /// Does something related to QTEs.
    #[brw(magic = 0x8000000Bu32)]
    UnkQTEB {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        arg1: bool,
    },
    /// Updates the content gauge.
    #[brw(magic = 0x8000000Cu32)]
    UpdateContentGauge {
        /// Index into the ContentGauge Excel sheet.
        content_gauge: u32,
        /// Progress of this gauge. From 0 to 10000.
        progress: u32,
        /// Read by the client, unsure what it does yet.
        unk1: u32,
    },
    /// ???
    #[brw(magic = 0x8000000Du32)]
    UnknownDirectorD { unk1: u32 },
    /// ???
    #[brw(magic = 0x8000000Eu32)]
    UnknownDirectorE { unk1: u32 },
    /// ???
    #[brw(magic = 0x8000000Fu32)]
    UnknownDirectorF { unk1: u32 },
    /// ???
    #[brw(magic = 0x80000010u32)]
    UnknownDirector10 {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// ???
    #[brw(magic = 0x80000011u32)]
    UnknownDirector11 {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// ???
    #[brw(magic = 0x80000012u32)]
    UnknownDirector12 { unk1: u32 },
    /// Implied in the disassembly, haven't tested it.
    #[brw(magic = 0x80000013u32)]
    ShowYesOrNo {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        arg1: bool,
    },
    /// ???
    #[brw(magic = 0x80000014u32)]
    UnknownDirector14 { unk1: u32 },
    /// ???
    #[brw(magic = 0x80000015u32)]
    UnknownDirector15 { unk1: u32 },
    /// ???
    #[brw(magic = 0x80000016u32)]
    UnknownDirector16 { unk1: u32 },
    /// Sound related?
    #[brw(magic = 0x80000017u32)]
    UnknownDirector17 { unk1: u32, unk2: u32 },
    /// Calls into GameMain?
    #[brw(magic = 0x80000018u32)]
    UnknownDirector18 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },
    /// Sets a field.
    #[brw(magic = 0x80000019u32)]
    UnknownDirector19 {},
    /// Plays a SGB timeline?
    #[brw(magic = 0x80000020u32)]
    UnknownDirector20 { arg1: u32, arg2: u32 },
    /// Sets a few fields.
    #[brw(magic = 0x80000021u32)]
    UnknownDirector21 { arg1: u32, arg2: u32 },
    /// Sets a few fields.
    #[brw(magic = 0x80000022u32)]
    UnknownDirector22 { arg1: u32 },
    /// Calls ShowTalkSubtitle?
    #[brw(magic = 0x80000023u32)]
    UnknownDirector23 { arg1: u32, arg2: u32 },
    /// Environment related?
    #[brw(magic = 0x80000024u32)]
    UnknownDirector24 { arg1: u32 },
    /// Manages a map effect?
    #[brw(magic = 0x80000025u32)]
    UnknownDirector25 { arg1: u32, arg2: u32 },
    /// Manages a map effect?
    #[brw(magic = 0x80000026u32)]
    UnknownDirector26 { arg1: u32, arg2: u32, arg3: u32 },
    /// At least used in The Merchant's Tale. First `arg` is the index into InstanceContextTextData.
    #[brw(magic = 0x80000027u32)]
    NpcYell {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
    /// Calls into RenderManager?
    #[brw(magic = 0x80000028u32)]
    UnknownDirector28 { arg1: u32, arg2: u32 },
    /// Calls into UIModule?
    #[brw(magic = 0x80000029u32)]
    UnknownDirector29 { arg1: u32, arg2: u32 },
    /// Calls into TargetSystem?
    #[brw(magic = 0x8000002Au32)]
    UnknownDirectorFocusTarget { arg1: u32, arg2: u32 },
    /// Calls into ContentsReplayManager.
    #[brw(magic = 0x8000002Bu32)]
    UnknownDirectorContentsReplayManager {},
    /// Executes a command (ClientTrigger.)
    #[brw(magic = 0x8000002Cu32)]
    UnknownDirectorExecuteCommand {},
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
