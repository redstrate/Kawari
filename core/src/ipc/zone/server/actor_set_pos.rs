use binrw::binrw;

use crate::common::{Position, WarpType, read_quantized_rotation, write_quantized_rotation};

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
    pub arg: u8,
    /// Uses the ambient sound from this row in the TerritoryIntendedUse Excel sheet. Isn't used for anything else I think.
    pub transition_territory_filter_key: u32,
    /// The position to warp the player to.
    #[brw(pad_after = 4)] // padding, not read by the client
    pub position: Position,
}
