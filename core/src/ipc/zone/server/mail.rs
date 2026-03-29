use binrw::binrw;

use crate::ipc::zone::server::{CHAR_NAME_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Clone, Debug)]
pub struct LetterPreview {
    /// The sender's content id.
    pub sender_content_id: u64,
    /// The time at which this letter was sent.
    pub timestamp: u32,
    /// Items attached to this letter, if any.
    #[brw(pad_after = 3)] // empty/zeroes
    pub attached_items: [SentItemInfo; 6],
    /// The sender's name.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,
    /// A preview of this message, truncated to 60 characters (fewer if multi-byte characters are used).
    #[brw(pad_size_to = 61)]
    #[br(count = 61)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 4)] // empty/zeroes
    pub message: String,
}

#[binrw]
#[derive(Copy, Clone, Debug)]
pub struct SentItemInfo {
    /// Index into the Items Excel sheet.
    pub item_id: u32,
    /// The quantity of this item.
    pub item_quantity: u32,
    unk: [u8; 12], // Observed as all zeroes, but this might be used for materia melds and item quality, etc.? Need to research more.
}
