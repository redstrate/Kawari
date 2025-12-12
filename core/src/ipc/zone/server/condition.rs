use binrw::binrw;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};

use crate::common::value_to_flag_byte_index_value;

// These names are derived from https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/Conditions.cs
#[repr(u32)]
#[derive(Debug, PartialEq, EnumIter, Clone, Copy, EnumString)]
pub enum Condition {
    Occupied = 0,
    InCombat = 1,
    Casting = 2,
    SufferingStatusAffliction = 3,
    SufferingStatusAffliction2 = 4,
    Occupied30 = 5,
    /// Also ween when beginning a walk-in event.
    OccupiedInEvent = 6,
    OccupiedInQuestEvent = 7,
    Occupied33 = 8,
    BoundByDuty = 9,
    OccupiedInCutSceneEvent = 10,
    InDuelingArea = 11,
    TradeOpen = 12,
    Occupied38 = 13,
    Occupied39 = 14,
    ExecutingCraftingAction = 15,
    PreparingToCraft = 16,
    /// Also seen when the client starts watching a Duty Recorder replay?
    ExecutingGatheringAction = 17,
    Fishing = 18,
    Unknown44 = 19,
    BetweenAreas = 20,
    Stealthed = 21,
    UsingChocoboTaxi = 22,
    OccupiedSummoningBell = 23,
    BetweenAreas51 = 24,
    /// When the client is logging out.
    LoggingOut = 25,
    ConditionLocation = 26,
    WaitingForDuty = 27,
    BoundByDuty56 = 28,
    WatchingCutscene = 29,
    WaitingForDutyFinder = 30,
    CreatingCharacter = 31,
    Jumping61 = 32,
    PvPDisplayActive = 33,
    SufferingStatusAffliction63 = 34,
    Mounting = 35,
    CarryingItem = 36,
    UsingPartyFinder = 37,
    UsingHousingFunctions = 38,
    Transformed = 39,
    OnFreeTrial = 40,
    BeingMoved = 41,
    Mounting71 = 42,
    SufferingStatusAffliction72 = 43,
    SufferingStatusAffliction73 = 44,
    RegisteringForRaceOrMatch = 45,
    WaitingForTripleTriadMatch = 46,
    InFlight = 47,
    WatchingCutscene78 = 48,
    InDeepDungeon = 49,
    Swimming = 50,
    Diving = 51,
    RegisteringForTripleTriadMatch = 52,
    WaitingForTripleTriadMatch83 = 53,
    ParticipatingInCrossWorldPartyOrAlliance = 54,
    Unknown85 = 55,
    DutyRecorderPlayback = 56,
    Casting87 = 57,
    MountImmobile = 58,
    InThisState89 = 59,
    RolePlaying = 60,
    InDutyQueue = 61,
    ReadyingVisitOtherWorld = 62,
    WaitingToVisitOtherWorld = 63,
    UsingFashionAccessory = 64,
    BoundByDuty95 = 65,
    Unknown96 = 66,
    Disguised = 67,
    RecruitingWorldOnly = 68,
    Unknown99 = 69,
    Unknown101 = 70,
    PilotingMech = 71,
}

#[binrw]
#[brw(little)]
#[derive(Default, Clone, Copy)]
pub struct Conditions {
    #[brw(pad_after = 4)] // padding
    flags: [u8; 12],
}

impl Conditions {
    pub fn conditions(&self) -> Vec<Condition> {
        let mut conditions = Vec::new();

        for condition in Condition::iter() {
            let (value, index) = value_to_flag_byte_index_value(condition as u32);
            if self.flags[index as usize] & value == value {
                conditions.push(condition);
            }
        }

        conditions
    }

    pub fn toggle_condition(&mut self, condition: Condition, set: bool) {
        if set {
            self.set_condition(condition);
        } else {
            self.remove_condition(condition);
        }
    }

    pub fn set_condition(&mut self, condition: Condition) {
        let (value, index) = value_to_flag_byte_index_value(condition as u32);
        self.flags[index as usize] |= value;
    }

    pub fn remove_condition(&mut self, condition: Condition) {
        let (value, index) = value_to_flag_byte_index_value(condition as u32);
        self.flags[index as usize] &= !value;
    }

    pub fn has_condition(&self, condition: Condition) -> bool {
        let (value, index) = value_to_flag_byte_index_value(condition as u32);
        (self.flags[index as usize] & value) == value
    }
}

impl std::fmt::Debug for Conditions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.flags.iter().all(|x| *x == 0) {
            return write!(f, "None");
        }

        if self.conditions().is_empty() {
            write!(f, "{:#?}", self.flags)
        } else {
            write!(f, "{:?}", self.conditions())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::{BinRead, BinWrite};

    use super::*;

    fn parse_conditions(bytes: &[u8]) -> Conditions {
        let mut buffer = Cursor::new(&bytes);
        Conditions::read_le(&mut buffer).unwrap()
    }

    fn write_conditions(conditions: &Conditions) -> Vec<u8> {
        let mut buffer = Cursor::new(Vec::new());
        conditions.write_le(&mut buffer).unwrap();

        buffer.into_inner()
    }

    #[test]
    fn parse_condition() {
        let bytes = [0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0];

        assert_eq!(
            parse_conditions(&bytes).conditions(),
            vec![Condition::LoggingOut]
        );
    }

    #[test]
    fn set_condition() {
        let mut conditions = Conditions::default();
        conditions.set_condition(Condition::LoggingOut);

        let current_bytes = write_conditions(&conditions);

        let expected_bytes = [
            0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, // padding
            0, 0, 0,
        ];
        assert_eq!(current_bytes, expected_bytes);
    }

    #[test]
    fn remove_condition() {
        let mut conditions = Conditions::default();
        conditions.set_condition(Condition::LoggingOut);
        conditions.remove_condition(Condition::LoggingOut);

        let current_bytes = write_conditions(&conditions);

        let expected_bytes = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // padding
            0, 0, 0,
        ];
        assert_eq!(current_bytes, expected_bytes);
    }
}
