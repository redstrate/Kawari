use binrw::binrw;

use crate::common::ObjectTypeId;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct EventStart {
    pub target_id: ObjectTypeId,
    pub event_id: u32,
    pub event_type: u8,
    pub flags: u8,
    #[brw(pad_before = 2)]
    #[brw(pad_after = 4)]
    pub event_arg: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::ObjectId;

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_intro_event_start() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("event_start.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let event_start = EventStart::read_le(&mut buffer).unwrap();
        assert_eq!(
            event_start.target_id,
            ObjectTypeId {
                object_id: ObjectId(277124129),
                object_type: 0
            }
        );
        assert_eq!(event_start.event_id, 0x130003); // aether intro
        assert_eq!(event_start.event_type, 15);
        assert_eq!(event_start.flags, 0);
        assert_eq!(event_start.event_arg, 182);
    }
}
