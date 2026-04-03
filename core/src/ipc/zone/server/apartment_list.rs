use binrw::binrw;

use crate::ipc::zone::{
    HousingAppealTag,
    server::{CHAR_NAME_MAX_LENGTH, read_bool_from, read_string, write_bool_as, write_string},
};

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct ApartmentList {
    /// The player's content id, unknown why it's here.
    pub content_id: u64,
    /// Unknown flags. Bit 7 seems to always be set (0x80), Bit 0 seems to indicate if this ward is a subdivision or not (1 if it is, 0 if not).
    pub flags: u16,
    /// Assumed, but it seems to be the current ward's number minus one, 0-based.
    pub ward_id: u16,
    /// The housing ward's zone id.
    pub zone_id: u16,
    /// The current world id.
    pub world_id: u16,
    #[brw(pad_after = 4)] // Seems to be empty/zeroes
    /// The starting index of this list. It's set to 1, 16, 31, 46, 61, or 76 depending on which tab the user selects in the UI. This is not a sequence value, as only one list is sent per tab selection.
    pub list_index: u32,
    /// The actual apartments' information.
    #[brw(pad_size_to = ApartmentListEntry::SIZE * ApartmentListEntry::COUNT)]
    #[br(count = ApartmentListEntry::COUNT)]
    pub apartments: Vec<ApartmentListEntry>,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct ApartmentListEntry {
    /// If this resident allows visitors or not.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    #[brw(pad_before = 12)] // Seems to be empty/zeroes
    pub visitors_permitted: bool,
    /// The housing tags the resident has set for their apartment. It gives visitors an idea of what to expect when entering.
    pub housing_appeal: [HousingAppealTag; 3],
    /// The resident's name.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub resident_name: String,
    /// A user-provided description of the apartment.
    #[brw(pad_size_to = 56)]
    #[br(count = 56)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub apartment_description: String,
}

impl ApartmentListEntry {
    pub const SIZE: usize = 104;
    pub const COUNT: usize = 15;
}
