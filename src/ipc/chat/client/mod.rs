use binrw::binrw;
use paramacro::opcode_data;

mod send_tell_message;
pub use send_tell_message::SendTellMessage;

mod send_party_message;
pub use send_party_message::SendPartyMessage;

use crate::{
    opcodes::ClientChatIpcType,
    packet::{
        IPC_HEADER_SIZE, IpcSegment, ReadWriteIpcOpcode, ReadWriteIpcSegment,
        ServerIpcSegmentHeader,
    },
};

pub type ClientChatIpcSegment =
    IpcSegment<ServerIpcSegmentHeader<ClientChatIpcType>, ClientChatIpcType, ClientChatIpcData>;

impl ReadWriteIpcSegment for ClientChatIpcSegment {
    fn calc_size(&self) -> u32 {
        IPC_HEADER_SIZE + self.header.op_code.calc_size()
    }

    fn get_name(&self) -> &'static str {
        self.header.op_code.get_name()
    }

    fn get_opcode(&self) -> u16 {
        self.header.op_code.get_opcode()
    }

    fn get_comment(&self) -> Option<&'static str> {
        self.header.op_code.get_comment()
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

    use crate::packet::IpcSegmentHeader;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_chat_ipc_sizes() {
        let ipc_types = [
            (
                ClientChatIpcType::SendTellMessage,
                ClientChatIpcData::SendTellMessage(SendTellMessage::default()),
            ),
            (
                ClientChatIpcType::SendPartyMessage,
                ClientChatIpcData::SendPartyMessage(SendPartyMessage::default()),
            ),
        ];

        for (opcode, ipc) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ClientChatIpcSegment {
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
