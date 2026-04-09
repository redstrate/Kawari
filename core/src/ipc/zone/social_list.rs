use binrw::binrw;

use crate::common::{
    CHAR_NAME_MAX_LENGTH, ClientLanguage, read_bool_from, read_string, write_bool_as, write_string,
};
use bitflags::bitflags;
use strum_macros::FromRepr;

use super::online_status::OnlineStatusMask;

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SocialListRequestType {
    #[default]
    Party = 0x1,
    Friends = 0x2,
    Linkshell = 0x3,
    SearchResults = 0x4,
    FreeCompanyOnline = 0x5,
    FreeCompanyOffline = 0x6,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SocialListRequest {
    pub community_id: u64, // Used for at least free companies, but probably also linkshells & fellowships too
    pub next_index: u16,
    pub request_type: SocialListRequestType,
    #[brw(pad_after = 4)] // empty
    pub sequence: u8,
}

/// Which languages the client's player wishes to be grouped and/or interacted with.
/// These are set by the client in the Edit Search Info menu (the Content Finder's seem to be used exclusively for grouping preferences?), but by default the primary language will be enabled.
/// Not to be confused with physis::Language.
#[binrw]
#[derive(Clone, Copy, Eq, PartialEq)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Integer))]
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

impl std::fmt::Debug for SocialListUILanguages {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        bitflags::parser::to_writer(self, f)
    }
}

#[cfg(feature = "server")]
impl diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite>
    for SocialListUILanguages
{
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(self.0 as i32);
        Ok(diesel::serialize::IsNull::No)
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite>
    for SocialListUILanguages
{
    fn from_sql(
        mut integer: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        Ok(SocialListUILanguages(integer.read_integer() as u8))
    }
}

/// Which Grand Company the player is currently associated with.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, FromRepr, PartialEq)]
#[cfg_attr(
    feature = "server",
    derive(diesel::expression::AsExpression, diesel::deserialize::FromSqlRow)
)]
#[cfg_attr(feature = "server", diesel(sql_type = diesel::sql_types::Integer))]
pub enum GrandCompany {
    #[default]
    None = 0,
    Maelstrom = 1,
    Adders = 2,
    Flames = 3,
}

#[cfg(feature = "server")]
impl mlua::IntoLua for GrandCompany {
    fn into_lua(self, _: &mlua::Lua) -> mlua::Result<mlua::Value> {
        Ok(mlua::Value::Integer(self as i64))
    }
}

#[cfg(feature = "server")]
impl diesel::serialize::ToSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite> for GrandCompany {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::sqlite::Sqlite>,
    ) -> diesel::serialize::Result {
        out.set_value(*self as i32);
        Ok(diesel::serialize::IsNull::No)
    }
}

#[cfg(feature = "server")]
impl diesel::deserialize::FromSql<diesel::sql_types::Integer, diesel::sqlite::Sqlite>
    for GrandCompany
{
    fn from_sql(
        mut integer: <diesel::sqlite::Sqlite as diesel::backend::Backend>::RawValue<'_>,
    ) -> diesel::deserialize::Result<Self> {
        Ok(GrandCompany::from_repr(integer.read_integer() as usize).unwrap())
    }
}

// TODO: This seems to actually be entirely wrong, or at least reused for friend group icons in the context of the friend list, we need to rethink this eventully
/// Flags to enable or disable various things in the Social Menu UI.
#[binrw]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SocialListUIFlags(u16);

bitflags! {
    impl SocialListUIFlags: u16 {
        const NONE = 0;
        /// The player data was unable to be retrieved (deleted, on another datacenter (?), some other issue).
        const UNABLE_TO_RETRIEVE = 1;
        const UNKNOWN_2 = 2;
        const UNKNOWN_4 = 4;
        const UNKNOWN_256 = 256;
        /// Enables the right-click context menu for this PlayerEntry.
        const ENABLE_CONTEXT_MENU = 4096;
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerEntry {
    /// This player's content id.
    pub content_id: u64,
    /// A 32-bit Unix timestamp that likely changes meanings depending on the SocialList type being sent. For friend lists it indicates when they became friends with the client who requested the list.
    pub timestamp: u32,
    pub unk1: [u8; 2], // Unknown if these are ever used
    #[brw(pad_after = 8)]
    /// The current world they're on.
    pub current_world_id: u16,
    pub unk2: [u8; 10],
    pub ui_flags: SocialListUIFlags,
    #[brw(pad_after = 2)]
    /// Their current zone id.
    pub zone_id: u16,
    /// Their Grand Company.
    pub grand_company: GrandCompany,
    /// Their client language: this is different than the languages used for queueing and player searching.
    pub client_language: ClientLanguage,
    /// The languages this player prefers to queue or otherwise interact with.
    pub social_ui_languages: SocialListUILanguages,
    /// If the player has a clickable search comment or not (speech bubble with "..." in it).
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub has_search_comment: bool,
    /// A mask indicating their online status: if they're in a party, in a duty, and so on.
    #[brw(pad_before = 4)]
    pub online_status_mask: OnlineStatusMask,
    /// Their current class/job.
    #[brw(pad_after = 1)]
    pub classjob_id: u8,
    /// Their current class/job's level.
    #[brw(pad_after = 7)]
    pub classjob_level: u8,
    /// The world they're originally from.
    pub home_world_id: u16,
    /// Their name.
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    /// Their free company tag, if any. The client will automatically display Voyager/Wanderer/Traveller in its place if they're from another world or datacenter.
    #[brw(pad_after = 6)]
    #[br(count = 6)]
    #[bw(pad_size_to = 6)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub fc_tag: String,
}

impl PlayerEntry {
    pub const COUNT: usize = 10;
    pub const SIZE: usize = 112;
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SocialList {
    pub community_id: u64, // Used for at least free companies, but probably also linkshells & fellowships too
    pub next_index: u16,
    pub current_index: u16,
    pub request_type: SocialListRequestType,
    pub sequence: u8,
    #[brw(pad_before = 2)] // Empty? Still possible it might have data in other SocialList types
    #[br(count = PlayerEntry::COUNT)]
    #[bw(pad_size_to = PlayerEntry::COUNT * PlayerEntry::SIZE)]
    pub entries: Vec<PlayerEntry>,
}

/// This struct represents information sent when the client adjusts the friend group icon of a friend with SetFriendGroupIcon. The server echoes it back as an acknowledgement in FriendGroupIcon.
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FriendGroupIconInfo {
    /// The friend's content id.
    content_id: u64,
    /// The desired group icon. 0 is no icon, and 1-7 correspond to the desired symbols.
    #[brw(pad_after = 4)] // empty
    icon: u32, // TODO: This is actually SocialListUIFlags, but we need to rework that first
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn player_entry_size() {
        ensure_size::<PlayerEntry, { PlayerEntry::SIZE }>();
    }
}
