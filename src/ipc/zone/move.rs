use binrw::binrw;

use crate::common::{
    Position, read_packed_position, read_quantized_rotation, write_packed_position,
    write_quantized_rotation,
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Move {
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub flag1: u16,
    pub flag2: u16,
    #[brw(pad_after = 4)] // empty
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
}
