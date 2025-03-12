use binrw::binrw;

use super::Position;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct InitZone {
    pub server_id: u16,
    pub zone_id: u16,
    pub zone_index: u16,
    pub content_finder_condition_id: u16,
    pub layer_set_id: u32,
    pub layout_id: u32,
    pub weather_id: u32,
    pub unk_bitmask1: u8,
    pub unk_bitmask2: u8,
    pub unk1: u8,
    pub unk2: u32,
    pub festival_id: u16,
    pub additional_festival_id: u16,
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
    pub unk6: [u32; 4],
    pub unk7: [u32; 3],
    pub position: Position,
    pub unk8: [u32; 4],
    pub unk9: u32,
}
