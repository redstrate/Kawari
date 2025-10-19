use binrw::binrw;
use paramacro::opcode_data;

use crate::{
    opcodes::ServerChatIpcType,
    packet::{IpcSegment, ServerIpcSegmentHeader},
};

mod tell_message;
pub use tell_message::{TellMessage, TellNotFoundError};

mod party_message;
pub use party_message::PartyMessage;

pub type ServerChatIpcSegment =
    IpcSegment<ServerIpcSegmentHeader<ServerChatIpcType>, ServerChatIpcType, ServerChatIpcData>;

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
    TellNotFoundError(TellNotFoundError),
    FreeCompanyEvent {
        // TODO: fill this in
        unk: [u8; 104],
    },
    JoinChannelResult {
        // TODO: fill this in
        unk: [u8; 32],
    },
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

    use crate::packet::{IpcSegmentHeader, ReadWriteIpcOpcode, ReadWriteIpcSegment};

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn server_chat_ipc_sizes() {
        let ipc_types = [
            ServerChatIpcData::LoginReply {
                timestamp: 0,
                sid: 0,
            },
            ServerChatIpcData::TellMessage(TellMessage::default()),
            ServerChatIpcData::PartyMessage(PartyMessage::default()),
            ServerChatIpcData::TellNotFoundError(TellNotFoundError::default()),
            ServerChatIpcData::FreeCompanyEvent { unk: [0; 104] },
            ServerChatIpcData::JoinChannelResult { unk: [0; 32] },
        ];

        for data in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let opcode: ServerChatIpcType = ReadWriteIpcOpcode::from_data(data);
            let ipc_segment = ServerChatIpcSegment {
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
