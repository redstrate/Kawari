use binrw::binrw;

use crate::CHAR_NAME_MAX_LENGTH;

use super::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CharacterDetails {
    #[brw(pad_after = 4)]
    pub id: u32,
    pub content_id: u64,
    #[brw(pad_after = 4)]
    pub index: u32,
    pub origin_server_id: u16,
    pub current_server_id: u16,
    pub unk1: [u8; 16],
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub origin_server_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub current_server_name: String,
    #[bw(pad_size_to = 1024)]
    #[br(count = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_detail_json: String,
    pub unk2: [u8; 20],
}
