use binrw::binrw;
use bstr::BString;

use crate::{
    common::{
        CHAR_NAME_MAX_LENGTH, ClientLanguage, read_sestring, read_string, write_sestring,
        write_string,
    },
    ipc::zone::{FellowshipActivityTag, SocialListUILanguages},
};

#[binrw]
#[derive(Clone, Default, Debug)]
pub struct FellowshipSearchInfo {
    /// The Fellowship's community id (assumed).
    pub community_id: u64,
    /// The master or recruiter's content id (assumed).
    pub master_content_id: u64, // TODO: Figure out which content id is which
    /// The recruiter or master's content id (assumed).
    pub recruiter_content_id: u64,
    /// A 32-bit UNIX timestamp indicating when this recruitment listing will expire.
    pub recruitment_deadline: u32,
    /// The languages this Fellowship is seeking.
    pub languages: SocialListUILanguages, // Fellowship language(s), this might be SocialListUILanguages
    #[brw(pad_before = 3, pad_after = 3)] // Probably just padding
    /// The primary/underlined language of this Fellowship. This seems to be set to the recruiter's client language at the time of Fellowship's creation, or during the time of starting a recruitment listing.
    pub primary_language: ClientLanguage, // 0 = Japanese, 1 = English, 2 = German, 3 = French
    /// The home world of the Fellowship's recruiter.
    pub recruiter_world_id: u16,
    /// The home world of the Fellowship's master.
    pub master_world_id: u16,
    /// The current number of members in this Fellowship, out of 1000 (but the client will display numbers higher than 1000).
    pub current_member_count: u16,
    /// The number of members the recruiter is attempting to reach (up to 1000, but the client can display up to 65,535 properly).
    pub target_member_count: u16,
    /// The Fellowship's first activity icon. References the CircleActivity excel sheet.
    pub activity1: FellowshipActivityTag,
    pub unk1: [u8; 4], // Has values in it, but it's unclear what they control yet. Changing them doesn't seem to affect what the client shows in the Finder window.
    /// The Fellowship's second activity icon. References the CircleActivity excel sheet.
    pub activity2: FellowshipActivityTag,
    /// The fellowship's third activity icon. References the CircleActivity excel sheet.
    pub activity3: FellowshipActivityTag,
    /// The name of the Fellowship. It can be up to 60 characters in length (fewer if using Unicode glyphs).
    #[brw(pad_size_to = 61)]
    #[br(count = 60)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fellowship_name: String,
    /// The Fellowship recruiter's name. The recruiter can be the same person as the master.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub recruiter_name: String,
    /// The Fellowship master's name.
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub master_name: String,
    /// The Fellowship's text description. This can be up to 192 characters in length (fewer if using Unicode glyphs).
    #[brw(pad_after = 5)]
    #[brw(pad_size_to = 192)]
    #[br(count = 192)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    pub fellowship_description: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
