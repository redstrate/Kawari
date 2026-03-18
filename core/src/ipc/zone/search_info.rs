use crate::{
    common::{read_string, write_string},
    ipc::zone::{GrandCompany, OnlineStatusMask, SocialListUILanguages},
};
use binrw::binrw;
use bitflags::bitflags;

#[binrw]
#[brw(little)]
#[derive(Clone, Default, Debug)]
pub struct SearchInfo {
    pub online_status: OnlineStatusMask,
    pub unk1: [u8; 9],
    pub selected_languages: SocialListUILanguages,
    #[brw(pad_size_to = 60)]
    #[br(count = 60)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub comment: String,
    #[br(count = 138)]
    #[bw(pad_size_to = 138)]
    pub unk: Vec<u8>,
}

#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SearchUIGrandCompanies(u8);

bitflags! {
    impl SearchUIGrandCompanies: u8 {
        const INVALID = 0; // This should never show up on searches, as the client searching for no companies uses NONE. This is included so we can start from a blank slate while processing search results.
        const MAELSTROM = 2;
        const ADDERS = 4;
        const FLAMES = 8;
        const NONE = 255;
    }
}

impl Default for SearchUIGrandCompanies {
    fn default() -> Self {
        SearchUIGrandCompanies::INVALID
    }
}

impl std::fmt::Debug for SearchUIGrandCompanies {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Special-case NONE because it isn't all enabled at once, it's the *absence* of all at once.
        if *self == SearchUIGrandCompanies::NONE {
            return write!(f, "NONE");
        }

        bitflags::parser::to_writer(self, f)
    }
}

impl From<&GrandCompany> for SearchUIGrandCompanies {
    fn from(gc: &GrandCompany) -> Self {
        let mut new_info = SearchUIGrandCompanies::default();

        match gc {
            GrandCompany::Adders => new_info.set(SearchUIGrandCompanies::ADDERS, true),
            GrandCompany::Flames => new_info.set(SearchUIGrandCompanies::FLAMES, true),
            GrandCompany::Maelstrom => new_info.set(SearchUIGrandCompanies::MAELSTROM, true),
            _ => {}
        }

        new_info
    }
}
