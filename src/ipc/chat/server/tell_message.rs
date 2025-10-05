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

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct TellNotFoundError {
    /// Assumed.
    pub recipient_account_id: u32,
    /// Commonly seen as 0x0000.
    pub unk1: u16,
    /// Commonly seen as 0x0040.
    pub unk2: u16,
    /// Assumed.
    pub sender_account_id: u32,
    /// Commonly seen as 0x0017.
    pub unk3: u16,
    /// Commonly seen as 0x0040.
    pub unk4: u16,
    /// Commonly seen as 0x68.
    pub unk5: u32,
    /// The recipient's world id.
    pub recipient_world_id: u16,
    /// The recipient's name.
    #[br(count = 32)]
    #[bw(pad_size_to = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 2)]
    pub recipient_name: String,
}
