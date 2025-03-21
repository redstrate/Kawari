use binrw::binrw;

use crate::CHAR_NAME_MAX_LENGTH;

use super::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CharacterDetails {
    #[brw(pad_after = 4)]
    pub actor_id: u32,
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

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct LobbyCharacterList {
    pub sequence: u64,
    pub counter: u8,
    #[brw(pad_after = 2)]
    pub num_in_packet: u8,
    pub unk1: u8,
    pub unk2: u8,
    pub unk3: u8,
    /// Set to 128 if legacy character
    pub unk4: u8,
    pub unk5: [u32; 7],
    pub unk6: u8,
    pub veteran_rank: u8,
    #[brw(pad_after = 1)]
    pub unk7: u8,
    pub days_subscribed: u32,
    pub remaining_days: u32,
    pub days_to_next_rank: u32,
    pub max_characters_on_world: u16,
    pub unk8: u16,
    #[brw(pad_after = 12)]
    pub entitled_expansion: u32,
    #[br(count = 2)]
    #[brw(pad_size_to = (1196 * 2))]
    pub characters: Vec<CharacterDetails>,
}
