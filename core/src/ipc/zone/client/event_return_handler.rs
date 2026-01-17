use binrw::binrw;

use crate::common::HandlerId;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub struct EventReturnHandler<const MAX_PARAMS: usize> {
    pub handler_id: HandlerId,
    pub scene: u16,
    pub error_code: u8,
    pub num_results: u8,
    pub params: [i32; MAX_PARAMS],
}

impl<const MAX_PARAMS: usize> Default for EventReturnHandler<{ MAX_PARAMS }> {
    fn default() -> Self {
        Self {
            handler_id: HandlerId::default(),
            scene: 0,
            error_code: 0,
            num_results: 0,
            params: [0i32; MAX_PARAMS],
        }
    }
}
