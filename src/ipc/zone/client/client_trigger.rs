use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{DistanceRange, ObjectId, read_bool_from, write_bool_as};
use crate::ipc::zone::common_emote::CommonEmoteInfo;

#[binrw]
#[derive(Debug, PartialEq, Clone, IntoStaticStr)]
pub enum ClientTriggerCommand {
    /// The player sheathes/unsheathes their weapon.
    #[brw(magic = 0x0001u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
    },

    /// The player looks or stops looking at an actor.
    #[brw(magic = 0x0003u32)]
    SetTarget { actor_id: ObjectId },

    /// The client requests a minion to be summoned.
    #[brw(magic = 0x0066u32)]
    SummonMinion { minion_id: u32 },

    /// The client requests a minion to be despawned.
    #[brw(magic = 0x0067u32)]
    DespawnMinion {},

    /// When the player right-clicks their status effect to remove it.
    #[brw(magic = 0x0068u32)]
    ManuallyRemoveEffect {
        effect_id: u32,
        unk1: u32,
        source_actor_id: ObjectId,
    },

    /// The client is finished zoning.
    #[brw(magic = 0x00C9u32)]
    FinishZoning {},

    /// The player selects a teleport destination.
    #[brw(magic = 0x00CAu32)]
    TeleportQuery {
        aetheryte_id: u32,
        // TODO: fill out the rest
    },

    /// The client toggles a sign for their current target.
    #[brw(magic = 0x012Du32)]
    ToggleSign { id: u32 },

    /// The client sets a specific title.
    #[brw(magic = 0x012Eu32)]
    SetTitle { title_id: u32 },

    /// The client requests the player's unlocked titles.
    #[brw(magic = 0x012Fu32)]
    RequestTitleList {},

    /// The client clears all waymarks.
    #[brw(magic = 0x0139u32)]
    ClearAllWaymarks {},

    /// The client places a waymark.
    #[brw(magic = 0x013Du32)]
    PlaceWaymark {
        id: u32,

        // probably coordinates?
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },

    /// The client clears a waymark.
    #[brw(magic = 0x013Eu32)]
    ClearWaymark { id: u32 },

    /// The player begins an emote.
    #[brw(magic = 0x01F4u32)]
    Emote(CommonEmoteInfo),

    /// The player explicitly changed their pose.
    #[brw(magic = 0x01F9u32)]
    ChangePose { unk1: u32, pose: u32 },

    /// The client is "reapplying" the existing pose, like after idling.
    #[brw(magic = 0x01FAu32)]
    ReapplyPose { unk1: u32, pose: u32 },

    #[brw(magic = 0x025Eu32)]
    WalkInTriggerFinished { unk1: u32 },

    /// When the player begins swimming. Seems to have no parameters.
    #[brw(magic = 0x0260u32)]
    BeginSwimming {},

    /// When the player stops swimming (by going back on land, mounting, etc.). Seems to have no parameters.
    #[brw(magic = 0x0261u32)]
    EndSwimming {},

    /// When the player enters an area where mounting is prohibited in a zone that otherwise permits zoning. Commonly seen during Moonfire Faire festivals, and does not seem to have an exit counterpart.
    #[brw(magic = 0x0264u32)]
    EnterMountingProhibitedArea { enabled: u32 },

    /// Unknown purpose, but is seen during the crystal bell/aesthetician cutscenes.
    #[brw(magic = 0x033Eu32)]
    EventRelatedUnk {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// The client is ready to begin loading a zone.
    #[brw(magic = 0x0C81u32)]
    BeginLoading {},

    /// The client tells us how far in the distance we should see actors.
    #[brw(magic = 0x232Du32)]
    SetDistanceRange { range: DistanceRange },

    // Sent whenever the client tries to begin a Hall of the Novice exercise.
    #[brw(magic = 0x0802u32)]
    BeginNoviceExercise {
        id: u32, // not specific to a class/job
    },

    // Sent whenever the Glamour Plates window is opened or closed.
    #[brw(magic = 0x934u32)]
    ToggleGlamourPlatesWindow {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        open: bool,
    },

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
            },
        }
    }
}
