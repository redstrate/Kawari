use crate::common::{
    CHAR_NAME_MAX_LENGTH, ObjectId, Position, read_packed_position, read_string,
    write_packed_position, write_string,
};
use crate::ipc::zone::StatusEffect;
use binrw::binrw;

#[binrw]
#[brw(repr = u16)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PartyUpdateStatus {
    #[default]
    None = 0,
    JoinParty = 1,
    PromoteLeader = 2,
    DisbandingParty = 3,
    MemberKicked = 4,
    SelfKicked = 5, // TODO: What is this?
    MemberLeftParty = 6,
    SelfLeftParty = 7,
    MemberChangedZones = 8,
    Unknown = 9, // TODO: This hasn't been observed yet, but it's included for completeness in case it does exist.
    MemberWentOffline = 0xA,
    MemberReturned = 0xB,
    PartyLeaderWentOffline = 0x12, // While this does get used on retail, we don't use it ourselves.
}

// TODO: This should maybe be moved to a more common place since it encompasses all (?) invite types?
#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum InviteType {
    #[default]
    Party = 1,
    FriendList = 2,
    // TODO: This probably also includes linkshells/cwls, free companies, and maybe novice network, but more captures are needed
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum InviteReply {
    #[default]
    Declined = 0,
    Accepted = 1,
    Cancelled = 2,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum InviteUpdateType {
    #[default]
    NewInvite = 1,
    InviteCancelled = 2,
    JoinedParty = 3,
    InviteAccepted = 4,
    InviteDeclined = 5,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct PartyMemberEntry {
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    #[brw(pad_before = 8)] // empty
    pub account_id: u64,
    pub content_id: u64,
    pub actor_id: ObjectId,
    pub entity_id: ObjectId,
    pub parent_id: ObjectId,
    pub current_hp: u32,
    pub max_hp: u32,
    pub current_mp: u16,
    pub max_mp: u16,
    pub home_world_id: u16,
    pub current_zone_id: u16,
    pub unk1: u8,
    pub classjob_id: u8,
    pub unk2: u8,
    pub classjob_level: u8,
    #[brw(pad_after = 8)] // empty
    pub status_effects: [StatusEffect; 30],
}

impl PartyMemberEntry {
    pub const SIZE: usize = 456;
    pub const NUM_ENTRIES: usize = 8;
}

// TODO: Move these position-related structs elsewhere in an eventual refactor
#[binrw]
#[derive(Clone, Debug, Default)]
pub struct MemberPosition {
    #[brw(pad_after = 1)]
    pub valid: u8, // Assumed, it's what Sapphire calls it. Seems to be set to 1 when there's position info, and 0 when there's not.
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub pos: Position,
}

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct PartyMemberPositions {
    pub positions: [MemberPosition; PartyMemberEntry::NUM_ENTRIES],
}
