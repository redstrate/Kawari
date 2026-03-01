use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Hater {
    pub actor_id: ObjectId,
    /// Value out of a 100, I think.
    pub enmity: u32,
}

impl Hater {
    pub const SIZE: usize = 0x8;
}

#[binrw]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HaterList {
    pub count: u32,
    #[brw(pad_after = 4)] // empty
    #[br(count = count)]
    #[bw(pad_size_to = Hater::SIZE * 32)]
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
