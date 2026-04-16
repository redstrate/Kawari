use binrw::binrw;

use super::StatusEffect;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusEffectList {
    /// Index into the ClassJob Excel sheet.
    pub classjob_id: u8,
    /// The level of your current class.
    pub level: u8,
    pub unk1: u8,
    pub unk2: u8,
    /// Amount of health points.
    pub health_points: u32,
    /// Maximum amount of health points.
    pub max_health_points: u32,
    /// Amount of resource points (MP/CP/GP etc.)
    pub resource_points: u16,
    /// Maximum amount of resource points (MP/CP/GP etc.)
    pub max_resource_points: u16,
    pub shield: u16,
    pub unk3: u16,
    /// List of status effects for the player.
    pub statuses: [StatusEffect; 30],
    pub unk4: u32,
}
