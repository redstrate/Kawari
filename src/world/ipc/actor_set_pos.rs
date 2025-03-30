use binrw::binrw;

use crate::common::Position;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorSetPos {
    pub unk: u32,
    pub layer_id: u32,
    pub position: Position,
    pub unk3: u32,
}
