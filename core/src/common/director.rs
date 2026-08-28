//! Content director related types.

use crate::common::{HandlerId, HandlerType, read_bool_from, write_bool_as};
use binrw::binrw;

/// Used by descendants of the `Client::Game::MassivePcContent::MassivePcContentDirector` class.
///
/// For the client implementation, see `Client::Game::MassivePcContent::MassivePcContentDirector.ProcessCommonDirectorUpdate`.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MassivePcContentEvent {
    /// Temporary placeholder!
    #[brw(magic = 0x80000000u32)]
    Unknown1 {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
}

/// Used by descendants of the `Client::Game::InstanceContent::ContentDirector` class.
///
/// For the client implementation, see `Client::Game::InstanceContent::ContentDirector.ProcessCommonDirectorUpdate`.
#[binrw]
#[br(import(handler_id: HandlerId))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContentDirectorEvent {
    /// Sets the current background music.
    #[brw(magic = 0x80000001u32)]
    SetBGM {
        /// Index into the BGM Excel sheet.
        bgm: u32,
    },
    /// Sets some field in ContentDirector, unsure how or when this is used.
    #[brw(magic = 0x80000002u32)]
    UnknownDirector2 { param: u32 },
    /// Sets the time left in ContentDirector, unsure how or when this is used.
    #[brw(magic = 0x80000003u32)]
    UnknownDirector3 { param: u32 },
    /// Sets the remaining time in this duty instance.
    #[brw(magic = 0x80000004u32)]
    SetDutyTimeRemaining {
        /// In seconds.
        time_remaining: u32,
    },
    /// Sets some field in ContentDirector, unsure how this is used.
    #[brw(magic = 0x80000005u32)]
    UnknownDirector5 { param: u32 },
    /// Sets some field in ContentDirector, unsure how this is used.
    #[brw(magic = 0x80000006u32)]
    UnknownDirector6 { param: u32 },
    /// Does something duty action manager related.
    #[brw(magic = 0x80000007u32)]
    UnknownDirector7 { arg1: u32 },
    /// Activates a new QTE event.
    #[brw(magic = 0x80000008u32)]
    QTEActivate { arg1: u32 },
    /// Deactivates the current QTE event.
    #[brw(magic = 0x80000009u32)]
    QTEDeactivate,
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
    UnknownDirector19,
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
    /// Used in The Merchant's Tale.
    #[brw(magic = 0x80000027u32)]
    NpcYell {
        /// Index into the InstanceContentTextData sheet.
        text_data: u32,
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
    UnknownDirectorContentsReplayManager,
    /// Sends a DirectorTrigger of unknown purpose.
    #[brw(magic = 0x8000002Cu32)]
    UnknownDirectorExecuteCommand,

    #[br(pre_assert(handler_id.handler_type() == HandlerType::InstanceContent))]
    InstanceContent(InstanceContentDirectorEvent),
}

/// Updates handled by `Client::Game::InstanceContent::InstanceContentDirector` class.
///
/// For the client implementation, see `Client::Game::InstanceContent::InstanceContentDirector.ProcessDirectorSpecificDirectorUpdate`.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstanceContentDirectorEvent {
    /// Shows "Duty Commenced", and starts the clock ticking down.
    #[brw(magic = 0x40000001u32)]
    DutyCommence {
        /// In seconds.
        time_limit: u32,
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
    /// Sets some array in InstanceContentDirector?
    #[brw(magic = 0x40000007u32)]
    Unknown7 { arg1: u32, arg2: u32 },
    /// Shows "one or more party members have yet to complete this duty" message along with the rewards.
    #[brw(magic = 0x4000000Cu32)]
    DutyFirstTimeCompletionNotice {
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
}

/// Updates handled by `Client::Game::Gimmick::GimmickEventHandler`.
/// While this is technically generic across Gimmick types, it's only been seen for GimmickRect so far.
///
/// For the client implementation, see `Client::Game::Gimmick::GimmickEventHandler.ProcessEventSpecificDirectorUpdate`.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GimmickRectEvent {
    /// (Assumed) when this is sent, the GimmickRect is set to active on the client-side.
    /// This is most commonly used for entrances in dungeons.
    #[brw(magic = 0x80000000u32)]
    Activate {
        /// Retail technically sends a 1 here, but I don't think it's read by the client.
        arg1: u32,
    },
}

/// Events are sent by the server (who is acting as the director) to change state.
#[binrw]
#[br(import(handler_id: HandlerId))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorEvent {
    /// Event for GimmickRect handlers.
    #[br(pre_assert(handler_id.handler_type() == HandlerType::GimmickRect))]
    GimmickRect(GimmickRectEvent),
    /// Event for handlers backed by ContentDirector.
    #[br(pre_assert(handler_id.handler_type().is_content_director()))]
    ContentDirector(#[br(args(handler_id))] ContentDirectorEvent),
    /// Event for MassivePcContent handlers.
    #[br(pre_assert(handler_id.handler_type() == HandlerType::MassivePcContent))]
    MassivePcContentDirector(MassivePcContentEvent),
    /// Unknown event.
    Unknown {
        id: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        arg4: u32,
    },
}

/// Used by descendants of the `Client::Game::InstanceContent::ContentDirector` class.
#[binrw]
#[br(import(handler_id: HandlerId))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ContentDirectorTrigger {
    #[br(pre_assert(handler_id.handler_type() == HandlerType::InstanceContent))]
    InstanceContent(InstanceContentDirectorTrigger),
}

/// Updates handled by `Client::Game::InstanceContent::InstanceContentDirector` class.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InstanceContentDirectorTrigger {
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
}

/// Updates handled by `Client::Game::Gimmick::GimmickEventHandler`.
/// While this is technically generic across Gimmick types, it's only been seen for GimmickRect so far.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum GimmickRectTrigger {
    /// (Assumed) that the client is checking whether this GimmickRect is active.
    #[brw(magic = 0x80000000u32)]
    CheckIfActive,
}

/// Used by descendants of the `Client::Game::MassivePcContent::MassivePcContentDirector` class.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MassivePcContentTrigger {
    /// Temporary placeholder.
    #[brw(magic = 0x80000000u32)]
    Unknown1 { arg1: u32, arg2: u32, arg3: u32 },
}

/// Triggers are sent by the client to inform the director of various actions it took.
#[binrw]
#[br(import(handler_id: HandlerId))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DirectorTrigger {
    /// Trigger for GimmickRect handlers.
    #[br(pre_assert(handler_id.handler_type() == HandlerType::GimmickRect))]
    GimmickRect(GimmickRectTrigger),
    /// Trigger for handlers backed by ContentDirector.
    #[br(pre_assert(handler_id.handler_type().is_content_director()))]
    ContentDirector(#[br(args(handler_id))] ContentDirectorTrigger),
    /// Trigger for MassivePcContent handlers.
    #[br(pre_assert(handler_id.handler_type() == HandlerType::MassivePcContent))]
    MassivePcContentDirector(MassivePcContentTrigger),
    /// Unknown trigger.
    Unknown {
        id: u32,
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },
}
