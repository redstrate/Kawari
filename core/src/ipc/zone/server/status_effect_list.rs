use binrw::binrw;

use super::StatusEffect;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusEffectList {
    pub classjob_id: u8,
    pub level: u8,
    pub unk1: u8,
    pub unk2: u8,
    pub curr_hp: u32,
    pub max_hp: u32,
    pub curr_mp: u16,
    pub max_mp: u16,
    pub shield: u16,
    pub unk3: u16,
    pub statues: [StatusEffect; 30],
    pub unk4: u32,
}
