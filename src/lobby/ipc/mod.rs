use binrw::binrw;

mod character_action;
pub use character_action::{LobbyCharacterAction, LobbyCharacterActionKind};

mod character_list;
pub use character_list::{CharacterDetails, CharacterFlag, LobbyCharacterList};

mod client_version_info;

mod server_list;
pub use server_list::{LobbyServerList, Server};

mod service_account_list;
pub use service_account_list::{LobbyServiceAccountList, ServiceAccount};

use crate::{
    common::{read_string, write_string},
    opcodes::{ClientLobbyIpcType, ServerLobbyIpcType},
    packet::{IpcSegment, ReadWriteIpcSegment},
};

pub type ClientLobbyIpcSegment = IpcSegment<ClientLobbyIpcType, ClientLobbyIpcData>;

impl ReadWriteIpcSegment for ClientLobbyIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + self.op_code.calc_size()
    }
}

// TODO: make generic
impl Default for ClientLobbyIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientLobbyIpcType::ClientVersionInfo,
            server_id: 0,
            timestamp: 0,
            data: ClientLobbyIpcData::ClientVersionInfo {
                sequence: 0,
                session_id: String::new(),
                version_info: String::new(),
            },
        }
    }
}

pub type ServerLobbyIpcSegment = IpcSegment<ServerLobbyIpcType, ServerLobbyIpcData>;

impl ReadWriteIpcSegment for ServerLobbyIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + self.op_code.calc_size()
    }
}

// TODO: make generic
impl Default for ServerLobbyIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ServerLobbyIpcType::LobbyError,
            server_id: 0,
            timestamp: 0,
            data: ServerLobbyIpcData::LobbyError {
                sequence: 0,
                error: 0,
                value: 0,
                exd_error_id: 0,
                unk1: 0,
            },
        }
    }
}

#[binrw]
#[br(import(magic: &ClientLobbyIpcType))]
#[derive(Debug, Clone)]
pub enum ClientLobbyIpcData {
    /// Sent by the client after exchanging encryption information with the lobby server.
    #[br(pre_assert(*magic == ClientLobbyIpcType::ClientVersionInfo))]
    ClientVersionInfo {
        sequence: u64,

        #[brw(pad_before = 10)] // full of nonsense i don't understand yet
        #[br(count = 64)]
        #[br(map = read_string)]
        #[bw(ignore)]
        session_id: String,

        #[brw(pad_before = 8)] // empty
        #[br(count = 128)]
        #[br(map = read_string)]
        #[bw(ignore)]
        version_info: String,
        // unknown stuff at the end, it's not completely empty
    },
    /// Sent by the client when it requests the character list in the lobby.
    #[br(pre_assert(*magic == ClientLobbyIpcType::RequestCharacterList))]
    RequestCharacterList {
        #[brw(pad_before = 16)]
        sequence: u64,
        // TODO: what is in here?
    },
    /// Sent by the client when they request something about the character (e.g. deletion.)
    #[br(pre_assert(*magic == ClientLobbyIpcType::LobbyCharacterAction))]
    LobbyCharacterAction(LobbyCharacterAction),
    /// Sent by the client when it requests to enter a world.
    #[br(pre_assert(*magic == ClientLobbyIpcType::RequestEnterWorld))]
    RequestEnterWorld {
        sequence: u64,
        content_id: u64,
        // TODO: what else is in here?
    },
}

#[binrw]
#[br(import(_magic: &ServerLobbyIpcType))]
#[derive(Debug, Clone)]
pub enum ServerLobbyIpcData {
    /// Sent by the server to inform the client of their service accounts.
    LobbyServiceAccountList(LobbyServiceAccountList),
    /// Sent by the server to inform the client of their servers.
    LobbyServerList(LobbyServerList),
    /// Sent by the server to inform the client of their retainers.
    LobbyRetainerList {
        // TODO: what is in here?
        #[brw(pad_before = 7)]
        #[brw(pad_after = 202)]
        unk1: u8,
    },
    /// Sent by the server to inform the client of their characters.
    LobbyCharacterList(LobbyCharacterList),
    /// Sent by the server to tell the client how to connect to the world server.
    LobbyEnterWorld {
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
    /// Sent by the server to indicate an lobby error occured.
    LobbyError {
        sequence: u64,
        error: u32,
        value: u32,
        exd_error_id: u16,
        #[brw(pad_after = 516)] // empty and garbage
        unk1: u16,
    },
    // Assumed what this is, but probably incorrect
    CharacterCreated {
        sequence: u64,
        unk1: u8,
        unk2: u8,
        #[brw(pad_after = 1)] // empty
        action: LobbyCharacterActionKind,
        #[brw(pad_before = 36)] // empty
        #[brw(pad_after = 1336)] // empty and garbage
        details: CharacterDetails,
    },
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn lobby_ipc_sizes() {
        let ipc_types = [
            (
                ServerLobbyIpcType::LobbyServiceAccountList,
                ServerLobbyIpcData::LobbyServiceAccountList(LobbyServiceAccountList::default()),
            ),
            (
                ServerLobbyIpcType::LobbyServerList,
                ServerLobbyIpcData::LobbyServerList(LobbyServerList::default()),
            ),
            (
                ServerLobbyIpcType::LobbyRetainerList,
                ServerLobbyIpcData::LobbyRetainerList { unk1: 0 },
            ),
            (
                ServerLobbyIpcType::LobbyCharacterList,
                ServerLobbyIpcData::LobbyCharacterList(LobbyCharacterList::default()),
            ),
            (
                ServerLobbyIpcType::LobbyEnterWorld,
                ServerLobbyIpcData::LobbyEnterWorld {
                    sequence: 0,
                    actor_id: 0,
                    content_id: 0,
                    token: String::new(),
                    port: 0,
                    host: String::new(),
                },
            ),
            (
                ServerLobbyIpcType::LobbyError,
                ServerLobbyIpcData::LobbyError {
                    sequence: 0,
                    error: 0,
                    value: 0,
                    exd_error_id: 0,
                    unk1: 0,
                },
            ),
            (
                ServerLobbyIpcType::CharacterCreated,
                ServerLobbyIpcData::CharacterCreated {
                    sequence: 0,
                    unk1: 0,
                    unk2: 0,
                    action: LobbyCharacterActionKind::ReserveName,
                    details: CharacterDetails::default(),
                },
            ),
        ];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerLobbyIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: opcode.clone(),
                server_id: 0,
                timestamp: 0,
                data: ipc.clone(),
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
