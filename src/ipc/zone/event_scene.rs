use binrw::binrw;

use crate::common::ObjectTypeId;

#[binrw]
#[brw(little)]
#[derive(Debug, Clone, Default)]
pub struct EventScene {
    pub actor_id: ObjectTypeId,
    pub event_id: u32,
    pub scene: u16,
    #[brw(pad_before = 2)] // FIXME: um, i don't think this is empty!!
    pub scene_flags: u32,
    pub unk1: u32,
    pub unk2: u8,
    #[brw(pad_before = 3)]
    pub unk3: u32,
    pub unk4: u32,
    pub unk5: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::ObjectId;

    use super::*;

    #[test]
    fn read_intro_event_start() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/event_play.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let event_play = EventScene::read_le(&mut buffer).unwrap();
        assert_eq!(
            event_play.actor_id,
            ObjectTypeId {
                object_id: ObjectId(277124129),
                object_type: 0
            }
        );
        assert_eq!(event_play.event_id, 0x130003); // aether intro
        assert_eq!(event_play.scene, 0);
        assert_eq!(event_play.scene_flags, 4959237);
        assert_eq!(event_play.unk1, 0);
        assert_eq!(event_play.unk2, 1);
        assert_eq!(event_play.unk3, 0);
        assert_eq!(event_play.unk4, 0);
        assert_eq!(event_play.unk5, 0);
    }
}
