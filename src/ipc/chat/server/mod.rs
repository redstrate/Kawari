use binrw::binrw;
use paramacro::opcode_data;

use crate::{
    opcodes::ServerChatIpcType,
    packet::{IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcOpcode, ReadWriteIpcSegment},
};

mod tell_message;
pub use tell_message::TellMessage;

mod party_message;
pub use party_message::PartyMessage;

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

    fn get_comment(&self) -> Option<&'static str> {
        self.op_code.get_comment()
    }
}

#[opcode_data(ServerChatIpcType)]
#[binrw]
#[br(import(magic: &ServerChatIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ServerChatIpcData {
    LoginReply {
        timestamp: u32,
        sid: u32,
    },
    TellMessage(TellMessage),
    PartyMessage(PartyMessage),
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

impl Default for ServerChatIpcData {
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
        ),
        (
            ServerChatIpcType::TellMessage,
            ServerChatIpcData::TellMessage(TellMessage {
                sender_account_id: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,
                sender_world_id: 0,
                flags: 0,
                sender_name: "".to_string(),
                message: "".to_string(),
            }),
        ),
        (
            ServerChatIpcType::PartyMessage,
            ServerChatIpcData::PartyMessage(PartyMessage {
                party_id: 0,
                sender_account_id: 0,
                unk1: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,

                sender_actor_id: 0,
                sender_world_id: 0,
                sender_name: "".to_string(),
                message: "".to_string(),
            }),
        ),
    ];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerChatIpcSegment {
                op_code: opcode.clone(),
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
