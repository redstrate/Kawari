use binrw::binrw;
use bstr::BString;
use strum_macros::FromRepr;

use crate::common::{read_sestring, write_sestring};
use crate::ipc::zone::server::{
    CHAR_NAME_MAX_LENGTH, read_bool_from, read_string, write_bool_as, write_string,
};

/// Letter message previews can only be up to 60 bytes in length. In-game this translates to approx. 60 characters (fewer with multi-byte glyphs).
pub const PREVIEW_MSG_MAX_LENGTH: usize = 60;

/// Letter message previews can only be up to 601 bytes in length. In-game this translates to approx. 200 characters/glpyhs.
pub const LETTER_MSG_MAX_LENGTH: usize = 601;

/// The player's mailbox can hold 130 entries before mail starts being "sent back", according to the Delivery Moogle.
pub const MAX_MAIL: usize = 130;

/// The attachments_counter on MailboxStatus goes up to 20.
pub const MAX_ATTACHMENTS: u16 = 20;

/// The player's mailbox can hold 100 letters from friends.
pub const MAX_FRIEND_LETTERS: u8 = 100;

/// The player's mailbox can hold 20 reward letters.
pub const MAX_REWARD_LETTERS: u8 = 20;

/// The player's mailbox can hold 10 system letters (mail from GMs).
pub const MAX_SYSTEM_LETTERS: u8 = 10;

/// The maximum amount of attachments a letter may contain.
pub const MAX_MAIL_ATTACHMENTS_STORAGE: usize = 6;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct LetterPreview {
    /// The sender's content id.
    pub sender_content_id: u64,
    /// The time at which this letter was sent.
    pub timestamp: u32,
    /// Items attached to this letter, if any.
    pub attached_items: [AttachedItemInfo; MAX_MAIL_ATTACHMENTS_STORAGE],
    /// If the letter has been read or not.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub read: bool,
    /// The type of letter.
    pub mail_type: LetterType,
    pub unk: u8, // Unknown purpose, sometimes seen as 1. Seems to have no effect when changed?
    /// The sender's name.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub sender_name: String,
    /// A preview of this message, truncated to 60 characters (fewer if multi-byte characters are used).
    #[brw(pad_size_to = PREVIEW_MSG_MAX_LENGTH + 5)] // There are 5 bytes of padding after this string (pad_after is not the correct thing to use here!)
    #[br(count = PREVIEW_MSG_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}

impl LetterPreview {
    pub const COUNT: usize = 5;
    pub const SIZE: usize = 232;
}

#[binrw]
#[derive(Clone, Copy, Debug, Default)]
pub struct AttachedItemInfo {
    /// Index into the Items Excel sheet.
    pub item_id: u32,
    /// The quantity of this item.
    pub item_quantity: u32,
    pub unk: [u8; 12], // Observed as all zeroes
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, FromRepr)]
pub enum LetterType {
    /// This letter was sent by a player.
    #[default]
    Player = 0,
    /// This letter was sent by the system for a cash shop purchase or other promotional reward.
    Reward = 1,
    /// This letter was sent by a GM.
    GM = 2,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct Letter {
    /// The sender's content id.
    pub sender_content_id: u64,
    /// When the letter was sent.
    pub timestamp: u32,
    /// The message body.
    #[brw(pad_size_to = LETTER_MSG_MAX_LENGTH)]
    #[br(count = LETTER_MSG_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    #[brw(pad_after = 3)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
