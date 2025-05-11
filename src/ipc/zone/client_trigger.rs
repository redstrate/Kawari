use binrw::binrw;

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ClientTriggerCommand {
    #[brw(magic = 0x3u16)]
    SetTarget {
        #[brw(pad_before = 2)]
        actor_id: u32,
    },
    #[brw(magic = 0xC81u16)]
    Unk1 {},
    #[brw(magic = 0xC9u16)]
    Unk2 {},
    #[brw(magic = 0x1F9u16)]
    ChangePose {
        #[brw(pad_before = 2)] // padding
        unk1: u32,
        pose: u32,
    },
    #[brw(magic = 0x1FAu16)]
    ReapplyPose {
        #[brw(pad_before = 2)] // padding
        unk1: u32,
        pose: u32,
    },
    #[brw(magic = 0xCAu16)]
    TeleportQuery {
        #[brw(pad_before = 2)]
        aetheryte_id: u32,
        // TODO: fill out the rest
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
