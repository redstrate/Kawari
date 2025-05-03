use binrw::binrw;

use crate::common::{
    Position, read_packed_position, read_packed_rotation_float, write_packed_position,
    write_packed_rotation_float,
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct Move {
    #[bw(map = write_packed_rotation_float)]
    #[br(map = read_packed_rotation_float)]
    pub rotation: f32,
    pub dir_before_slip: u8,
    pub flag1: u8,
    pub flag2: u8,
    pub speed: u8,
    #[brw(pad_before = 1)] // padding
    #[brw(pad_after = 4)] // empty
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
}
