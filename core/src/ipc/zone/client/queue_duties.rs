use binrw::binrw;
use bitflags::bitflags;

use crate::ipc::zone::SocialListUILanguages;

// TODO: Rename to DutyFinderSetting
#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ContentRegistrationFlags(u64);

bitflags! {
    impl ContentRegistrationFlags: u64 {
        /// No special settings were enabled.
        const NONE = 0x0;
        /// Enables join party in progress mode.
        const JOIN_PARTY_IN_PROGRESS = 0x2;
        /// Enables unrestricted party mode.
        const UNRESTRICTED_PARTY = 0x2000;
        /// Enables minimum item level mode.
        const MINIMUM_ITEM_LEVEL = 0x4000;
        /// Enables level sync mode.
        const LEVEL_SYNC = 0x200000;
        /// Enables silence echo mode.
        const SILENCE_ECHO = 0x10000000;
        /// Enables explorer mode. If the client enables this, no other flags are sent.
        const EXPLORER_MODE = 0x100000000;
    }
}

impl Default for ContentRegistrationFlags {
    fn default() -> Self {
        ContentRegistrationFlags::NONE
    }
}

impl std::fmt::Debug for ContentRegistrationFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct QueueDuties {
    unk1: [u8; 8],
    /// The settings the client is queuing with.
    pub flags: ContentRegistrationFlags,
    /// Selected languages to match with.
    pub languages: SocialListUILanguages,
    unk3: u8,
    unk6: u8,
    unk4: [u8; 7],
    /// List of Content Finder Condition IDs the player signed up for.
    pub content_ids: [u16; 5],
    unk5: [u8; 4],
}
