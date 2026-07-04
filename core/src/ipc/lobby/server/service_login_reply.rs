use binrw::binrw;
use bitflags::bitflags;

use crate::common::CHAR_NAME_MAX_LENGTH;

use super::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

/// Sent by the server to inform the client of their characters. The layout matches the CN client's
/// `LobbyDownAccountLoginReply`, which simply gives proper names to the fields the international
/// client leaves unknown — the binary layout (total size 0x9A8) is identical for both.
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ServiceLoginReply {
    /// 0x00
    pub request_id: u32,
    /// 0x04
    pub timestamp: u32,
    /// 0x08: bit 0 = is_last_reply, bits 1-7 = reply_index.
    pub reply_flags: u8,
    /// 0x09 number of characters in this packet.
    pub count: u8,
    /// 0x0A
    pub param1: u16,
    /// 0x0C
    pub param2: u32,
    /// 0x10 月卡剩余秒数 (subscription seconds remaining).
    pub paid_monthly: u64,
    /// 0x18 点卡剩余秒数 (time-card seconds remaining).
    pub paid_point: u64,
    /// 0x20 免费试用剩余秒数 (free-trial seconds remaining).
    pub free_point: u64,
    /// 0x28 支付类型: 0 = 月卡, 1 = 点卡.
    pub pay_type: u32,
    /// 0x2C 64 = config system online.
    pub flag: u8,
    /// 0x2D
    pub veteran_rank: u8,
    /// 0x30 (preceded by 2 bytes of padding at 0x2E-0x2F).
    #[brw(pad_before = 2)]
    pub sumday: u32,
    /// 0x34
    pub remaining_days: u32,
    /// 0x38
    pub next_reward_days: u32,
    /// 0x3C
    pub max_create_character: u16,
    /// 0x3E
    pub max_character_list: u16,
    /// 0x40 expansion version (controls which races/expansions are available).
    pub exver: u16,
    /// 0x44 (preceded by 2 bytes of padding at 0x42-0x43).
    #[brw(pad_before = 2)]
    pub save_time: u32,
    /// 0x48
    pub save_platform: u32,
    /// 0x4C
    pub save_error: u8,
    /// 0x50 (preceded by 3 bytes of padding at 0x4D-0x4F).
    #[brw(pad_before = 3)]
    #[br(count = Self::MAX_CHARACTERS)]
    #[brw(pad_size_to = (CharacterDetails::SIZE * Self::MAX_CHARACTERS))]
    pub characters: Vec<CharacterDetails>,
    /// 0x990 云存档 hash (cloud-save hash), 0 = none. 4 bytes of trailing padding bring the
    /// struct to its full 0x9A8 size.
    #[brw(pad_after = 4)]
    pub hash: [u8; 20],
}

impl ServiceLoginReply {
    pub const MAX_CHARACTERS: usize = 2;
    pub const SIZE: usize = 0x9A8;
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn character_details_size() {
        ensure_size::<CharacterDetails, { CharacterDetails::SIZE }>();
    }

    /// `ServiceLoginReply` holds a Vec, so check its serialized size matches the client's 0x9A8.
    #[test]
    fn service_login_reply_size() {
        use binrw::BinWrite;
        let mut buffer = Vec::new();
        ServiceLoginReply::default()
            .write_le(&mut std::io::Cursor::new(&mut buffer))
            .unwrap();
        assert_eq!(buffer.len(), ServiceLoginReply::SIZE);
    }
}
