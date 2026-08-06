use binrw::binrw;

use crate::common::{CHAR_NAME_MAX_LENGTH, read_string, write_string};

// TODO: what about favorites and wishlists?
#[binrw]
#[derive(Debug, Clone, Default)]
pub struct MarketBoardItem {
    /// Index into the Item Excel sheet.
    pub item_id: u32,
    /// How many offers of this item are available.
    pub count: u32,
}

impl MarketBoardItem {
    pub const SIZE: usize = 8;
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct MarketBoardHistoryEntry {
    pub unk1: u16,
    /// In Gil.
    pub price: u32,
    pub timestamp: u32, // timestamp?
    /// How many items were sold.
    pub quantity: u32,
    pub unk3: u16,
    #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

impl MarketBoardHistoryEntry {
    pub const SIZE: usize = 48;
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct MarketBoardHistory {
    /// Index into the Item Excel sheet.
    pub item_id: u16,
    #[brw(pad_after = 6)] // padding
    #[br(count = 20)]
    #[brw(pad_size_to = MarketBoardHistoryEntry::SIZE * 20)]
    pub entries: Vec<MarketBoardHistoryEntry>,
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn marketboard_item_size() {
        ensure_size::<MarketBoardItem, { MarketBoardItem::SIZE }>();
    }

    #[test]
    fn marketboard_history_size() {
        ensure_size::<MarketBoardHistoryEntry, { MarketBoardHistoryEntry::SIZE }>();
    }
}
