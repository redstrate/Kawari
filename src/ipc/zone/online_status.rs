use binrw::binrw;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::common::value_to_flag_byte_index_value;

// See https://github.com/aers/FFXIVClientStructs/blob/28d9f0f77fdf388f596ba65768c7d6441e962d06/FFXIVClientStructs/FFXIV/Client/UI/Info/InfoProxyCommonList.cs#L86
// TODO: This entire enum seems to be used as both literal values (e.g. in PlayerSpawn, where a single byte indicates status) and shift values as a u64 for the SocialList (possibly other places too).
#[binrw]
#[brw(little)]
#[brw(repr = u8)]
#[derive(Clone, Debug, Default, EnumIter, Eq, PartialEq)]
pub enum OnlineStatus {
    Offline = 0,
    GameQA = 1,
    GameMaster = 2,
    GameMasterBlue = 3,
    EventParticipant = 4,
    Disconnected = 5,
    WaitingForFriendListApproval = 6,
    WaitingForLinkshellApproval = 7,
    WaitingForFreeCompanyApproval = 8,
    NotFound = 9,
    OfflineExd = 10,
    BattleMentor = 11,
    Busy = 12,
    PvP = 13,
    PlayingTripleTriad = 14,
    ViewingCutscene = 15,
    UsingAChocoboPorter = 16,
    AwayFromKeyboard = 17,
    CameraMode = 18,
    LookingForRepairs = 19,
    LookingToRepair = 20,
    LookingToMeldMateria = 21,
    RolePlaying = 22,
    LookingForParty = 23,
    SwordForHire = 24,
    WaitingForDutyFinder = 25,
    RecruitingPartyMembers = 26,
    Mentor = 27,
    PvEMentor = 28,
    TradeMentor = 29,
    PvPMentor = 30,
    Returner = 31,
    NewAdventurer = 32,
    AllianceLeader = 33,
    AlliancePartyLeader = 34,
    AlliancePartyMember = 35,
    PartyLeader = 36,
    PartyMember = 37,
    PartyLeaderCrossWorld = 38,
    PartyMemberCrossWorld = 39,
    AnotherWorld = 40,
    SharingDuty = 41,
    SimilarDuty = 42,
    InDuty = 43,
    TrialAdventurer = 44,
    FreeCompany = 45,
    GrandCompany = 46,
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
        write!(f, "OnlineStatusMask {:#?} ({:#?})", self.flags, self.mask())
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
