use binrw::binrw;

use crate::common::Position;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorSetPos {
    pub dir: u16,
    pub warp_type: u8,
    pub warp_type_arg: u8,
    pub layer_set: u32,
    #[brw(pad_after = 4)] // padding
    pub position: Position,
}
