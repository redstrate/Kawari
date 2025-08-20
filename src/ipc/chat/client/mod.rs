use binrw::binrw;
use paramacro::opcode_data;

mod send_tell_message;
pub use send_tell_message::SendTellMessage;

mod send_party_message;
pub use send_party_message::SendPartyMessage;

use crate::{
    opcodes::ClientChatIpcType,
    packet::{IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcOpcode, ReadWriteIpcSegment},
};

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

    fn get_comment(&self) -> Option<&'static str> {
        self.op_code.get_comment()
    }
}

#[opcode_data(ClientChatIpcType)]
#[binrw]
#[br(import(magic: &ClientChatIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientChatIpcData {
    SendTellMessage(SendTellMessage),
    SendPartyMessage(SendPartyMessage),
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

impl Default for ClientChatIpcData {
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
    fn client_chat_ipc_sizes() {
        let ipc_types = [
            (
                ClientChatIpcType::SendTellMessage,
                ClientChatIpcData::SendTellMessage(SendTellMessage {
                    origin_world_id: 0,
                    recipient_world_id: 0,
                    recipient_name: "".to_string(),
                    message: "".to_string(),
                }),
            ),
        ];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ClientChatIpcSegment {
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
