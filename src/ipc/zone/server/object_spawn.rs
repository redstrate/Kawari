use binrw::binrw;

use crate::common::{Position, read_quantized_rotation, write_quantized_rotation};
use serde::Deserialize;

// TODO: this is all kinds of wrong, take the fields with a grain of salt
#[binrw]
#[brw(little)]
#[derive(Debug, Copy, Clone, Default, Deserialize)]
pub struct ObjectSpawn {
    pub index: u8,
    pub kind: u8,
    #[brw(pad_after = 1)] // padding, or part of flag?
    pub flag: u8,
    pub base_id: u32,
    pub entity_id: u32,
    pub layout_id: u32,
    pub content_id: u32,
    pub owner_id: u32,
    pub bind_layout_id: u32,
    pub scale: f32,
    pub shared_group_timeline_state: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub fate: u16,
    pub permission_invisibility: u8,
    pub args1: u8,
    pub args2: u32,
    pub args3: u32,
    pub unk1: u32,
    pub position: Position,
}
