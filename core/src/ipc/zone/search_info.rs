use crate::{
    common::{read_string, value_to_flag_byte_index_value, write_string},
    ipc::zone::{GrandCompany, OnlineStatusMask, SocialListUILanguages},
};
use binrw::binrw;
use bitflags::bitflags;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

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

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, EnumIter, Eq, FromRepr, Hash, PartialEq)]
pub enum SearchUIClassJob {
    #[default]
    Gladiator = 0,
    Pugilist = 1,
    Marauder = 2,
    Lancer = 3,
    Archer = 4,
    Conjurer = 5,
    Thaumaturge = 6,
    Carpenter = 7,
    Blacksmith = 8,
    Armorer = 9,
    Goldsmith = 10,
    Leatherworker = 11,
    Weaver = 12,
    Alchemist = 13,
    Culinarian = 14,
    Miner = 15,
    Botanist = 16,
    Fisher = 17,
    Paladin = 18,
    Monk = 19,
    Warrior = 20,
    Dragoon = 21,
    Bard = 22,
    WhiteMage = 23,
    BlackMage = 24,
    Arcanist = 25,
    Summoner = 26,
    Scholar = 27,
    Rogue = 28,
    Ninja = 29,
    Machinist = 30,
    DarkKnight = 31,
    Astrologian = 32,
    Samurai = 33,
    RedMage = 34,
    BlueMage = 35,
    Gunbreaker = 36,
    Dancer = 37,
    Reaper = 38,
    Sage = 39,
    Viper = 40,
    Pictomancer = 41,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Default, Hash, PartialEq)]
pub struct SearchUIClassJobMask {
    pub flags: [u8; 8],
}

impl From<[u8; 8]> for SearchUIClassJobMask {
    fn from(flags: [u8; 8]) -> Self {
        Self { flags }
    }
}

impl SearchUIClassJobMask {
    pub fn from_searchui_classjob(classjob: SearchUIClassJob) -> Self {
        let mut classjobs = Self::default();
        classjobs.set_classjob(classjob);

        classjobs
    }

    pub fn mask(&self) -> Vec<SearchUIClassJob> {
        let mut classjobs = Vec::new();

        for classjob in SearchUIClassJob::iter() {
            let (value, index) = value_to_flag_byte_index_value(classjob as u32);
            if self.flags[index as usize] & value == value {
                classjobs.push(classjob);
            }
        }

        classjobs
    }

    pub fn set_classjob(&mut self, classjob: SearchUIClassJob) {
        let (value, index) = value_to_flag_byte_index_value(classjob as u32);
        self.flags[index as usize] |= value;
    }

    pub fn remove_classjob(&mut self, classjob: SearchUIClassJob) {
        let (value, index) = value_to_flag_byte_index_value(classjob as u32);
        self.flags[index as usize] ^= value;
    }

    pub fn has_classjob(&self, classjob: SearchUIClassJob) -> bool {
        let (value, index) = value_to_flag_byte_index_value(classjob as u32);
        self.flags[index as usize] & value == value
    }
}

impl std::fmt::Debug for SearchUIClassJobMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flags.iter().all(|x| *x == 0) {
            return write!(f, "None");
        }

        if self.mask().is_empty() {
            write!(f, "{:#?}", self.flags)
        } else {
            write!(f, "{:?}", self.mask())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_searchui_classjob_lancer() {
        let mask: [u8; 8] = [8, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            SearchUIClassJobMask::from(mask).mask(),
            vec![SearchUIClassJob::Lancer]
        );
    }

    #[test]
    fn read_searchui_classjob_archer() {
        let mask: [u8; 8] = [16, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            SearchUIClassJobMask::from(mask).mask(),
            vec![SearchUIClassJob::Archer]
        );
    }
}
