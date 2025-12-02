use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusEffect {
    pub effect_id: u16,
    pub param: u16,
    pub duration: f32,
    pub source_actor_id: ObjectId,
}
