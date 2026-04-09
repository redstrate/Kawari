use binrw::binrw;

use crate::{
    common::{LandId, read_string, write_string},
    ipc::zone::{HousingAppealTag, HousingFlags, PurchaseType, TenantType},
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct HousingWardInfo {
    pub land_id: LandId,

    #[br(count = 60)]
    #[bw(pad_size_to = 60 * HousingWardSummaryItem::SIZE)]
    pub house_summaries: Vec<HousingWardSummaryItem>,

    pub purchase_type: PurchaseType,
    pub unk1: u8,
    pub tenant_type: TenantType,

    #[brw(pad_after = 1)]
    pub terminator: u32,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct HousingWardSummaryItem {
    pub plot_price: u32,
    pub flags: HousingFlags,
    pub tags: [HousingAppealTag; 3],
    #[brw(pad_size_to = 32)]
    #[br(count = 32)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
}

impl HousingWardSummaryItem {
    pub const SIZE: usize = 40;
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn housing_ward_size() {
        ensure_size::<HousingWardSummaryItem, { HousingWardSummaryItem::SIZE }>();
    }
}
