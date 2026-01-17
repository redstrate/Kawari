use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{
    DirectorTrigger, DistanceRange, HandlerId, ObjectId, read_bool_from, write_bool_as,
};
use crate::ipc::zone::common_emote::CommonEmoteInfo;

#[binrw]
#[derive(Debug, PartialEq, Clone, IntoStaticStr)]
pub enum ClientTriggerCommand {
    /// The player sheathes/unsheathes their weapon.
    #[brw(magic = 1u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        unk_flag: u32,
    },

    /// When toggling auto-attack on and off.
    #[brw(magic = 2u32)]
    ToggleAutoAttack {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        on: bool,
        unk1: u32,
        unk3: u32,
    },

    /// The player looks or stops looking at an actor.
    #[brw(magic = 3u32)]
    SetTarget {
        actor_id: ObjectId,
        /// The client sends value(s) that differ from ObjectTypeKind, so we cannot use it here.
        /// It differs for minions, which the client sends as 2, while 4 is used elsewhere (see ObjectTypeKind).
        actor_type: u32,
    },

    /// The client is trying to dismount their current mount.
    #[brw(magic = 101u32)]
    Dismount { sequence: u32 },

    /// The client requests a minion to be summoned.
    #[brw(magic = 102u32)]
    SummonMinion { minion_id: u32 },

    /// The client requests a minion to be despawned.
    #[brw(magic = 103u32)]
    DespawnMinion {},

    /// When the player right-clicks their status effect to remove it.
    #[brw(magic = 104u32)]
    ManuallyRemoveEffect {
        effect_id: u32,
        effect_param: u32,
        source_actor_id: ObjectId,
    },

    /// The client is finished zoning.
    #[brw(magic = 201u32)]
    FinishZoning {},

    /// The player selects a teleport destination.
    #[brw(magic = 202u32)]
    TeleportQuery {
        aetheryte_id: u32,
        // TODO: fill out the rest
    },

    /// The client toggles a sign for their current target.
    #[brw(magic = 301u32)]
    ToggleSign { id: u32 },

    /// The client sets a specific title.
    #[brw(magic = 302u32)]
    SetTitle { title_id: u32 },

    /// The client requests the player's unlocked titles.
    #[brw(magic = 303u32)]
    RequestTitleList {},

    /// The client shows an Active Help pop-up. This is triggered *when* it's shown, not when closed.
    #[brw(magic = 306u32)]
    ShownActiveHelp { id: u32 },

    /// The client has seen this cutscene. This is usually followed up by a ToggleCutsceneSeen ACS.
    #[brw(magic = 307u32)]
    SeenCutscene { id: u32 },

    /// The client clears all waymarks.
    #[brw(magic = 313u32)]
    ClearAllWaymarks {},

    /// The client begins using the Idle Camera or Group Pose feature.
    #[brw(magic = 314u32)]
    GroupPoseOrIdleCamera { unk1: u32, unk2: u32 },

    /// The client places a waymark.
    #[brw(magic = 317u32)]
    PlaceWaymark {
        id: u32,

        // probably coordinates?
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },

    /// The client clears a waymark.
    #[brw(magic = 318u32)]
    ClearWaymark { id: u32 },

    /// The client requests materia melding from another player.
    #[brw(magic = 413u32)]
    RequestMateriaMeld { actor_id: ObjectId },

    #[brw(magic = 444u32)]
    OpenChocoboSaddlebag {},

    /// The client requests repair from another player.
    #[brw(magic = 450u32)]
    RequestRepair { actor_id: ObjectId },

    /// The player begins an emote.
    #[brw(magic = 500u32)]
    Emote(CommonEmoteInfo),

    /// The player interrupts their emote.
    #[brw(magic = 503u32)]
    EmoteInterrupted {},

    /// The player explicitly changed their pose.
    #[brw(magic = 505u32)]
    ChangePose { unk1: u32, pose: u32 },

    /// The client is "reapplying" the existing pose, like after idling.
    #[brw(magic = 506u32)]
    ReapplyPose { unk1: u32, pose: u32 },

    #[brw(magic = 602u32)]
    GimmickJumpLanded {
        /// Index into the GimmickJump Excel sheet.
        gimmick_jump_type: u32,
    },

    #[brw(magic = 606u32)]
    WalkInTriggerFinished { unk1: u32 },

    /// When the player begins swimming. Seems to have no parameters.
    #[brw(magic = 608u32)]
    BeginSwimming {},

    /// When the player stops swimming (by going back on land, mounting, etc.). Seems to have no parameters.
    #[brw(magic = 609u32)]
    EndSwimming {},

    /// When the player enters an area where mounting is prohibited in a zone that otherwise permits zoning. Commonly seen during Moonfire Faire festivals, and does not seem to have an exit counterpart.
    #[brw(magic = 612u32)]
    EnterMountingProhibitedArea { enabled: u32 },

    /// When the player starts flying on their mount.
    #[brw(magic = 616u32)]
    StartFlying {},

    /// Various triggers related to instanced content.
    #[brw(magic = 808u32)]
    DirectorTrigger {
        handler_id: HandlerId,
        trigger: DirectorTrigger,
        arg: u32,
    },

    /// When a player requests an NPC (or player?) to a Triple Triad match.
    #[brw(magic = 815u32)]
    TripleTriadChallenge {
        actor_id: u32, // probably
    },

    /// When a player requests to abandon their instanced content.
    #[brw(magic = 819u32)]
    AbandonContent { unk1: u32 },

    /// Unknown purpose, but is seen during the crystal bell/aesthetician cutscenes.
    #[brw(magic = 830u32)]
    EventRelatedUnk {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 1107u32)]
    RequestHousingWardInfo {
        /// The zone id of the housing ward.
        zone_id: u32,
        /// 0-based index, so ward number - 1.
        ward_index: u32,
    },

    /// The client requests to repair and item at a mender.
    #[brw(magic = 1600u32)]
    RepairItem { unk1: u32, unk2: u32, unk3: u32 },

    /// The client opens the General tab in the Gold Saucer window.
    #[brw(magic = 1850u32)]
    OpenGoldSaucerGeneralTab {},

    /// The client opens the Chocobo tab in the Gold Saucer window.
    #[brw(magic = 1850u32)]
    OpenGoldSaucerChocoboTab {},

    /// The client is ready to begin loading a zone.
    #[brw(magic = 3201u32)]
    BeginLoading {},

    #[brw(magic = 1980u32)]
    BeginContentsReplay {},

    #[brw(magic = 1981u32)]
    EndContentsReplay {},

    /// Sent whenever the client tries to begin a Hall of the Novice exercise.
    #[brw(magic = 2050u32)]
    BeginNoviceExercise {
        id: u32, // not specific to a class/job
    },

    /// Sent whenever the client requests to duel another player.
    #[brw(magic = 2200u32)]
    RequestDuel {
        /// Whom to duel with.
        actor_id: ObjectId,
    },

    /// Sent whenever the Glamour Plates window is opened or closed.
    #[brw(magic = 2356u32)]
    ToggleGlamourPlatesWindow {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        open: bool,
    },

    /// The client opens the Mahjong tab in the Gold Saucer window.
    #[brw(magic = 2550u32)]
    OpenGoldSaucerMahjongTab {},

    /// The client opens the Trust window.
    #[brw(magic = 2651u32)]
    OpenTrustWindow {},

    /// The client opens the Duty Support window.
    #[brw(magic = 2653u32)]
    OpenDutySupportWindow {},

    /// The client opens the Portrait window.
    #[brw(magic = 3200u32)]
    OpenPortraitsWindow {},

    /// The client opens the Mogpendium.
    #[brw(magic = 9003u32)]
    OpenMogpendium { unk1: u32, unk2: u32 },

    /// The client tells us how far in the distance we should see actors.
    #[brw(magic = 9005u32)]
    SetDistanceRange { range: DistanceRange },

    Unknown {
        category: u32,
        // seen in haircut event
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct ClientTrigger {
    #[brw(pad_size_to = 32)] // take into account categories without params
    pub trigger: ClientTriggerCommand,
}

impl Default for ClientTrigger {
    fn default() -> Self {
        Self {
            trigger: ClientTriggerCommand::SetTarget {
                actor_id: ObjectId::default(),
                actor_type: 0,
            },
        }
    }
}
