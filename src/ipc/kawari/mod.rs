use binrw::binrw;
use paramacro::opcode_data;

use crate::{
    common::{CHAR_NAME_MAX_LENGTH, read_bool_from, read_string, write_bool_as, write_string},
    ipc::lobby::CharacterDetails,
    opcodes::CustomIpcType,
    packet::{IpcSegment, ServerlessIpcSegmentHeader},
};

pub type CustomIpcSegment =
    IpcSegment<ServerlessIpcSegmentHeader<CustomIpcType>, CustomIpcType, CustomIpcData>;

#[opcode_data(CustomIpcType)]
#[binrw]
#[br(import(magic: &CustomIpcType, _size: &u32))]
#[derive(Debug, Clone)]
pub enum CustomIpcData {
    RequestCreateCharacter {
        service_account_id: u64,
        #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
        #[bw(pad_size_to = 1024)]
        #[br(count = 1024)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        chara_make_json: String,
    },
    CharacterCreated {
        actor_id: u32,
        content_id: u64,
    },
    GetActorId {
        content_id: u64,
    },
    ActorIdFound {
        actor_id: u32,
    },
    CheckNameIsAvailable {
        #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
    },
    NameIsAvailableResponse {
        #[br(map = read_bool_from::<u8>)]
        #[bw(map = write_bool_as::<u8>)]
        free: bool,
    },
    RequestCharacterList {
        service_account_id: u64,
    },
    RequestCharacterListRepsonse {
        #[bw(calc = characters.len() as u8)]
        num_characters: u8,
        #[br(count = num_characters)]
        #[brw(pad_size_to = 1184 * 8)]
        characters: Vec<CharacterDetails>, // TODO: maybe chunk this into 4 parts ala the lobby server?
    },
    DeleteCharacter {
        content_id: u64,
    },
    CharacterDeleted {
        deleted: u8,
    },
    ImportCharacter {
        service_account_id: u64,
        #[bw(pad_size_to = 128)]
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        path: String,
    },
    RemakeCharacter {
        content_id: u64,
        #[bw(pad_size_to = 1024)]
        #[br(count = 1024)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        chara_make_json: String,
    },
    CharacterRemade {
        content_id: u64,
    },
    CharacterImported {
        #[bw(pad_size_to = 128)]
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        message: String,
    },
    DeleteServiceAccount {
        service_account_id: u64,
    },
    RequestFullCharacterList {},
    FullCharacterListResponse {
        #[bw(pad_size_to = 1024)]
        #[br(count = 1024)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        json: String,
    },
    Unknown,
}

impl Default for CustomIpcData {
    fn default() -> CustomIpcData {
        CustomIpcData::RequestCreateCharacter {
            service_account_id: 0,
            chara_make_json: String::new(),
            name: String::new(),
        }
    }
}
