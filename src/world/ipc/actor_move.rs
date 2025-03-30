use binrw::binrw;

use crate::common::{Position, read_packed_position, write_packed_position};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorMove {
    pub dir: u8,
    pub dir_before_slip: u8,
    pub flag1: u8,
    pub flat2: u8,
    pub speed: u8,
    #[brw(pad_before = 1, pad_after = 4)] // empty
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
}
