use binrw::binrw;

use crate::common::{
    HandlerId, ObjectId, Position, read_packed_position, read_quantized_rotation,
    write_packed_position, write_quantized_rotation,
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SpawnTreasure {
    /// Index into the Treasure Excel sheet.
    pub base_id: u32,
    /// This entity's ID.
    pub entity_id: ObjectId,
    /// Game object instance ID in the level.
    pub layout_id: u32,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub spawn_index: u8,
    pub item_count: u8,
    pub event_state: u8,
    pub coffer_kind: u8,
    #[brw(pad_after = 1)] // empty?
    pub visibility: u8,
    pub countdown_time: f32,
    pub countdown_start_time: f32,
    pub claim_time: f32,
    pub handler_id: HandlerId,
    pub exported_sg_row_id: u32,
    #[brw(pad_after = 1)] // empty?
    pub targetable: u8,
    /// The position of this treasure.
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
    pub lootable_item_ids: [u32; 16],
}
