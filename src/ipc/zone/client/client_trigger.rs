use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{DistanceRange, ObjectId, read_bool_from, write_bool_as};

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, IntoStaticStr)]
pub enum ClientTriggerCommand {
    /// The player sheathes/unsheathes their weapon.
    #[brw(magic = 0x1u16)]
    ToggleWeapon {
        #[brw(pad_before = 2)]
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
    },
    /// The player looks or stops looking at an actor.
    #[brw(magic = 0x3u16)]
    SetTarget {
        #[brw(pad_before = 2)]
        actor_id: u32,
    },
    #[brw(magic = 0xC81u16)]
    BeginLoading {},
    #[brw(magic = 0xC9u16)]
    FinishZoning {},
    /// The player begins an emote.
    #[brw(magic = 0x1F4u16)]
    Emote {
        #[brw(pad_before = 2)] // padding
        emote: u32,
    },
    /// The player explicitly changed their pose.
    #[brw(magic = 0x1F9u16)]
    ChangePose {
        #[brw(pad_before = 2)] // padding
        unk1: u32,
        pose: u32,
    },
    /// The client is "reapplying" the existing pose, like after idling.
    #[brw(magic = 0x1FAu16)]
    ReapplyPose {
        #[brw(pad_before = 2)] // padding
        unk1: u32,
        pose: u32,
    },
    /// The player selects a teleport destination.
    #[brw(magic = 0xCAu16)]
    TeleportQuery {
        #[brw(pad_before = 2)]
        aetheryte_id: u32,
        // TODO: fill out the rest
    },
    #[brw(magic = 0x25eu16)]
    WalkInTriggerFinished {
        #[brw(pad_before = 2)]
        unk1: u32,
    },
    #[brw(magic = 0x033Eu16)]
    EventRelatedUnk {
        // seen in haircut event
        #[brw(pad_before = 2)] // padding
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },
    #[brw(magic = 0x12Fu16)]
    RequestTitleList {},
    /// When the player right-clicks their status effect to remove it.
    #[brw(magic = 0x68u16)]
    ManuallyRemoveEffect {
        #[brw(pad_before = 2)] // padding
        effect_id: u32,
        unk1: u32,
        source_actor_id: ObjectId,
    },
    /// When the player begins swimming. Seems to have no parameters.
    #[brw(magic = 0x260u16)]
    BeginSwimming {},
    /// When the player stops swimming (by going back on land, mounting, etc.). Seems to have no parameters.
    #[brw(magic = 0x261u16)]
    EndSwimming {},
    /// The client telling us how far in the distance we should see actors.
    #[brw(magic = 0x232Du16)]
    SetDistanceRange {
        #[brw(pad_before = 2)]
        range: DistanceRange,
    },
    Unknown {
        category: u16,
        // seen in haircut event
        #[brw(pad_before = 2)] // padding
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
            trigger: ClientTriggerCommand::SetTarget { actor_id: 0 },
        }
    }
}
