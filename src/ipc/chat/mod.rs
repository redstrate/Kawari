use binrw::binrw;

use crate::{
    opcodes::{ClientChatIpcType, ServerChatIpcType},
    packet::{IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcSegment},
};

pub type ServerChatIpcSegment = IpcSegment<ServerChatIpcType, ServerChatIpcData>;

impl ReadWriteIpcSegment for ServerChatIpcSegment {
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
impl Default for ServerChatIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ServerChatIpcType::LoginReply,
            option: 0,
            timestamp: 0,
            data: ServerChatIpcData::LoginReply {
                timestamp: 0,
                sid: 0,
            },
        }
    }
}

pub type ClientChatIpcSegment = IpcSegment<ClientChatIpcType, ClientChatIpcData>;

impl ReadWriteIpcSegment for ClientChatIpcSegment {
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
impl Default for ClientChatIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientChatIpcType::Unknown(0),
            option: 0,
            timestamp: 0,
            data: ClientChatIpcData::Unknown { unk: Vec::new() },
        }
    }
}

#[binrw]
#[br(import(magic: &ServerChatIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ServerChatIpcData {
    /// Sent by the server to Initialize something chat-related?
    #[br(pre_assert(*magic == ServerChatIpcType::LoginReply))]
    LoginReply { timestamp: u32, sid: u32 },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

#[binrw]
#[br(import(_magic: &ClientChatIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientChatIpcData {
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
    fn server_chat_ipc_sizes() {
        let ipc_types = [(
            ServerChatIpcType::LoginReply,
            ServerChatIpcData::LoginReply {
                timestamp: 0,
                sid: 0,
            },
        )];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerChatIpcSegment {
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
