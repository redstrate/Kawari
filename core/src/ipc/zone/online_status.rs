use binrw::binrw;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, FromRepr};

use crate::common::value_to_flag_byte_index_value;

// See https://github.com/aers/FFXIVClientStructs/blob/28d9f0f77fdf388f596ba65768c7d6441e962d06/FFXIVClientStructs/FFXIV/Client/UI/Info/InfoProxyCommonList.cs#L86
// TODO: This entire enum seems to be used as both literal values (e.g. in PlayerSpawn, where a single byte indicates status) and shift values as a u64 for the SocialList (possibly other places too).
#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[repr(u8)]
#[derive(Clone, Debug, Default, EnumIter, Eq, PartialEq, FromRepr)]
pub enum OnlineStatus {
    /// No icon is shown at all.
    Offline = 0,
    /// Blue GM icon.
    GameQA = 1,
    /// Orange GM icon.
    GameMaster = 2,
    /// Blue GM icon.
    GameMasterBlue = 3,
    /// Yellow ARR icon.
    EventParticipant = 4,
    /// Disconnected icon.
    Disconnected = 5,
    /// Yellow smiley hexagon icon.
    WaitingForFriendListApproval = 6,
    /// Yellow smiley hexagon icon.
    WaitingForLinkshellApproval = 7,
    /// Yellow smiley hexagon icon.
    WaitingForFreeCompanyApproval = 8,
    /// Red cross icon.
    NotFound = 9,
    /// Transparent, dim offline icon.
    OfflineExd = 10,
    /// Sword and crown mentor icon.
    BattleMentor = 11,
    /// Red worried face icon.
    Busy = 12,
    /// Crossed swords icon.
    PvP = 13,
    /// Triple triad icon.
    PlayingTripleTriad = 14,
    /// Cutscene icon.
    ViewingCutscene = 15,
    /// Yellow chocobo icon.
    UsingAChocoboPorter = 16,
    /// Red chair icon.
    AwayFromKeyboard = 17,
    /// Camera icon.
    CameraMode = 18,
    /// Yellow repair hexagon icon.
    LookingForRepairs = 19,
    /// Yellow repair icon.
    LookingToRepair = 20,
    /// Yellow materia hexagon icon.
    LookingToMeldMateria = 21,
    /// Roleplaying icon.
    RolePlaying = 22,
    /// Green looking for party hexagon icon.
    LookingForParty = 23,
    /// Blue sword hexagon icon.
    SwordForHire = 24,
    /// Blue duty finder icon.
    WaitingForDutyFinder = 25,
    /// Blue recruiting hexagon icon.
    RecruitingPartyMembers = 26,
    /// Crown icon.
    Mentor = 27,
    /// Sword and crown icon.
    PvEMentor = 28,
    /// Hammer and crown icon.
    TradeMentor = 29,
    /// Flag and crown icon.
    PvPMentor = 30,
    /// Crown and leaf icon.
    Returner = 31,
    /// Sprout icon.
    NewAdventurer = 32,
    /// Yellow "AL" icon.
    AllianceLeader = 33,
    /// Blue "AL" icon.
    AlliancePartyLeader = 34,
    /// Blue "A" icon.
    AlliancePartyMember = 35,
    /// ???
    PartyLeader = 36,
    /// ???
    PartyMember = 37,
    /// Purple "PL" icon.
    PartyLeaderCrossWorld = 38,
    /// Purple flag icon.
    PartyMemberCrossWorld = 39,
    /// Red boxed play icon.
    AnotherWorld = 40,
    /// Yellow boxed play icon.
    SharingDuty = 41,
    /// Dim, transparent boxed play icon.
    SimilarDuty = 42,
    /// Red crossed swords icon.
    InDuty = 43,
    /// ???
    TrialAdventurer = 44,
    /// ???
    FreeCompany = 45,
    /// ???
    GrandCompany = 46,
    /// Online, no icon.
    #[default]
    Online = 47,
}

/// Represents a 64-bit online status. For possible values, see common_spawn.rs's OnlineStatus enum.
#[binrw]
#[brw(little)]
#[derive(Clone, Copy, Default)]
pub struct OnlineStatusMask {
    flags: [u8; 8],
}

impl From<[u8; 8]> for OnlineStatusMask {
    fn from(flags: [u8; 8]) -> Self {
        Self { flags }
    }
}

impl OnlineStatusMask {
    pub fn mask(&self) -> Vec<OnlineStatus> {
        let mut statuses = Vec::new();

        for status in OnlineStatus::iter() {
            let (value, index) = value_to_flag_byte_index_value(status.clone() as u32);
            if self.flags[index as usize] & value == value {
                statuses.push(status);
            }
        }
        statuses
    }

    pub fn set_status(&mut self, status: OnlineStatus) {
        let (value, index) = value_to_flag_byte_index_value(status as u32);
        self.flags[index as usize] |= value;
    }

    pub fn remove_status(&mut self, status: OnlineStatus) {
        let (value, index) = value_to_flag_byte_index_value(status as u32);
        self.flags[index as usize] ^= value;
    }
}

impl std::fmt::Debug for OnlineStatusMask {
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
    fn read_onlinestatus_duty() {
        let mask: [u8; 8] = [0, 0, 0, 0, 0, 129, 0, 0];
        assert_eq!(
            OnlineStatusMask::from(mask).mask(),
            vec![OnlineStatus::AnotherWorld, OnlineStatus::Online]
        );
    }

    #[test]
    fn read_onlinestatus_normal() {
        let mask: [u8; 8] = [0, 0, 0, 0, 0, 128, 0, 0];
        assert_eq!(
            OnlineStatusMask::from(mask).mask(),
            vec![OnlineStatus::Online]
        );
    }
}
