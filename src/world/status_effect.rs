use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub struct StatusEffect {
    effect_id: u16,
    param: u16,
    duration: f32,
    source_actor_id: u32,
}
