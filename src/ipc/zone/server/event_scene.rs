use binrw::binrw;

use crate::common::ObjectTypeId;
use crate::ipc::zone::server::{ServerZoneIpcData, ServerZoneIpcType};

#[derive(Debug, Clone, Default)]
#[binrw]
#[brw(little)]
#[brw(import{max_params: usize})]
#[brw(assert(params.len() <= max_params, "Too many params! {} > {}", params.len(), max_params))]
pub struct EventScene {
    pub actor_id: ObjectTypeId,
    pub event_id: u32,
    pub scene: u16,
    #[brw(pad_before = 2)] // FIXME: um, i don't think this is empty!!
    pub scene_flags: u32,
    pub unk1: u32,
    pub params_count: u8,
    // Extra padding seems needed after or the client will seemingly softlock even with 2 params, possibly used for alignment?
    #[brw(pad_before = 3, pad_after = 4)]
    #[br(count = max_params)]
    #[bw(pad_size_to = 4 * max_params)]
    pub params: Vec<u32>,
}

impl EventScene {
    pub fn package_scene(&self) -> Option<(ServerZoneIpcType, ServerZoneIpcData)> {
        let op_code;
        let data;
        match self.params.len() {
            // TODO: it would be nice to avoid cloning if possible
            0..=2 => {
                op_code = ServerZoneIpcType::EventScene;
                data = ServerZoneIpcData::EventScene { data: self.clone() };
            }
            3..=4 => {
                op_code = ServerZoneIpcType::EventScene4;
                data = ServerZoneIpcData::EventScene4 { data: self.clone() };
            }
            5..=8 => {
                op_code = ServerZoneIpcType::EventScene8;
                data = ServerZoneIpcData::EventScene8 { data: self.clone() };
            }
            9..=16 => {
                op_code = ServerZoneIpcType::EventScene16;
                data = ServerZoneIpcData::EventScene16 { data: self.clone() };
            }
            17..=32 => {
                op_code = ServerZoneIpcType::EventScene32;
                data = ServerZoneIpcData::EventScene32 { data: self.clone() };
            }
            33..=64 => {
                op_code = ServerZoneIpcType::EventScene64;
                data = ServerZoneIpcData::EventScene64 { data: self.clone() };
            }
            65..=128 => {
                op_code = ServerZoneIpcType::EventScene128;
                data = ServerZoneIpcData::EventScene128 { data: self.clone() };
            }
            129..255 => {
                op_code = ServerZoneIpcType::EventScene255;
                data = ServerZoneIpcData::EventScene255 { data: self.clone() };
            }
            _ => {
                return None;
            }
        }

        Some((op_code, data))
    }
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

        let event_play =
            EventScene::read_le_args(&mut buffer, EventSceneBinReadArgs { max_params: 2 }).unwrap();
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
        assert_eq!(event_play.params_count, 1);
        assert_eq!(event_play.params[0], 0);
        assert_eq!(event_play.params[1], 0);
    }
}
