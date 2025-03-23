use binrw::binrw;

// See https://github.com/awgil/ffxiv_reverse/blob/f35b6226c1478234ca2b7149f82d251cffca2f56/vnetlog/vnetlog/ServerIPC.cs#L266 for a REALLY useful list of known values
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ActorControlCategory {
    #[brw(magic = 0x26u16)]
    ToggleInvisibility {
        #[brw(pad_before = 2)]
        invisible: u32, // FIXME: change to bool
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
            category: ActorControlCategory::ToggleInvisibility { invisible: 1 },
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
            category: ActorControlCategory::ToggleInvisibility { invisible: 1 },
        }
    }
}
