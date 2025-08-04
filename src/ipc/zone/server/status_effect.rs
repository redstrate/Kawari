use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusEffect {
    pub effect_id: u16,
    pub param: u16,
    pub duration: f32,
    pub source_actor_id: u32,
}
