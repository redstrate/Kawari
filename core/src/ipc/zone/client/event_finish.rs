use binrw::binrw;

use crate::common::HandlerId;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone)]
pub struct EventFinish<const MAX_PARAMS: usize> {
    pub handler_id: HandlerId,
    pub scene: u16,
    pub error_code: u8,
    pub num_results: u8,
    pub params: [i32; MAX_PARAMS],
}

impl<const MAX_PARAMS: usize> Default for EventFinish<{ MAX_PARAMS }> {
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

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_event_finish1_payload() {
        let buffer = [
            0x08, 0x00, 0x02, 0x00, // handler id
            0x00, 0x00, // scene
            0x00, // error code
            0x00, // num results
            0x00, 0x00, 0x00, 0x00, // param 0
            0x00, 0x00, 0x00, 0x00, // param 1
        ];
        let mut buffer = Cursor::new(buffer);

        let event_finish = EventFinish::<2>::read_le(&mut buffer).unwrap();
        assert_eq!(event_finish.handler_id.0, 0x00020008);
        assert_eq!(event_finish.scene, 0);
        assert_eq!(event_finish.error_code, 0);
        assert_eq!(event_finish.num_results, 0);
        assert_eq!(event_finish.params, [0, 0]);
        assert_eq!(buffer.position(), 16);
    }
}
