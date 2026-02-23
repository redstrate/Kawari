use binrw::binrw;
use kawari_core_macro::opcode_data;

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
    GetChannelListResponse {
        unk: [u8; 768],
    },
}

#[cfg(test)]
mod tests {
    use crate::common::test_opcodes;

    use super::*;

    // Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn server_chat_ipc_sizes() {
        test_opcodes::<ServerChatIpcSegment>();
    }
}
