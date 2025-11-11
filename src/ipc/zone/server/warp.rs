use binrw::binrw;

use crate::common::{Position, read_quantized_rotation, write_quantized_rotation};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Warp {
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub dir: f32,
    pub warp_type: u8,
    pub warp_type_arg: u8,
    pub layer_set: u32,
    #[brw(pad_after = 4)] // padding
    pub position: Position,
}
