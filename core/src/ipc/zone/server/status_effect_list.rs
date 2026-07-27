use binrw::binrw;

use super::StatusEffect;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusEffectList {
    /// Index into the ClassJob Excel sheet.
    pub classjob_id: u8,
    /// The level of your current class.
    #[brw(pad_after = 1)] // is probably synced level? but it's not read by the client soooo
    pub level: u8,
    pub flags: u8,
    /// Amount of health points.
    pub health_points: u32,
    /// Maximum amount of health points.
    pub max_health_points: u32,
    /// Amount of resource points (MP/CP/GP etc.)
    pub resource_points: u16,
    /// Maximum amount of resource points (MP/CP/GP etc.)
    pub max_resource_points: u16,
    #[brw(pad_after = 3)] // not read by the client
    pub shield: u8,
    /// List of status effects for the player.
    #[brw(pad_after = 4)] // not read by the client
    pub statuses: [StatusEffect; 30],
}
