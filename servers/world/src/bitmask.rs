use std::marker::PhantomData;

use diesel::{
    backend::Backend,
    deserialize::{self, FromSqlRow},
    expression::AsExpression,
    serialize::{self},
    sql_types::Text,
    sqlite::Sqlite,
};

use kawari::common::{value_to_flag_byte_index_value, value_to_flag_byte_index_value_quests};

pub trait BitmaskTransformation {
    fn transform_value(value: u32) -> (u8, u16);
}

#[derive(Debug, Clone)]
pub struct NormalBitmaskTransformation {}

impl BitmaskTransformation for NormalBitmaskTransformation {
    fn transform_value(value: u32) -> (u8, u16) {
        value_to_flag_byte_index_value(value)
    }
}

#[derive(Debug, Clone)]
pub struct QuestBitmaskTransformation {}

impl BitmaskTransformation for QuestBitmaskTransformation {
    fn transform_value(value: u32) -> (u8, u16) {
        value_to_flag_byte_index_value_quests(value)
    }
}

#[derive(Debug, Clone, AsExpression, FromSqlRow)]
#[diesel(sql_type = Text)]
pub struct GenericBitmask<const N: usize, T: BitmaskTransformation + std::fmt::Debug> {
    pub data: Vec<u8>,
    _phantom: PhantomData<T>,
}

impl<const N: usize, T: BitmaskTransformation + std::fmt::Debug> serialize::ToSql<Text, Sqlite>
    for GenericBitmask<N, T>
{
    fn to_sql<'b>(&'b self, out: &mut serialize::Output<'b, '_, Sqlite>) -> serialize::Result {
        out.set_value(serde_json::to_string(&self.data).unwrap());
        Ok(serialize::IsNull::No)
    }
}

impl<const N: usize, T: BitmaskTransformation + std::fmt::Debug> deserialize::FromSql<Text, Sqlite>
    for GenericBitmask<N, T>
{
    fn from_sql(mut bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        Ok(Self::from(
            serde_json::from_str::<Vec<u8>>(bytes.read_text())
                .ok()
                .unwrap(),
        ))
    }
}

impl<const N: usize, T: BitmaskTransformation + std::fmt::Debug> From<Vec<u8>>
    for GenericBitmask<N, T>
{
    fn from(value: Vec<u8>) -> Self {
        Self {
            data: value,
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, T: BitmaskTransformation + std::fmt::Debug> Default for GenericBitmask<N, T> {
    fn default() -> Self {
        Self {
            data: vec![0; N],
            _phantom: PhantomData,
        }
    }
}

impl<const N: usize, T: BitmaskTransformation + std::fmt::Debug> GenericBitmask<N, T> {
    /// Sets this specific `value`.
    pub fn set(&mut self, value: u32) {
        let (value, index) = T::transform_value(value);
        if (index as usize) < self.data.len() {
            self.data[index as usize] |= value;
        } else {
            tracing::warn!(
                "Failed to set bitmask: {index} despite {}?!",
                self.data.len()
            );
        }
    }

    /// Clears this specific `value`.
    pub fn clear(&mut self, value: u32) {
        let (value, index) = T::transform_value(value);
        self.data[index as usize] &= !value;
    }

    /// Toggles the `value`, and if wasn't previously set then this returns true. Otherwise false.
    pub fn toggle(&mut self, value: u32) -> bool {
        let previously_unset = !self.contains(value);

        let (value, index) = T::transform_value(value);
        self.data[index as usize] ^= value;

        previously_unset
    }

    /// Checks if this `value` is set.
    pub fn contains(&self, value: u32) -> bool {
        let (value, index) = T::transform_value(value);
        (self.data[index as usize] & value) == value
    }

    /// Sets all bits of this mask to 0xFF (255)
    pub fn set_all(&mut self) {
        self.data = vec![0xFF; N];
    }
}

pub type Bitmask<const N: usize> = GenericBitmask<N, NormalBitmaskTransformation>;
pub type QuestBitmask<const N: usize> = GenericBitmask<N, QuestBitmaskTransformation>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        bitmask.set(0);
        assert_eq!(bitmask.data, vec![1, 0, 0, 0]);

        bitmask.set(1);
        assert_eq!(bitmask.data, vec![3, 0, 0, 0]);
    }

    #[test]
    fn toggle_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        assert_eq!(bitmask.toggle(0), true);
        assert_eq!(bitmask.data, vec![1, 0, 0, 0]);

        assert_eq!(bitmask.toggle(0), false);
        assert_eq!(bitmask.data, vec![0, 0, 0, 0]);
    }

    #[test]
    fn clear_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        bitmask.set(0);
        bitmask.set(1);
        assert_eq!(bitmask.data, vec![3, 0, 0, 0]);

        bitmask.clear(0);
        assert_eq!(bitmask.data, vec![2, 0, 0, 0]);
        bitmask.clear(0); // Using clear again shouldn't do anything.
        assert_eq!(bitmask.data, vec![2, 0, 0, 0]);
    }

    #[test]
    fn contains_bitmask() {
        let mut bitmask: Bitmask<4> = Bitmask::default();

        bitmask.set(0);
        assert_eq!(bitmask.data, vec![1, 0, 0, 0]);
        assert_eq!(bitmask.contains(0), true);
        assert_eq!(bitmask.contains(1), false);

        bitmask.set(1);
        assert_eq!(bitmask.data, vec![3, 0, 0, 0]);
        assert_eq!(bitmask.contains(0), true);
        assert_eq!(bitmask.contains(1), true);
    }

    #[test]
    fn set_quest_bitmask() {
        let mut bitmask: QuestBitmask<4> = QuestBitmask::default();

        bitmask.set(0);
        assert_eq!(bitmask.data, vec![128, 0, 0, 0]);

        bitmask.set(1);
        assert_eq!(bitmask.data, vec![192, 0, 0, 0]);
    }

    #[test]
    fn toggle_quest_bitmask() {
        let mut bitmask: QuestBitmask<4> = QuestBitmask::default();

        assert_eq!(bitmask.toggle(0), true);
        assert_eq!(bitmask.data, vec![128, 0, 0, 0]);

        assert_eq!(bitmask.toggle(0), false);
        assert_eq!(bitmask.data, vec![0, 0, 0, 0]);
    }

    #[test]
    fn clear_quest_bitmask() {
        let mut bitmask: QuestBitmask<4> = QuestBitmask::default();

        bitmask.set(0);
        bitmask.set(1);
        assert_eq!(bitmask.data, vec![192, 0, 0, 0]);

        bitmask.clear(0);
        assert_eq!(bitmask.data, vec![64, 0, 0, 0]);
        bitmask.clear(0); // Using clear again shouldn't do anything.
        assert_eq!(bitmask.data, vec![64, 0, 0, 0]);
    }

    #[test]
    fn contains_quest_bitmask() {
        let mut bitmask: QuestBitmask<4> = QuestBitmask::default();

        bitmask.set(0);
        assert_eq!(bitmask.data, vec![128, 0, 0, 0]);
        assert_eq!(bitmask.contains(0), true);
        assert_eq!(bitmask.contains(1), false);

        bitmask.set(1);
        assert_eq!(bitmask.data, vec![192, 0, 0, 0]);
        assert_eq!(bitmask.contains(0), true);
        assert_eq!(bitmask.contains(1), true);
    }
}
