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
