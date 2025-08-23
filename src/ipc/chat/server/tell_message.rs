use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct TellMessage {
    pub sender_account_id: u32,
    pub unk2: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub sender_world_id: u16,
    /// Indicates if it's a GM tell or not.
    pub flags: u8,

    #[br(count = 32)]
    #[bw(pad_size_to = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,

    #[br(count = 1024)]
    #[bw(pad_size_to = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 5)]
    pub message: String,
}
