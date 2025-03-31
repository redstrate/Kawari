use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct PlayerStats {
    pub strength: u32,
    pub dexterity: u32,
    pub vitality: u32,
    pub intelligence: u32,
    pub mind: u32,
    pub piety: u32,
    pub hp: u32,
    pub mp: u32,
    pub tp: u32,
    pub gp: u32,
    pub cp: u32,
    pub delay: u32,
    pub tenacity: u32,
    pub attack_power: u32,
    pub defense: u32,
    pub direct_hit_rate: u32,
    pub evasion: u32,
    pub magic_defense: u32,
    pub critical_hit: u32,
    pub attack_magic_potency: u32,
    pub healing_magic_potency: u32,
    pub elemental_bonus: u32,
    pub determination: u32,
    pub skill_speed: u32,
    pub spell_speed: u32,
    pub haste: u32,
    pub craftmanship: u32,
    pub control: u32,
    pub gathering: u32,
    pub perception: u32,
    pub unk1: [u32; 6],
}
