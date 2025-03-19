use binrw::binrw;

// See https://github.com/awgil/ffxiv_reverse/blob/f35b6226c1478234ca2b7149f82d251cffca2f56/vnetlog/vnetlog/ServerIPC.cs#L266 for a REALLY useful list of known values
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Default)]
#[brw(repr = u16)]
pub enum ActorControlCategory {
    #[default]
    ZoneIn = 0xC8,
    SetCharaGearParamUI = 0x260,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorControl {
    #[brw(pad_after = 2)]
    pub category: ActorControlCategory,
    pub param1: u32,
    pub param2: u32,
    pub param3: u32,
    #[brw(pad_after = 4)] // maybe not always empty?
    pub param4: u32,
}
