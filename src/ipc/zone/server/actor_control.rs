use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{EquipDisplayFlag, ObjectId, ObjectTypeId, read_bool_from, write_bool_as};
use crate::ipc::zone::common_emote::CommonEmoteInfo;

use super::OnlineStatus;

// TODO: these are all somewhat related, but maybe should be separated?

// See https://github.com/awgil/ffxiv_reverse/blob/f35b6226c1478234ca2b7149f82d251cffca2f56/vnetlog/vnetlog/ServerIPC.cs#L266 for a REALLY useful list of known values
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, IntoStaticStr)]
pub enum ActorControlCategory {
    #[brw(magic = 0u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        /// This seems to always be set to 1. If set to another value, the animation glitches for other clients.
        unk_flag: u32,
    },

    #[brw(magic = 15u32)]
    CancelCast {},

    #[brw(magic = 17u32)]
    Cooldown { unk1: u32, unk2: u32, unk3: u32 },

    #[brw(magic = 20u32)]
    GainEffect {
        effect_id: u32,
        param: u32,
        source_actor_id: ObjectId,
    },

    #[brw(magic = 21u32)]
    LoseEffect {
        effect_id: u32,
        unk2: u32,
        source_actor_id: ObjectId,
    },

    #[brw(magic = 24u32)]
    UpdateRestedExp { exp: u32 },

    #[brw(magic = 27u32)]
    Flee { speed: u16 },

    #[brw(magic = 38u32)]
    ToggleInvisibility {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        invisible: bool,
    },

    #[brw(magic = 41u32)]
    ToggleUnlock {
        /// Corresponds to an UnlockLink. Could be a spell, action, emote, etc.
        // See https://github.com/Haselnussbomber/HaselDebug/blob/main/HaselDebug/Tabs/UnlocksTabs/UnlockLinks/UnlockLinksTable.cs
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 50u32)]
    SetTarget {
        #[brw(pad_before = 20)] // Blank since there are no params in the ACT
        target: ObjectTypeId,
    },

    #[brw(magic = 131u32)]
    UnlockInstanceContent {
        /// Index into InstanceContent Excel sheet
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 138u32)]
    EventRelatedUnk3 { event_id: u32 },

    #[brw(magic = 200u32)]
    ZoneIn {
        /// When set to 1, it slowly fades the character in.
        warp_finish_anim: u32,
        /// When set to 1, it plays the effects *and* animation for raising.
        raise_anim: u32,
        /// If set to 110, then it plays the "we teleported here" animation.
        #[brw(pad_before = 4)] // empty
        unk1: u32,
    },

    #[brw(magic = 203u32)]
    TeleportStart {
        insufficient_gil: u32,
        aetheryte_id: u32,
    },

    #[brw(magic = 236u32)]
    WalkInTriggerRelatedUnk3 { unk1: u32 },

    #[brw(magic = 253u32)]
    CompanionUnlock {
        unk1: u32,
        unk2: u32, // unlocked?
    },

    #[brw(magic = 260u32)]
    WalkInTriggerRelatedUnk2 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        /// Usually 7?
        unk4: u32,
    },

    #[brw(magic = 270u32)]
    MinionSpawnControl {
        /// When set to 0, the player's minion is despawned.
        minion_id: u32,
    },

    #[brw(magic = 274u32)]
    UpdateHater { unk1: u32 },

    #[brw(magic = 295u32)]
    Pose { unk1: u32, pose: u32 },

    #[brw(magic = 263u32)]
    WalkInTriggerRelatedUnk1 { unk1: u32 },

    #[brw(magic = 290u32)]
    Emote(CommonEmoteInfo),

    #[brw(magic = 378u32)]
    PlayerCurrency { unk1: u32, unk2: u32, unk3: u32 },

    #[brw(magic = 504u32)]
    SetStatusIcon { icon: OnlineStatus },

    #[brw(magic = 509u32)]
    LearnTeleport {
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 511u32)]
    EventRelatedUnk1 { unk1: u32 },

    #[brw(magic = 512u32)]
    EventRelatedUnk2 { unk1: u32 },

    #[brw(magic = 521u32)]
    SetItemLevel { level: u32 },

    #[brw(magic = 608u32)]
    SetEquipDisplayFlags { display_flag: EquipDisplayFlag },

    #[brw(magic = 609u32)]
    ToggleWireframeRendering(),

    #[brw(magic = 902u32)]
    SetFestival {
        festival1: u32, // Multiple festivals can be set at the same time.
        festival2: u32,
        festival3: u32,
        festival4: u32,
    },

    #[brw(magic = 918u32)]
    ToggleOrchestrionUnlock {
        song_id: u16,
        /* TODO: guessed, Sapphire suggests it's an u32 item id,
        * but it behaves as an unlock boolean like aetherytes, so
        perhaps it's been repurposed since Shadowbringers. */
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    Unknown {
        category: u32,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct ActorControl {
    #[brw(pad_after = 4)]
    #[brw(pad_size_to = 20)] // take into account categories without params
    pub category: ActorControlCategory,
}

impl Default for ActorControl {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}

// Has more padding than ActorControl?
#[binrw]
#[derive(Debug, Clone)]
pub struct ActorControlSelf {
    #[brw(pad_after = 12)]
    #[brw(pad_size_to = 20)] // take into account categories without params
    pub category: ActorControlCategory,
}

impl Default for ActorControlSelf {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}

// Has more padding than ActorControl?
#[binrw]
#[derive(Debug, Clone)]
pub struct ActorControlTarget {
    #[brw(pad_size_to = 32)]
    // take into account categories without params, TODO: investigate if the last 4 bytes are padding or a possible 7th param
    pub category: ActorControlCategory,
}

impl Default for ActorControlTarget {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}
