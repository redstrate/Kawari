use binrw::binrw;
use paramacro::opcode_data;

mod send_tell_message;
pub use send_tell_message::SendTellMessage;

mod send_party_message;
pub use send_party_message::SendPartyMessage;

use crate::{
    opcodes::ClientChatIpcType,
    packet::{IpcSegment, ServerIpcSegmentHeader},
};

pub type ClientChatIpcSegment =
    IpcSegment<ServerIpcSegmentHeader<ClientChatIpcType>, ClientChatIpcType, ClientChatIpcData>;

#[opcode_data(ClientChatIpcType)]
#[binrw]
#[br(import(magic: &ClientChatIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientChatIpcData {
    SendTellMessage(SendTellMessage),
    SendPartyMessage(SendPartyMessage),
    GetChannelList {
        unk: [u8; 8],
    },
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

    use crate::packet::{IpcSegmentHeader, ReadWriteIpcOpcode, ReadWriteIpcSegment};

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_chat_ipc_sizes() {
        let ipc_types = [
            ClientChatIpcData::SendTellMessage(SendTellMessage::default()),
            ClientChatIpcData::SendPartyMessage(SendPartyMessage::default()),
            ClientChatIpcData::GetChannelList { unk: [0; 8] },
        ];

        for data in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let opcode: ClientChatIpcType = ReadWriteIpcOpcode::from_data(data);
            let ipc_segment = ClientChatIpcSegment {
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
