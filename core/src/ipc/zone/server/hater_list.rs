use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hater {
    /// ID of the hater.
    pub actor_id: ObjectId,
    /// Enmity percentage, capped to 0..100.
    #[brw(pad_after = 3)]
    pub enmity: u8,
}

impl Hater {
    pub const SIZE: usize = 0x8;
}

#[binrw]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HaterList {
    #[brw(pad_after = 3)]
    pub count: u8,
    #[br(count = count)]
    #[brw(pad_size_to = Hater::SIZE * 32)]
    #[brw(pad_after = 4)] // empty tail padding
    pub list: Vec<Hater>,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::{ObjectId, ensure_size};

    use crate::server_zone_tests_dir;

    use super::*;

    #[test]
    fn read_intro_event_start() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(server_zone_tests_dir!("hater_list.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let hater_list = HaterList::read_le(&mut buffer).unwrap();
        assert_eq!(
            hater_list,
            HaterList {
                count: 2,
                list: vec![
                    Hater {
                        actor_id: ObjectId(1073795094),
                        enmity: 100
                    },
                    Hater {
                        actor_id: ObjectId(1073795687),
                        enmity: 100
                    }
                ]
            }
        );
    }

    #[test]
    fn hater_size() {
        ensure_size::<Hater, { Hater::SIZE }>();
    }
}
