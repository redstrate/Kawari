use binrw::binrw;
use kawari_core_macro::opcode_data;

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
    GetChannelList { unk: [u8; 8] },
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
    use crate::common::test_opcodes;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_chat_ipc_sizes() {
        test_opcodes::<ClientChatIpcSegment>();
    }
}
