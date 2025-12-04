use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct RetainerInfo {
    id: u64,
    owner_id: u64,
    slot_id: u8,
    param1: u8,
    status: u16,
    param2: u32,
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    name: String,
}

impl RetainerInfo {
    pub const SIZE: usize = 56;
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct DistRetainerInfo {
    pub sequence: u64,
    pub timestamp: u32,
    pub index: u8,
    pub count: u8,
    pub option_param: u16,
    pub option_arg: u32,
    pub num_contracted: u16,
    pub num_active: u16,
    pub num_total: u16,
    pub num_free_slots: u16,
    pub total_retainers: u16,
    pub active_retainers: u16,
    #[br(count = 9)]
    #[brw(pad_size_to = (9 * RetainerInfo::SIZE))]
    pub characters: Vec<RetainerInfo>,
}
