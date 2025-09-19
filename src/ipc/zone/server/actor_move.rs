use binrw::binrw;

use crate::common::{
    MoveAnimationSpeed, MoveAnimationState, MoveAnimationType, Position, read_packed_position,
    read_quantized_rotation, write_packed_position, write_quantized_rotation,
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorMove {
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub anim_type: MoveAnimationType,
    pub anim_state: MoveAnimationState,
    #[brw(pad_after = 1)] // empty
    pub anim_speed: MoveAnimationSpeed,
    #[brw(pad_after = 4)] // empty
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
}
