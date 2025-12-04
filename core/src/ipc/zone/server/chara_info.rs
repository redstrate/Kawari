use crate::common::{CHAR_NAME_MAX_LENGTH, read_string, write_string};
use binrw::binrw;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CharaInfoFromContentIdsData {
    #[brw(pad_before = 8)] // empty
    pub content_id: u64,
    pub home_world_id: u16,
    pub current_world_id: u16,
    pub unk: u16,
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 2)] // empty
    pub character_name: String,
}

impl CharaInfoFromContentIdsData {
    pub const SIZE: usize = 56;
}
