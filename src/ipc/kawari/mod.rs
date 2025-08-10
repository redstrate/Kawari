use binrw::binrw;

use crate::{
    common::{CHAR_NAME_MAX_LENGTH, read_bool_from, read_string, write_bool_as, write_string},
    ipc::lobby::CharacterDetails,
    packet::{IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcOpcode, ReadWriteIpcSegment},
};

pub type CustomIpcSegment = IpcSegment<CustomIpcType, CustomIpcData>;

impl ReadWriteIpcSegment for CustomIpcSegment {
    fn calc_size(&self) -> u32 {
        IPC_HEADER_SIZE
            + match self.op_code {
                CustomIpcType::RequestCreateCharacter => 1024 + CHAR_NAME_MAX_LENGTH as u32,
                CustomIpcType::CharacterCreated => 12,
                CustomIpcType::GetActorId => 8,
                CustomIpcType::ActorIdFound => 4,
                CustomIpcType::CheckNameIsAvailable => CHAR_NAME_MAX_LENGTH as u32,
                CustomIpcType::NameIsAvailableResponse => 1,
                CustomIpcType::RequestCharacterList => 4,
                CustomIpcType::RequestCharacterListRepsonse => 1 + (1184 * 8),
                CustomIpcType::DeleteCharacter => 8,
                CustomIpcType::CharacterDeleted => 1,
                CustomIpcType::ImportCharacter => 132,
                CustomIpcType::RemakeCharacter => 1024 + 8,
                CustomIpcType::CharacterRemade => 8,
            }
    }

    fn get_name(&self) -> &'static str {
        ""
    }

    fn get_opcode(&self) -> u16 {
        todo!()
    }
}

#[binrw]
#[brw(repr = u16)]
#[derive(Default, Clone, PartialEq, Debug)]
pub enum CustomIpcType {
    #[default]
    /// Request the world server to create a character
    RequestCreateCharacter = 0x1,
    /// Response from the world server when the character is created
    CharacterCreated = 0x2,
    /// Request the actor id from the content id of a character
    GetActorId = 0x3,
    /// Response from the world server when the actor id is found
    ActorIdFound = 0x4,
    /// Check if a name is available on the world server
    CheckNameIsAvailable = 0x5,
    /// Response to CheckNameIsAvailable
    NameIsAvailableResponse = 0x6,
    /// Request the character list from the world server
    RequestCharacterList = 0x7,
    /// Response to RequestCharacterList
    RequestCharacterListRepsonse = 0x8,
    /// Request that a character be deleted from the world server
    DeleteCharacter = 0x9,
    /// Response to DeleteCharacter
    CharacterDeleted = 0x10,
    /// Request to import a character backup
    ImportCharacter = 0x11,
    /// Remake a character
    RemakeCharacter = 0x12,
    // Character has been remade
    CharacterRemade = 0x13,
}

impl ReadWriteIpcOpcode<CustomIpcData> for CustomIpcType {
    fn from_data(_: &CustomIpcData) -> Self {
        todo!()
    }
}

#[binrw]
#[br(import(magic: &CustomIpcType, _size: &u32))]
#[derive(Debug, Clone)]
pub enum CustomIpcData {
    #[br(pre_assert(*magic == CustomIpcType::RequestCreateCharacter))]
    RequestCreateCharacter {
        service_account_id: u32,
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
    #[br(pre_assert(*magic == CustomIpcType::CharacterCreated))]
    CharacterCreated { actor_id: u32, content_id: u64 },
    #[br(pre_assert(*magic == CustomIpcType::GetActorId))]
    GetActorId { content_id: u64 },
    #[br(pre_assert(*magic == CustomIpcType::ActorIdFound))]
    ActorIdFound { actor_id: u32 },
    #[br(pre_assert(*magic == CustomIpcType::CheckNameIsAvailable))]
    CheckNameIsAvailable {
        #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        name: String,
    },
    #[br(pre_assert(*magic == CustomIpcType::NameIsAvailableResponse))]
    NameIsAvailableResponse {
        #[br(map = read_bool_from::<u8>)]
        #[bw(map = write_bool_as::<u8>)]
        free: bool,
    },
    #[br(pre_assert(*magic == CustomIpcType::RequestCharacterList))]
    RequestCharacterList { service_account_id: u32 },
    #[br(pre_assert(*magic == CustomIpcType::RequestCharacterListRepsonse))]
    RequestCharacterListRepsonse {
        #[bw(calc = characters.len() as u8)]
        num_characters: u8,
        #[br(count = num_characters)]
        #[brw(pad_size_to = 1184 * 8)]
        characters: Vec<CharacterDetails>, // TODO: maybe chunk this into 4 parts ala the lobby server?
    },
    #[br(pre_assert(*magic == CustomIpcType::DeleteCharacter))]
    DeleteCharacter { content_id: u64 },
    #[br(pre_assert(*magic == CustomIpcType::CharacterDeleted))]
    CharacterDeleted { deleted: u8 },
    #[br(pre_assert(*magic == CustomIpcType::ImportCharacter))]
    ImportCharacter {
        service_account_id: u32,
        #[bw(pad_size_to = 128)]
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        path: String,
    },
    #[br(pre_assert(*magic == CustomIpcType::RemakeCharacter))]
    RemakeCharacter {
        content_id: u64,
        #[bw(pad_size_to = 1024)]
        #[br(count = 1024)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        chara_make_json: String,
    },
    #[br(pre_assert(*magic == CustomIpcType::CharacterRemade))]
    CharacterRemade { content_id: u64 },
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
