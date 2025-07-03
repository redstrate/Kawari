use binrw::binrw;

mod chara_make;
pub use chara_make::{CharaMake, LobbyCharacterActionKind};

mod service_login_reply;
pub use service_login_reply::{CharacterDetails, CharacterFlag, ServiceLoginReply};

mod server_list;
pub use server_list::{DistWorldInfo, Server};

mod login_reply;
pub use login_reply::{LoginReply, ServiceAccount};

use crate::{
    common::{read_string, write_string},
    opcodes::{ClientLobbyIpcType, ServerLobbyIpcType},
    packet::{IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcSegment},
};

pub type ClientLobbyIpcSegment = IpcSegment<ClientLobbyIpcType, ClientLobbyIpcData>;

impl ReadWriteIpcSegment for ClientLobbyIpcSegment {
    fn calc_size(&self) -> u32 {
        IPC_HEADER_SIZE + self.op_code.calc_size()
    }

    fn get_name(&self) -> &'static str {
        self.op_code.get_name()
    }

    fn get_opcode(&self) -> u16 {
        self.op_code.get_opcode()
    }
}

// TODO: make generic
impl Default for ClientLobbyIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientLobbyIpcType::LoginEx,
            option: 0,
            timestamp: 0,
            data: ClientLobbyIpcData::LoginEx {
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
        IPC_HEADER_SIZE + self.op_code.calc_size()
    }

    fn get_name(&self) -> &'static str {
        self.op_code.get_name()
    }

    fn get_opcode(&self) -> u16 {
        self.op_code.get_opcode()
    }
}

// TODO: make generic
impl Default for ServerLobbyIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ServerLobbyIpcType::NackReply,
            option: 0,
            timestamp: 0,
            data: ServerLobbyIpcData::NackReply {
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
#[br(import(magic: &ClientLobbyIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientLobbyIpcData {
    /// Sent by the client when it requests the character list in the lobby.
    #[br(pre_assert(*magic == ClientLobbyIpcType::ServiceLogin))]
    ServiceLogin {
        #[brw(pad_before = 16)]
        sequence: u64,
        // TODO: what is in here?
    },
    /// Sent by the client when it requests to enter a world.
    #[br(pre_assert(*magic == ClientLobbyIpcType::GameLogin))]
    GameLogin {
        sequence: u64,
        content_id: u64,
        // TODO: what else is in here?
    },
    /// Sent by the client after exchanging encryption information with the lobby server.
    #[br(pre_assert(*magic == ClientLobbyIpcType::LoginEx))]
    LoginEx {
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
    #[br(pre_assert(*magic == ClientLobbyIpcType::ShandaLogin))]
    ShandaLogin {
        #[bw(pad_size_to = 1456)]
        #[br(count = 1456)]
        unk: Vec<u8>,
    },
    /// Sent by the client when they request something about the character (e.g. deletion.)
    #[br(pre_assert(*magic == ClientLobbyIpcType::CharaMake))]
    CharaMake(CharaMake),
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

#[binrw]
#[br(import(magic: &ServerLobbyIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ServerLobbyIpcData {
    /// Sent by the server to indicate an lobby error occured.
    #[br(pre_assert(*magic == ServerLobbyIpcType::NackReply))]
    NackReply {
        sequence: u64,
        error: u32,
        value: u32,
        exd_error_id: u16,
        #[brw(pad_after = 516)] // empty and garbage
        unk1: u16,
    },
    /// Sent by the server to inform the client of their service accounts.
    #[br(pre_assert(*magic == ServerLobbyIpcType::LoginReply))]
    LoginReply(LoginReply),
    /// Sent by the server to inform the client of their characters.
    #[br(pre_assert(*magic == ServerLobbyIpcType::ServiceLoginReply))]
    ServiceLoginReply(ServiceLoginReply),
    // Assumed what this is, but probably incorrect
    #[br(pre_assert(*magic == ServerLobbyIpcType::CharaMakeReply))]
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
    /// Sent by the server to tell the client how to connect to the world server.
    #[br(pre_assert(*magic == ServerLobbyIpcType::GameLoginReply))]
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
    /// Sent by the server to inform the client of their servers.
    #[br(pre_assert(*magic == ServerLobbyIpcType::DistWorldInfo))]
    DistWorldInfo(DistWorldInfo),
    /// Sent by the server to inform the client of their retainers.
    #[br(pre_assert(*magic == ServerLobbyIpcType::DistRetainerInfo))]
    DistRetainerInfo {
        // TODO: what is in here?
        #[brw(pad_before = 7)]
        #[brw(pad_after = 202)]
        unk1: u8,
    },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
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
                ServerLobbyIpcType::LoginReply,
                ServerLobbyIpcData::LoginReply(LoginReply::default()),
            ),
            (
                ServerLobbyIpcType::DistWorldInfo,
                ServerLobbyIpcData::DistWorldInfo(DistWorldInfo::default()),
            ),
            (
                ServerLobbyIpcType::DistRetainerInfo,
                ServerLobbyIpcData::DistRetainerInfo { unk1: 0 },
            ),
            (
                ServerLobbyIpcType::ServiceLoginReply,
                ServerLobbyIpcData::ServiceLoginReply(ServiceLoginReply::default()),
            ),
            (
                ServerLobbyIpcType::GameLoginReply,
                ServerLobbyIpcData::GameLoginReply {
                    sequence: 0,
                    actor_id: 0,
                    content_id: 0,
                    token: String::new(),
                    port: 0,
                    host: String::new(),
                },
            ),
            (
                ServerLobbyIpcType::NackReply,
                ServerLobbyIpcData::NackReply {
                    sequence: 0,
                    error: 0,
                    value: 0,
                    exd_error_id: 0,
                    unk1: 0,
                },
            ),
            (
                ServerLobbyIpcType::CharaMakeReply,
                ServerLobbyIpcData::CharaMakeReply {
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
                option: 0,
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
