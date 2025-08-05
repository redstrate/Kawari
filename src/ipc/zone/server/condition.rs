use binrw::binrw;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};

use crate::common::value_to_flag_byte_index_value;

#[repr(u32)]
#[derive(Debug, PartialEq, EnumIter, Clone, Copy, EnumString)]
pub enum Condition {
    None = 0,
    /// Seen when beginning a walk-in event.
    WalkInEvent = 6,
    /// When the client is logging out.
    LoggingOut = 25,
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

    pub fn set_condition(&mut self, condition: Condition) {
        let (value, index) = value_to_flag_byte_index_value(condition as u32);
        self.flags[index as usize] |= value;
    }

    pub fn remove_condition(&mut self, condition: Condition) {
        let (value, index) = value_to_flag_byte_index_value(condition as u32);
        self.flags[index as usize] ^= value;
    }
}

impl std::fmt::Debug for Conditions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Conditions {:#?} ({:#?})", self.flags, self.conditions())
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
