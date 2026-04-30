use binrw::binrw;
use strum_macros::FromRepr;

use crate::common::{read_bool_from, write_bool_as};
use crate::ipc::zone::server::{CHAR_NAME_MAX_LENGTH, ChatChannel, read_string, write_string};

pub const CWLS_MAX_MEMBERS: usize = 64;
pub const LWLS_MAX_MEMBERS: usize = 128;

/// Represents one entry in the Linkshells opcode.
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct LinkshellEntry {
    pub common_ids: CWLSCommonIdentifiers,
    pub unk1: u32,
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 4)] // Seems to be empty/zeroes
    pub linkshell_name: String,
}

impl LinkshellEntry {
    pub const SIZE: usize = 56;
    pub const COUNT: usize = 8;
}

/// Represents one member entry in the CWLSMemberList.
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CWLSMemberListEntry {
    pub content_id: u64,
    pub unk_timestamp: u32, // Possibly when this member joined, or last had their rank changed?
    pub home_world_id: u16,
    pub current_world_id: u16,
    pub zone_id: u16,
    pub rank: CWLSPermissionRank,
    pub unk1: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub is_online: bool,
    pub unk2: u8, // TODO: What is this? It seems to always be 1, but changing it makes no apparent difference.
    #[brw(pad_after = 2)] // Seems to be empty/zeroes
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

impl CWLSMemberListEntry {
    pub const SIZE: usize = 56;
    pub const COUNT: usize = 8;
}

/// Represents one of the several ranks available in a CWLS.
#[binrw]
#[brw(repr = u8)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, FromRepr, PartialEq, PartialOrd)]
pub enum CWLSPermissionRank {
    #[default]
    /// The player has been invited but has yet to answer the invitation.
    Invitee = 0,
    /// The player is a member of this CWLS.
    Member = 1,
    /// The player is a leader (silver star/moderator) in this CWLS.
    Leader = 2,
    /// The player is the master of this CWLS.
    Master = 3,
}

/// Represents the CWLS's id number and ChatChannel. This was added to help reduce copy paste in CrossworldLinkshell & CrossworldLinkshellEx.
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CWLSCommonIdentifiers {
    pub linkshell_id: u64,
    pub linkshell_chat_id: ChatChannel,
}

/// Represents the CWLS's name & permission rank info. This was added to help reduce copy paste in CrossworldLinkshell & CrossworldLinkshellEx.
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CWLSCommon {
    /// The client's rank in the CWLS.
    pub rank: CWLSPermissionRank,
    /// The CWLS's name.
    #[brw(pad_after = 7)] // Seems to be empty/zeroes
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)] // TODO: Likely only 20 characters like regular LSes, but this keeps the padding easier to follow
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

/// Represents data of a single CWLS. This version is used on login.
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CrossworldLinkshell {
    pub ids: CWLSCommonIdentifiers,
    /// The client's name and rank in the CWLS.
    pub common: CWLSCommon,
}

impl CrossworldLinkshell {
    pub const SIZE: usize = 56;
    pub const COUNT: usize = 8;
}

/// Represents data of a single CWLS. This extended version is used when the CWLS menu is opened.
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CrossworldLinkshellEx {
    pub ids: CWLSCommonIdentifiers,
    /// A 32-bit Unix timestmap indicating when this CWLS was created.
    #[brw(pad_after = 4)] // Seems to be empty/zeroes
    pub creation_time: u32,
    /// The client's name and rank in the CWLS.
    pub common: CWLSCommon,
}

impl CrossworldLinkshellEx {
    pub const SIZE: usize = 64;
    pub const COUNT: usize = 8;

    pub fn is_empty(&self) -> bool {
        // We don't consider the ChatChannel because there are valid use-cases on Kawari where it's not set to a valid channel, particularly in the database or when zone/chat connections are gathering their initial info.
        self.ids.linkshell_id == 0 && self.common.name == String::default()
    }
}

/// The result sent back to the client when they ask if a CWLS's name is available for use.
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum CWLSNameAvailability {
    #[default]
    Available = 0,
    NotAvailable = 1,
}

/// The reason this player is leaving the linkshell.
// TODO: This might actually just be a bitfield?
#[binrw]
#[brw(repr = u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, FromRepr)]
pub enum CWLSLeaveReason {
    /// This player is departing of their own volition.
    #[default]
    Leaving = 0,
    /// This player has been kicked.
    Kicked = 1,
    /// Unknown, also treated as kicked.
    KickedUnk = 2,
    /// This player declined an invite to the linkshell.
    DeclinedInvite = 3,
    /// This player was kicked while their rank was invitee.
    InviteWithdrawn = 4,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct CrossworldLinkshellInvite {
    pub linkshell_id: u64,
    pub execute_content_id: u64,
    pub target_content_id: u64,
    pub execute_world_id: u16,
    pub target_world_id: u16,
    pub unk1: u8, // Always 1?
    pub unk2: u8, // Always 1?
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub linkshell_name: String,
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub execute_name: String,
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 2)] // Seems to be empty/zeroes
    pub target_name: String,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LinkshellInviteResponse {
    #[default]
    Declined = 1,
    Accepted = 2,
}
