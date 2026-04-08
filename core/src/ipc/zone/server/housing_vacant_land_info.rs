use binrw::binrw;

use crate::ipc::zone::{AvailabilityType, PurchaseType, TenantType};

///Represents a vacant housing plot
#[binrw]
#[derive(Debug, Default, Clone)]
pub struct HousingVacantLandInfo {
    pub purchase_type: PurchaseType,
    pub tenant_type: TenantType,
    pub availability_type: AvailabilityType,
    pub unk1: u8,
    pub unk2: u32,
    pub phase_ends_at: u32,
    pub unk3: u32,
    pub entry_count: u32,

    /// First 12 bytes 0x0, Last 8 bytes always 0xFF
    pub unk4: [u8; 20],
}
