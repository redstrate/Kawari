use binrw::binrw;

use crate::common::{
    HandlerId, ObjectId, Position, read_bool_from, read_packed_position, read_quantized_rotation,
    write_bool_as, write_packed_position, write_quantized_rotation,
};

#[binrw]
#[derive(Debug, Clone, Default, Copy, PartialEq)]
#[brw(repr = u8)]
#[repr(u8)]
pub enum TreasureKind {
    #[default]
    Unknown = 0,
    Levequest = 1,
    DungeonRaid = 2,
    Unk3 = 3,
    TreasureHunt = 4,
    /// Variant, Occult Crescent, etc.
    PersonalLoot = 5,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct SpawnTreasure {
    /// Index into the Treasure Excel sheet.
    pub base_id: u32,
    /// This entity's ID.
    pub entity_id: ObjectId,
    /// Instance ID of the Treasure in the LGB.
    pub layout_id: u32,
    /// The rotation to create the object, in radians.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// The object's spawn index. Note that this is a completely separate index from actors.
    pub spawn_index: u8,
    pub item_count: u8,
    pub event_state: u8,
    pub coffer_kind: TreasureKind,
    #[brw(pad_after = 1)] // empty?
    /// Whether this object is initially hidden or not.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub is_hidden: bool,
    /// Starts counting down in seconds from `CountdownStartTime` once spawned.
    pub countdown_time: f32,
    /// The starting value of `CountdownTime` (in seconds) at initial object spawn.
    pub countdown_start_time: f32,
    /// The number of seconds available after opening the treasure to be able to roll on or assign drops before auto-disposition of loot.
    pub claim_time: f32,
    /// The event handler that owns this object.
    pub handler_id: HandlerId,
    pub exported_sg_row_id: u32,
    #[brw(pad_after = 1)] // empty?
    /// Whether this object should be targetable or not.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub not_targetable: bool,
    /// The position of this treasure.
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    pub position: Position,
    pub lootable_item_ids: [u32; 16],
}
