use binrw::binrw;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, read_bool_from, read_string, write_bool_as, write_string,
};
use bitflags::bitflags;

use super::online_status::OnlineStatusMask;

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

/// Which languages the client's player wishes to be grouped and/or interacted with.
/// These are set by the client in the Edit Search Info menu (the Content Finder's seem to be used exclusively for grouping preferences?), but by default the primary language will be enabled.
/// Not to be confused with physis::common::Language.
#[binrw]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SocialListUILanguages(u8);

bitflags! {
    impl SocialListUILanguages: u8 {
        const JAPANESE = 1;
        const ENGLISH = 2;
        const GERMAN = 4;
        const FRENCH = 8;
    }
}

impl Default for SocialListUILanguages {
    fn default() -> Self {
        SocialListUILanguages::JAPANESE
    }
}

/// Which language the client indicates as its primary language.
/// Not to be confused with physis::common::Language.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum ClientLanguage {
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
    pub client_language: ClientLanguage,
    pub social_ui_languages: SocialListUILanguages,
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
