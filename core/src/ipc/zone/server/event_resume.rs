#![allow(unused_assignments)] // false positive caused by binrw

use binrw::binrw;

use crate::common::HandlerId;
use crate::ipc::zone::server::{ServerZoneIpcData, ServerZoneIpcSegment};

#[derive(Debug, Clone, Default)]
#[binrw]
#[brw(little)]
#[brw(import{max_params: usize})]
#[brw(assert(params.len() <= max_params, "Too many params! {} > {}", params.len(), max_params))]
pub struct EventResume {
    pub handler_id: HandlerId,
    pub scene: u16,
    /// Seems to be a custom ID (handled internally by an EventHandler on the client.)
    pub resume_id: u8,
    pub params_count: u8,
    #[br(count = max_params)]
    #[bw(pad_size_to = 4 * max_params)]
    pub params: Vec<u32>,
}

impl EventResume {
    pub fn package_resume(&self) -> Option<ServerZoneIpcSegment> {
        match self.params.len() {
            // TODO: it would be nice to avoid cloning if possible
            0..=2 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventResume2 {
                data: self.clone(),
            })),
            3..=4 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventResume4 {
                data: self.clone(),
            })),
            5..=8 => Some(ServerZoneIpcSegment::new(ServerZoneIpcData::EventResume8 {
                data: self.clone(),
            })),
            _ => None,
        }
    }
}
