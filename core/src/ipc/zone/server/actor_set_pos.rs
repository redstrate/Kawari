use binrw::binrw;

use crate::common::{Position, read_quantized_rotation, write_quantized_rotation};

#[binrw]
#[derive(Debug, Clone, Copy, Default)]
pub enum WarpType {
    /// Instantaneously change to this position.
    #[brw(magic = 0u8)]
    #[default]
    None,
    /// Unknown, needs a better name but fades out the screen.
    #[brw(magic = 2u8)]
    Normal,
    #[brw(magic = 5u8)]
    Unk1,
    #[brw(magic = 8u8)]
    Unk2,
    #[brw(magic = 19u8)]
    Unk3,
    /// Seen during Mt Gulg, assuming it applies to all instanced content.
    #[brw(magic = 25u8)]
    InstanceContent,
    #[brw(magic = 30u8)]
    Unk4,
    Unknown(u8),
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorSetPos {
    /// The direction to which the player should be facing.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// What kind of warp this is.
    pub warp_type: WarpType,
    /// Argument based on `warp_type`.
    pub warp_type_arg: u8,
    /// Unknown purpose.
    pub transition_territory_filter_key: u32,
    /// The position to warp the player to.
    #[brw(pad_after = 4)] // padding
    pub position: Position,
}
