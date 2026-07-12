use binrw::binrw;
use bitflags::bitflags;

use crate::common::CHAR_NAME_MAX_LENGTH;

use super::{read_string, write_string};

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct CharacterFlag(u8);

bitflags! {
    impl CharacterFlag : u8 {
        /// "You cannot select this character with your current account."
        const LOCKED = 1;
        /// "A name change is required to log in with this character."
        const NAME_CHANGE_REQUIRED = 2;
        /// Not working?
        const MISSING_EXPANSION_FOR_LOGIN = 4;
        /// "To log in with this character you must first install A Realm Reborn". Depends on an expansion version of the race maybe?
        const MISSING_EXPANSION_FOR_EDIT = 8;
        /// Shows a DC traveling icon on the right, and changes the text on the left
        const DC_TRAVELING = 16;
        /// "This character is currently visiting the XYZ data center". ???
        const DC_TRAVELING_MESSAGE = 32;
    }
}

impl std::fmt::Debug for CharacterFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct CharacterDetails {
    pub player_id: u64,
    pub content_id: u64,
    pub index: u8,
    pub flags: CharacterFlag,
    pub unk1: [u8; 6],
    pub origin_server_id: u16,
    pub current_server_id: u16,
    pub unk2: [u8; 16],
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub origin_server_name: String,
    #[bw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub current_server_name: String,
    #[bw(pad_size_to = 1024)]
    #[br(count = 1024)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub character_detail_json: String,
    pub unk3: [u32; 5],
}

impl CharacterDetails {
    pub const SIZE: usize = 1184;
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceLoginReplyFlag1(u8);

bitflags! {
    impl ServiceLoginReplyFlag1 : u8 {

    }
}

impl std::fmt::Debug for ServiceLoginReplyFlag1 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceLoginReplyFlag2(u8);

bitflags! {
    impl ServiceLoginReplyFlag2 : u8 {
        const HIDE_SUBSCRIPTION_INFORMATION = 16;
        const LEGACY_ACCOUNT = 128;
    }
}

impl std::fmt::Debug for ServiceLoginReplyFlag2 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceLoginReplyFlag3(u8);

bitflags! {
    impl ServiceLoginReplyFlag3 : u8 {
        const CONFIG_SYSTEM_ONLINE = 64;
    }
}

impl std::fmt::Debug for ServiceLoginReplyFlag3 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct ServiceLoginReplyFlag4(u16);

bitflags! {
    impl ServiceLoginReplyFlag4 : u16 {
        const CAN_CREATE_NEW_CHARACTERS = 1;
    }
}

impl std::fmt::Debug for ServiceLoginReplyFlag4 {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ServiceLoginReply {
    pub sequence: u64,
    pub counter: u8,
    #[brw(pad_after = 2)]
    pub num_in_packet: u8,
    pub unk1: u8,
    pub unk2: u8,
    pub flag1: ServiceLoginReplyFlag1,
    pub flag2: ServiceLoginReplyFlag2,
    pub unk5: [u32; 7],
    pub flag3: ServiceLoginReplyFlag3,
    pub veteran_rank: u8,
    #[brw(pad_after = 1)]
    pub unk7: u8,
    pub days_subscribed: i32,
    pub remaining_days: i32,
    pub days_to_next_rank: i32,
    pub max_characters_on_world: u16,
    pub flag4: ServiceLoginReplyFlag4,
    /// Seems to control which races are available.
    #[brw(pad_after = 12)]
    pub entitled_expansion: u32,
    #[brw(pad_after = 24)]
    #[br(count = Self::MAX_CHARACTERS)]
    #[brw(pad_size_to = (CharacterDetails::SIZE * Self::MAX_CHARACTERS))]
    pub characters: Vec<CharacterDetails>,
}

impl ServiceLoginReply {
    pub const MAX_CHARACTERS: usize = 2;
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn character_details_size() {
        ensure_size::<CharacterDetails, { CharacterDetails::SIZE }>();
    }
}
