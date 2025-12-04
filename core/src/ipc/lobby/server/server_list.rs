use binrw::binrw;

use crate::common::{read_bool_from, read_string, write_bool_as, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Server {
    pub id: u16,
    pub index: u16,
    /// Whether this World is restricted to new characters.
    #[br(map = read_bool_from::<u32>)]
    #[bw(map = write_bool_as::<u32>)]
    pub restricted: bool,
    /// Whether this World indicates it gives an EXP Bonus.
    #[brw(pad_before = 4)]
    #[brw(pad_after = 4)]
    #[br(map = read_bool_from::<u32>)]
    #[bw(map = write_bool_as::<u32>)]
    pub exp_bonus: bool,
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
