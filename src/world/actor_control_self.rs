use binrw::binrw;

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Default)]
#[brw(repr = u16)]
pub enum ActorControlType {
    #[default]
    SetCharaGearParamUI = 0x260,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorControlSelf {
    #[brw(pad_after = 2)]
    pub category: ActorControlType,
    pub param1: u32,
    pub param2: u32,
    pub param3: u32,
    pub param4: u32,
    pub param5: u32,
    #[brw(pad_after = 4)]
    pub param6: u32,
}
