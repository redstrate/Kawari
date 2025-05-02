use binrw::binrw;

use crate::common::CHAR_NAME_MAX_LENGTH;

use super::{read_string, write_string};

#[binrw]
#[derive(Clone, PartialEq, Debug)]
pub enum LobbyCharacterActionKind {
    #[brw(magic = 0x1u8)]
    ReserveName,
    #[brw(magic = 0x2u8)]
    Create,
    #[brw(magic = 0x3u8)]
    Rename,
    #[brw(magic = 0x4u8)]
    Delete,
    #[brw(magic = 0x5u8)]
    Move,
    #[brw(magic = 0x6u8)]
    RemakeRetainer,
    #[brw(magic = 0x7u8)]
    RemakeChara,
    #[brw(magic = 0x8u8)]
    SettingsUploadBegin,
    #[brw(magic = 0xCu8)]
    SettingsUpload,
    #[brw(magic = 0xEu8)]
    WorldVisit,
    #[brw(magic = 0xFu8)]
    DataCenterToken,
    #[brw(magic = 0x15u8)]
    Request,
}

#[binrw]
#[derive(Clone, PartialEq, Debug)]
pub struct LobbyCharacterAction {
    pub sequence: u64,
    pub content_id: u64,
    #[br(pad_before = 8)]
    pub character_index: u8,
    pub action: LobbyCharacterActionKind,
    pub world_id: u16,
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    #[bw(pad_size_to = 436)]
    #[br(count = 436)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub json: String,
}
