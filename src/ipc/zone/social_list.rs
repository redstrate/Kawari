use binrw::binrw;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, read_bool_from, read_string, value_to_flag_byte_index_value,
    write_bool_as, write_string,
};
use crate::ipc::zone::OnlineStatus;
use bitflags::bitflags;
use strum::IntoEnumIterator;

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, Clone, Copy, Default)]
pub enum SocialListRequestType {
    #[default]
    Party = 0x1,
    Friends = 0x2,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SocialListRequest {
    #[brw(pad_before = 10)] // empty
    pub request_type: SocialListRequestType,
    #[brw(pad_after = 4)] // empty
    pub count: u8,
}

// TODO: Move OnlineStatusMask elsewhere if it ends up being used in multiple places
/// Represents a 64-bit online status. For possible values, see common_spawn.rs's OnlineStatus enum.
#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Default)]
pub struct OnlineStatusMask {
    flags: [u8; 8],
}

impl OnlineStatusMask {
    pub fn mask(&self) -> Vec<OnlineStatus> {
        let mut statuses = Vec::new();

        for status in OnlineStatus::iter() {
            let (value, index) = value_to_flag_byte_index_value(status.clone() as u32);
            if self.flags[index as usize] & value == value {
                statuses.push(status);
            }
        }
        statuses
    }

    pub fn set_status(&mut self, status: OnlineStatus) {
        let (value, index) = value_to_flag_byte_index_value(status as u32);
        self.flags[index as usize] |= value;
    }

    pub fn remove_status(&mut self, status: OnlineStatus) {
        let (value, index) = value_to_flag_byte_index_value(status as u32);
        self.flags[index as usize] ^= value;
    }
}

impl std::fmt::Debug for OnlineStatusMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OnlineStatusMask {:#?} ({:#?})", self.flags, self.mask())
    }
}

/// Which languages the client's player wishes to be grouped and/or interacted with.
#[binrw]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Language(u8);

bitflags! {
    impl Language: u8 {
        const JAPANESE = 1;
        const ENGLISH = 2;
        const GERMAN = 4;
        const FRENCH = 8;
    }
}

/// Which language the client indicates as its primary language.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum LanguageUnderline {
    #[default]
    Japanese = 0,
    English = 1,
    German = 2,
    French = 3,
}

/// Which Grand Company the player is currently associated with.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum GrandCompany {
    #[default]
    None = 0,
    Maelstrom = 1,
    Adders = 2,
    Flames = 3,
}

/// Flags to enable or disable various things in the Social Menu UI.
#[binrw]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SocialListUIFlags(u16);

bitflags! {
    impl SocialListUIFlags: u16 {
        /// The player data was unable to be retrieved (deleted, on another datacenter (?), some other issue).
        const UNABLE_TO_RETRIEVE = 1;
        /// Enables the right-click context menu for this PlayerEntry.
        const ENABLE_CONTEXT_MENU = 4096;
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerEntry {
    pub content_id: u64,
    pub unk1: [u8; 6],
    #[brw(pad_after = 8)]
    pub current_world_id: u16,
    pub unk2: [u8; 10],
    pub ui_flags: SocialListUIFlags,
    #[brw(pad_after = 2)]
    pub zone_id: u16,
    pub grand_company: GrandCompany,
    pub language_underline: LanguageUnderline,
    pub language: Language,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub has_search_comment: bool,
    #[brw(pad_before = 4)]
    pub online_status_mask: OnlineStatusMask,
    #[brw(pad_after = 1)]
    pub classjob_id: u8,
    #[brw(pad_after = 7)]
    pub classjob_level: u8,
    pub home_world_id: u16,
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    #[brw(pad_after = 6)]
    #[br(count = 6)]
    #[bw(pad_size_to = 6)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,
}

impl PlayerEntry {
    pub const SIZE: usize = 112;
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SocialList {
    #[brw(pad_before = 12)] // empty
    pub request_type: SocialListRequestType,
    pub sequence: u8,
    #[brw(pad_before = 2)] // empty
    #[br(count = 10)]
    #[bw(pad_size_to = 10 * PlayerEntry::SIZE)]
    pub entries: Vec<PlayerEntry>,
}
