use binrw::binrw;

use crate::common::ObjectId;

#[binrw]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PlayerEnmity {
    pub actor_id: ObjectId,
    /// Value out of a 100, I think.
    pub enmity: u32,
}

impl PlayerEnmity {
    pub const SIZE: usize = 0x8;
}

#[binrw]
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnmityList {
    pub count: u32,
    #[brw(pad_after = 4)] // empty
    #[br(count = count)]
    #[bw(pad_size_to = PlayerEnmity::SIZE * 8)]
    pub list: Vec<PlayerEnmity>,
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
        d.push(server_zone_tests_dir!("enmity_list.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let enmity_list = EnmityList::read_le(&mut buffer).unwrap();
        assert_eq!(
            enmity_list,
            EnmityList {
                count: 1,
                list: vec![PlayerEnmity {
                    actor_id: ObjectId(277869081),
                    enmity: 100
                }]
            }
        );
    }
}
