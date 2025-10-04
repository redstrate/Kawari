use binrw::binrw;
use paramacro::opcode_data;

pub use super::chara_make::{CharaMake, LobbyCharacterActionKind};

use crate::{
    common::{read_string, write_string},
    opcodes::ClientLobbyIpcType,
    packet::{IpcSegment, ServerlessIpcSegmentHeader},
};

pub type ClientLobbyIpcSegment = IpcSegment<
    ServerlessIpcSegmentHeader<ClientLobbyIpcType>,
    ClientLobbyIpcType,
    ClientLobbyIpcData,
>;

#[opcode_data(ClientLobbyIpcType)]
#[binrw]
#[br(import(magic: &ClientLobbyIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientLobbyIpcData {
    ServiceLogin {
        sequence: u64,
        account_index: u8,
        unk1: u8,
        unk2: u16,
        unk3: u32, // TODO: probably multiple params
        account_id: u32,
        unk4: u32,
    },
    GameLogin {
        sequence: u64,
        content_id: u64,
        // TODO: what else is in here?
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },
    LoginEx {
        sequence: u64,
        timestamp: u32,
        #[brw(pad_after = 2)]
        unk1: u32,
        #[br(count = 64)]
        #[bw(pad_size_to = 64)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        session_id: String,

        #[br(count = 144)]
        #[bw(pad_size_to = 144)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        version_info: String,

        #[brw(pad_before = 910)] // empty
        unk2: u64,
    },
    ShandaLogin {
        #[bw(pad_size_to = 1456)]
        #[br(count = 1456)]
        unk: Vec<u8>,
    },
    CharaMake(CharaMake),
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

impl Default for ClientLobbyIpcData {
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

    use crate::{packet::IpcSegmentHeader, packet::ReadWriteIpcSegment};

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_lobby_ipc_sizes() {
        let ipc_types = [
            (
                ClientLobbyIpcType::ServiceLogin,
                ClientLobbyIpcData::ServiceLogin {
                    sequence: 0,
                    account_index: 0,
                    unk1: 0,
                    unk2: 0,
                    account_id: 0,
                    unk3: 0,
                    unk4: 0,
                },
            ),
            (
                ClientLobbyIpcType::GameLogin,
                ClientLobbyIpcData::GameLogin {
                    sequence: 0,
                    content_id: 0,
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                },
            ),
            (
                ClientLobbyIpcType::LoginEx,
                ClientLobbyIpcData::LoginEx {
                    sequence: 0,
                    session_id: String::default(),
                    version_info: String::default(),
                    unk1: 0,
                    timestamp: 0,
                    unk2: 0,
                },
            ),
            (
                ClientLobbyIpcType::ShandaLogin,
                ClientLobbyIpcData::ShandaLogin {
                    unk: Vec::default(),
                },
            ),
            (
                ClientLobbyIpcType::CharaMake,
                ClientLobbyIpcData::CharaMake(CharaMake::default()),
            ),
        ];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ClientLobbyIpcSegment {
                header: IpcSegmentHeader::from_opcode(opcode.clone()),
                data: ipc.clone(),
                ..Default::default()
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{:#?} did not match size!",
                opcode
            );
        }
    }
}
