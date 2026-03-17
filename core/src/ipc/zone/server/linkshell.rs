use binrw::binrw;

use crate::ipc::zone::server::{CHAR_NAME_MAX_LENGTH, ChatChannel, read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct LinkshellEntry {
    pub linkshell_id: u64,
    pub chatchannel_id: ChatChannel,
    pub unk1: u32,
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 4)]
    pub linkshell_name: String,
}

impl LinkshellEntry {
    pub const SIZE: usize = 56;
    pub const COUNT: usize = 8;
}
