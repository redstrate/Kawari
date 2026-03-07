#![allow(unused_assignments)] // false positive caused by binrw

use binrw::binrw;

use crate::{
    common::HandlerId,
    ipc::zone::{ServerZoneIpcData, ServerZoneIpcSegment},
};

#[derive(Debug, Clone, Default)]
#[binrw]
#[brw(little)]
#[brw(import{max_params: usize})]
pub struct MapEffects {
    /// Should be the ID of the instance's director.
    pub handler_id: HandlerId,
    /// Unsure of what these flags mean.
    pub unk_flag: u16,
    /// The initial states each map effect should be in.
    #[br(count = max_params)]
    #[brw(pad_size_to = max_params * 2)]
    pub states: Vec<u16>,
    pub unk1: u16,
}

impl MapEffects {
    pub fn package(&self) -> Option<ServerZoneIpcSegment> {
        match self.states.len() {
            0..=64 => Some(ServerZoneIpcSegment::new(
                ServerZoneIpcData::DirectorSetupMapEffects64 { data: self.clone() },
            )),
            65..=128 => Some(ServerZoneIpcSegment::new(
                ServerZoneIpcData::DirectorSetupMapEffects128 { data: self.clone() },
            )),
            _ => None,
        }
    }
}
