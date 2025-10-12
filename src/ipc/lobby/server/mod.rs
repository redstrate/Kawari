use binrw::binrw;
use paramacro::opcode_data;

pub mod dist_retainer_info;
pub mod login_reply;
pub mod nack_reply;
pub mod server_list;
pub mod service_login_reply;

pub use crate::ipc::lobby::chara_make::LobbyCharacterActionKind;

pub use service_login_reply::{CharacterDetails, CharacterFlag, ServiceLoginReply};

pub use server_list::{DistWorldInfo, Server};

pub use login_reply::{LoginReply, ServiceAccount};

pub use dist_retainer_info::{DistRetainerInfo, RetainerInfo};

pub use nack_reply::NackReply;

use crate::{
    common::{read_string, write_string},
    opcodes::ServerLobbyIpcType,
    packet::{IpcSegment, ServerlessIpcSegmentHeader},
};

pub type ServerLobbyIpcSegment = IpcSegment<
    ServerlessIpcSegmentHeader<ServerLobbyIpcType>,
    ServerLobbyIpcType,
    ServerLobbyIpcData,
>;

#[opcode_data(ServerLobbyIpcType)]
#[binrw]
#[br(import(magic: &ServerLobbyIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ServerLobbyIpcData {
    NackReply(NackReply),
    LoginReply(LoginReply),
    ServiceLoginReply(ServiceLoginReply),
    CharaMakeReply {
        sequence: u64,
        unk1: u8,
        unk2: u8,
        #[brw(pad_after = 1)] // empty
        action: LobbyCharacterActionKind,
        #[brw(pad_before = 36)] // empty
        #[brw(pad_after = 1336)] // empty and garbage
        details: CharacterDetails,
    },
    GameLoginReply {
        sequence: u64,
        actor_id: u32,
        #[brw(pad_before = 4)]
        content_id: u64,
        #[brw(pad_before = 4)]
        #[bw(pad_size_to = 66)]
        #[br(count = 66)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        token: String, // WHAT IS THIS FOR??
        port: u16,
        #[brw(pad_after = 16)] // garbage?
        #[br(count = 48)]
        #[brw(pad_size_to = 48)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        host: String,
    },
    DistWorldInfo(DistWorldInfo),
    DistRetainerInfo(DistRetainerInfo),
    XiCharacterInfo {
        unk: [u8; 496],
    },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

impl Default for ServerLobbyIpcData {
    fn default() -> Self {
        Self::Unknown {
            unk: Vec::default(),
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
    fn server_lobby_ipc_sizes() {
        let ipc_types = [
            ServerLobbyIpcData::NackReply(NackReply::default()),
            ServerLobbyIpcData::LoginReply(LoginReply::default()),
            ServerLobbyIpcData::ServiceLoginReply(ServiceLoginReply::default()),
            ServerLobbyIpcData::CharaMakeReply {
                sequence: 0,
                unk1: 0,
                unk2: 0,
                action: LobbyCharacterActionKind::ReserveName,
                details: CharacterDetails::default(),
            },
            ServerLobbyIpcData::GameLoginReply {
                sequence: 0,
                actor_id: 0,
                content_id: 0,
                token: String::new(),
                port: 0,
                host: String::new(),
            },
            ServerLobbyIpcData::DistWorldInfo(DistWorldInfo::default()),
            ServerLobbyIpcData::DistRetainerInfo(DistRetainerInfo::default()),
            ServerLobbyIpcData::XiCharacterInfo { unk: [0; 496] },
        ];

        for data in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let opcode: ServerLobbyIpcType = ReadWriteIpcOpcode::from_data(data);
            let ipc_segment = ServerLobbyIpcSegment {
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
