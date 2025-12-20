use diesel::{
    backend::Backend,
    deserialize::{self, FromSqlRow},
    expression::AsExpression,
    serialize::{self},
    sql_types::Text,
    sqlite::Sqlite,
};
use serde::{Deserialize, Serialize};

use kawari::common::{value_to_flag_byte_index_value, value_to_flag_byte_index_value_quests};

#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct Bitmask<const N: usize>(pub Vec<u8>);

impl<const N: usize> serialize::ToSql<Text, Sqlite> for Bitmask<N> {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(&self).unwrap());
        Ok(serialize::IsNull::No)
    }
}

impl<const N: usize> deserialize::FromSql<Text, Sqlite> for Bitmask<N> {
    fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(serde_json::from_str(bytes.read_text()).ok().unwrap())
    }
}

impl<const N: usize> From<Vec<u8>> for Bitmask<N> {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl<const N: usize> Default for Bitmask<N> {
    fn default() -> Self {
        Self(vec![0; N])
    }
}

impl<const N: usize> Bitmask<N> {
    /// Sets this specific `value`.
    pub fn set(&mut self, value: u32) {
        let (value, index) = value_to_flag_byte_index_value(value);
        self.0[index as usize] |= value;
    }

    /// Clears this specific `value`.
    pub fn clear(&mut self, value: u32) {
        let (value, index) = value_to_flag_byte_index_value(value);
        self.0[index as usize] &= !value;
    }

    /// Toggles the `value`, and if wasn't previously set then this returns true. Otherwise false.
    pub fn toggle(&mut self, value: u32) -> bool {
        let previously_unset = !self.contains(value);

        let (value, index) = value_to_flag_byte_index_value(value);
        self.0[index as usize] ^= value;

        previously_unset
    }

    /// Checks if this `value` is set.
    pub fn contains(&self, value: u32) -> bool {
        let (value, index) = value_to_flag_byte_index_value(value);
        (self.0[index as usize] & value) == value
    }

    /// Sets all bits of this mask to 0xFF (255)
    pub fn set_all(&mut self) {
        self.0 = vec![0xFF; N];
    }
}

// TODO: de-duplicate the two implementations, this is stupid
#[derive(Debug, Clone, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct QuestBitmask<const N: usize>(pub Vec<u8>);

impl<const N: usize> serialize::ToSql<Text, Sqlite> for QuestBitmask<N> {
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(&self).unwrap());
        Ok(serialize::IsNull::No)
    }
}

impl<const N: usize> deserialize::FromSql<Text, Sqlite> for QuestBitmask<N> {
    fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(serde_json::from_str(bytes.read_text()).ok().unwrap())
    }
}

impl<const N: usize> From<Vec<u8>> for QuestBitmask<N> {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl<const N: usize> Default for QuestBitmask<N> {
    fn default() -> Self {
        Self(vec![0; N])
    }
}

impl<const N: usize> QuestBitmask<N> {
    /// Sets this specific `value`.
    pub fn set(&mut self, value: u32) {
        let (value, index) = value_to_flag_byte_index_value_quests(value);
        self.0[index as usize] |= value;
    }

    /// Clears this specific `value`.
    pub fn clear(&mut self, value: u32) {
        let (value, index) = value_to_flag_byte_index_value_quests(value);
        self.0[index as usize] &= !value;
    }

    /// Toggles the `value`, and if wasn't previously set then this returns true. Otherwise false.
    pub fn toggle(&mut self, value: u32) -> bool {
        let previously_unset = !self.contains(value);

        let (value, index) = value_to_flag_byte_index_value_quests(value);
        self.0[index as usize] ^= value;

        previously_unset
    }

    /// Checks if this `value` is set.
    pub fn contains(&self, value: u32) -> bool {
        let (value, index) = value_to_flag_byte_index_value_quests(value);
        (self.0[index as usize] & value) == value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        bitmask.set(0);
        assert_eq!(bitmask.0, vec![1, 0, 0, 0]);

        bitmask.set(1);
        assert_eq!(bitmask.0, vec![3, 0, 0, 0]);
    }

    #[test]
    fn toggle_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        assert_eq!(bitmask.toggle(0), true);
        assert_eq!(bitmask.0, vec![1, 0, 0, 0]);

        assert_eq!(bitmask.toggle(0), false);
        assert_eq!(bitmask.0, vec![0, 0, 0, 0]);
    }

    #[test]
    fn clear_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        bitmask.set(0);
        bitmask.set(1);
        assert_eq!(bitmask.0, vec![3, 0, 0, 0]);

        bitmask.clear(0);
        assert_eq!(bitmask.0, vec![2, 0, 0, 0]);
        bitmask.clear(0); // Using clear again shouldn't do anything.
        assert_eq!(bitmask.0, vec![2, 0, 0, 0]);
    }

    #[test]
    fn contains_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        bitmask.set(0);
        assert_eq!(bitmask.0, vec![1, 0, 0, 0]);
        assert_eq!(bitmask.contains(0), true);
        assert_eq!(bitmask.contains(1), false);

        bitmask.set(1);
        assert_eq!(bitmask.0, vec![3, 0, 0, 0]);
        assert_eq!(bitmask.contains(0), true);
        assert_eq!(bitmask.contains(1), true);
    }
}
