use binrw::binrw;

use crate::common::{read_bool_from, write_bool_as};

use super::OnlineStatus;

// TODO: these are all somewhat related, but maybe should be separated?

// See https://github.com/awgil/ffxiv_reverse/blob/f35b6226c1478234ca2b7149f82d251cffca2f56/vnetlog/vnetlog/ServerIPC.cs#L266 for a REALLY useful list of known values
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ActorControlCategory {
    #[brw(magic = 0x0u16)]
    ToggleWeapon {
        #[brw(pad_before = 2)]
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
    },
    #[brw(magic = 0x26u16)]
    ToggleInvisibility {
        #[brw(pad_before = 2)]
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        invisible: bool,
    },
    #[brw(magic = 0xC8u16)]
    ZoneIn {
        #[brw(pad_before = 2)]
        warp_finish_anim: u32,
        raise_anim: u32,
    },
    #[brw(magic = 0x260u16)]
    SetCharaGearParamUI {
        #[brw(pad_before = 2)]
        unk1: u32,
        unk2: u32,
    },
    #[brw(magic = 0x01F8u16)]
    SetStatusIcon {
        #[brw(pad_before = 2)]
        icon: OnlineStatus,
    },
    #[brw(magic = 0x261u16)]
    ToggleWireframeRendering(),
    #[brw(magic = 0x32u16)]
    SetTarget {
        #[brw(pad_before = 22)] // actually full of info, and 2 bytes of padding at the beginning
        actor_id: u32,
    },
    #[brw(magic = 0x127u16)]
    Pose {
        #[brw(pad_before = 2)] //padding
        unk1: u32,
        pose: u32,
    },
    #[brw(magic = 0x1FDu16)]
    LearnTeleport {
        #[brw(pad_before = 2)] //padding
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },
    #[brw(magic = 0x29u16)]
    ToggleUnlock {
        /// Corresponds to an UnlockLink. Could be a spell, action, emote, etc.
        // See https://github.com/Haselnussbomber/HaselDebug/blob/main/HaselDebug/Tabs/UnlocksTabs/UnlockLinks/UnlockLinksTable.cs
        #[brw(pad_before = 2)] //padding
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },
    #[brw(magic = 0xCBu16)]
    TeleportStart {
        #[brw(pad_before = 2)] //padding
        insufficient_gil: u32,
        aetheryte_id: u32,
    },
    #[brw(magic = 0x1Bu16)]
    Flee {
        #[brw(pad_before = 2)] // padding
        speed: u16,
    },
    #[brw(magic = 0x386u16)]
    SetFestival {
        #[brw(pad_before = 2)] // padding
        festival1: u32, // Multiple festivals can be set at the same time.
        festival2: u32,
        festival3: u32,
        festival4: u32,
    },
    #[brw(magic = 0xFu16)]
    CancelCast {},
    #[brw(magic = 0x396u16)]
    ToggleOrchestrionUnlock {
        #[brw(pad_before = 2)] // padding
        song_id: u16,
        /* TODO: guessed, Sapphire suggests it's an u32 item id,
        * but it behaves as an unlock boolean like aetherytes, so
        perhaps it's been repurposed since Shadowbringers. */
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },
    #[brw(magic = 0x122u16)]
    Emote {
        #[brw(pad_before = 2)] // padding
        emote: u32,
    },
    #[brw(magic = 0x1ffu16)]
    EventRelatedUnk1 {
        #[brw(pad_before = 2)] // padding
        unk1: u32,
    },
    #[brw(magic = 0x200u16)]
    EventRelatedUnk2 {
        #[brw(pad_before = 2)] // padding
        unk1: u32,
    },
    Unknown {
        category: u16,
        #[brw(pad_before = 2)] // padding
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
    #[brw(pad_size_to = 28)] // take into account categories without params
    pub category: ActorControlCategory,
}

impl Default for ActorControlTarget {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}
