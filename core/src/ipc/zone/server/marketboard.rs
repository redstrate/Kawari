use binrw::binrw;

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

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn housing_ward_size() {
        ensure_size::<MarketBoardItem, { MarketBoardItem::SIZE }>();
    }
}
