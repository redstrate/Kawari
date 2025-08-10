use binrw::binrw;
use paramacro::opcode_data;

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
}

#[opcode_data(ClientChatIpcType)]
#[binrw]
#[br(import(_magic: &ClientChatIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientChatIpcData {
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
