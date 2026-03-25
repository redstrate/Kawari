use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CrossRealmListing {
    pub listing_id: u64,
    pub account_id: u64,
    pub content_id: u64,
    pub category: u32,
    pub duty: u16,
    pub duty_type: u16,
    pub world_id: u16,
    pub objective: u8,
    pub beginners_welcome: u8,
    pub duty_finder_settings: u8,
    pub loot_rule: u8,
    pub last_patch_hotfix_timestamp: u32,
    pub time_left: u16,
    pub avg_item_lv: u16,
    pub home_world_id: u16,
    pub client_language: u8,
    pub total_slots: u8,
    pub slots_filled: u8,
    pub join_condition_flags: u8,
    pub is_alliance: u8,
    pub number_of_parties: u8,
    pub slot_flags: [u64; 8],
    pub jobs_present: [u8; 8],
    // TODO: remove once the field positions are fixed
    #[br(count = 42)]
    #[brw(pad_size_to = 42)]
    pub bad_padding: Vec<u8>,
    /// Name of the character who put up the listing.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub recruiter_name: String,
    #[brw(pad_size_to = 192)]
    #[br(count = 192)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub comment: String,
    #[br(count = 8)]
    #[brw(pad_size_to = 8)]
    pub bad_padding2: Vec<u8>,
}

impl CrossRealmListing {
    pub const SIZE: usize = 0x190;
}

// Based off of https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Network/PartyFinderPackets.cs
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CrossRealmListings {
    pub unk10: u32,
    pub unk11: u32,
    pub unk12: u32,
    /// Starts at 1, counts up, ends with 0.
    pub segment_index: u32,
    #[br(count = 4)]
    #[brw(pad_size_to = CrossRealmListing::SIZE * 4)]
    pub entries: Vec<CrossRealmListing>,
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn listing_size() {
        ensure_size::<CrossRealmListing, { CrossRealmListing::SIZE }>();
    }
}
