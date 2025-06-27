use binrw::binrw;

use crate::{
    opcodes::ServerChatIpcType,
    packet::{IpcSegment, ReadWriteIpcSegment},
};

pub type ServerChatIpcSegment = IpcSegment<ServerChatIpcType, ServerChatIpcData>;

impl ReadWriteIpcSegment for ServerChatIpcSegment {
    fn calc_size(&self) -> u32 {
        // 16 is the size of the IPC header
        16 + self.op_code.calc_size()
    }

    fn get_name(&self) -> &'static str {
        self.op_code.get_name()
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

#[binrw]
#[br(import(magic: &ServerChatIpcType))]
#[derive(Debug, Clone)]
pub enum ServerChatIpcData {
    /// Sent by the server to Initialize something chat-related?
    #[br(pre_assert(*magic == ServerChatIpcType::LoginReply))]
    LoginReply {
        timestamp: u32,
        sid: u32,
    },
    Unknown,
}
