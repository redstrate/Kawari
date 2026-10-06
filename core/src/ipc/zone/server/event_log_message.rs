#![allow(unused_assignments)] // false positive caused by binrw

use crate::common::HandlerId;
use crate::ipc::zone::server::{ServerZoneIpcData, ServerZoneIpcSegment};
use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Default)]
#[brw(import{max_params: usize})]
#[brw(assert(params.len() <= max_params, "Too many params! {} > {}", params.len(), max_params))]
pub struct EventLogMessage {
    /// Which event sent this message.
    pub handler_id: HandlerId,
    /// Index into the LogMessage Excel sheet.
    pub message_type: u32,
    #[brw(pad_after = 3)]
    #[br(temp)]
    #[bw(calc = params.len() as u8)]
    params_count: u8,
    #[brw(pad_after = 4)]
    #[br(count = params_count)]
    #[brw(pad_size_to = 4 * max_params)]
    pub params: Vec<u32>,
}

impl EventLogMessage {
    pub fn package(&self) -> Option<ServerZoneIpcSegment> {
        match self.params.len() {
            0..=2 => Some(ServerZoneIpcSegment::new(
                ServerZoneIpcData::EventLogMessage2(self.clone()),
            )),
            3..=4 => Some(ServerZoneIpcSegment::new(
                ServerZoneIpcData::EventLogMessage4(self.clone()),
            )),
            _ => None,
        }
    }
}
