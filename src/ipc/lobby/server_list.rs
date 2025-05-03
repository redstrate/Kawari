use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Server {
    pub id: u16,
    pub index: u16,
    pub flags: u32,
    #[brw(pad_before = 4)]
    #[brw(pad_after = 4)]
    pub icon: u32,
    #[bw(pad_size_to = 64)]
    #[br(count = 64)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct DistWorldInfo {
    pub sequence: u64,
    pub unk1: u16,
    pub offset: u16,
    #[brw(pad_after = 8)]
    pub num_servers: u32,
    #[br(count = 6)]
    #[brw(pad_size_to = 504)]
    pub servers: Vec<Server>,
}
