use binrw::binrw;
use bitflags::bitflags;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ServerNoticeFlags(pub u8);

impl std::fmt::Debug for ServerNoticeFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

// See https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/common/Network/PacketDef/Zone/ServerZoneDef.h#L250
bitflags! {
    impl ServerNoticeFlags : u8 {
        /// Shows in the chat log.
        const NONE = 0x000;
        /// Shows in the chat log.
        const CHAT_LOG = 0x001;
        /// Shows as an on-screen message.
        const ON_SCREEN = 0x004;
    }
}

impl Default for ServerNoticeFlags {
    fn default() -> Self {
        Self::NONE
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ServerNoticeMessage {
    pub flags: ServerNoticeFlags,
    #[brw(pad_size_to = 775)]
    #[br(count = 775)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
