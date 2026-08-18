use binrw::binrw;

use crate::{
    common::{
        ObjectId, ObjectTypeId, Position, read_packed_position, read_quantized_rotation,
        write_packed_position, write_quantized_rotation,
    },
    ipc::zone::{ServerZoneIpcData, ServerZoneIpcSegment},
};

#[binrw]
#[derive(Debug, Default, Clone)]
#[brw(import{max_targets: usize})]
#[brw(assert(target_ids.len() <= max_targets, "Too many targets! {} > {}", target_ids.len(), max_targets))]
pub struct AoeEffect {
    source_actor: ObjectId,
    unk1: u32,
    /// Index into the Action Excel sheet.
    pub action_id: u32,
    pub global_sequence: u16,
    duration: f32,
    unk3: u32,
    request_id: u16,
    unk: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// Usually the same as `action_id`.
    pub spell_id: u16,
    unk10: [u8; 3],
    #[br(temp)]
    #[bw(calc = target_ids.len() as u8)]
    target_count: u8,
    unk11: [u8; 14],
    #[br(count = 512)]
    #[brw(pad_size_to = 512)]
    effects: Vec<u8>,
    #[br(count = max_targets)]
    #[brw(pad_size_to = max_targets * 8)]
    target_ids: Vec<ObjectTypeId>,
    #[brw(pad_after = 6)] // empty
    #[br(map = read_packed_position)]
    #[bw(map = write_packed_position)]
    position: Position,
}

impl AoeEffect {
    pub fn package(&self) -> Option<ServerZoneIpcSegment> {
        match self.target_ids.len() {
            0..=8 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::AoeEffect8 {
                data: self.clone(),
            })),
            _ => None,
        }
    }
}
