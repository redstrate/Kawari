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
    RequestCharacterListResponse {
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use crate::packet::{IpcSegmentHeader, ReadWriteIpcOpcode, ReadWriteIpcSegment};

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn custom_ipc_sizes() {
        let ipc_types = [
            CustomIpcData::RequestCreateCharacter {
                service_account_id: 0,
                name: "".to_string(),
                chara_make_json: "".to_string(),
            },
            CustomIpcData::CharacterCreated {
                actor_id: 0,
                content_id: 0,
            },
            CustomIpcData::GetActorId {
                content_id: 0,
            },
            CustomIpcData::ActorIdFound {
                actor_id: 0,
            },
            CustomIpcData::CheckNameIsAvailable {
                name: "".to_string(),
            },
            CustomIpcData::NameIsAvailableResponse {
                free: false,
            },
            CustomIpcData::RequestCharacterList {
                service_account_id: 0,
            },
            CustomIpcData::RequestCharacterListResponse {
                characters: vec![CharacterDetails::default(); 8],
            },
            CustomIpcData::DeleteCharacter {
                content_id: 0,
            },
            CustomIpcData::CharacterDeleted {
                deleted: 0,
            },
            CustomIpcData::ImportCharacter {
                service_account_id: 0,
                path: "".to_string(),
            },
            CustomIpcData::RemakeCharacter {
                content_id: 0,
                chara_make_json: "".to_string(),
            },
            CustomIpcData::CharacterRemade {
                content_id: 0,
            },
            CustomIpcData::CharacterImported {
                message: "".to_string(),
            },
            CustomIpcData::DeleteServiceAccount {
                service_account_id: 0,
            },
            CustomIpcData::RequestFullCharacterList {}, // Included here for completeness despite the 0 size
            CustomIpcData::FullCharacterListResponse {
                json: "".to_string(),
            },
        ];

        for data in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let opcode: CustomIpcType = ReadWriteIpcOpcode::from_data(data);
            let ipc_segment = CustomIpcSegment {
                header: IpcSegmentHeader::from_opcode(opcode.clone()),
                data: data.clone(),
                ..Default::default()
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{opcode:#?} did not match size!"
            );
        }
    }
}
