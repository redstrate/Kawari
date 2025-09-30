use binrw::binrw;

use crate::common::{ObjectTypeId, read_quantized_rotation, write_quantized_rotation};

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Default)]
#[brw(repr = u8)]
pub enum ActionKind {
    #[default]
    Nothing = 0x0,
    Normal = 0x1,
    Item = 0x2,
    Mount = 0xD,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActionRequest {
    pub exec_proc: u8, // what?
    pub action_kind: ActionKind,
    #[brw(pad_before = 2)] // padding, i think it's filled with GARBAGE
    pub action_key: u32, // See Action Excel sheet
    pub request_id: u16,
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    pub dir: u16,
    pub dir_target: u16,
    pub target: ObjectTypeId,
    pub arg: u32,
    pub padding_prob: u32,
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use crate::common::ObjectId;

    use crate::client_zone_tests_dir;

    use super::*;

    #[test]
    fn read_actionrequest() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(client_zone_tests_dir!("action_request.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_request = ActionRequest::read_le(&mut buffer).unwrap();
        assert_eq!(action_request.action_kind, ActionKind::Normal);
        assert_eq!(action_request.target.object_id, ObjectId(0x400097d0));
        assert_eq!(action_request.request_id, 0x2);
        assert_eq!(action_request.rotation, 1.9694216);
    }

    #[test]
    fn read_actionrequest_mount() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(client_zone_tests_dir!("action_request_mount.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_request = ActionRequest::read_le(&mut buffer).unwrap();
        assert_eq!(action_request.action_kind, ActionKind::Mount);
        assert_eq!(action_request.action_key, 55);
        assert_eq!(action_request.target.object_id, ObjectId(277114100));
        assert_eq!(action_request.request_id, 4);
        assert_eq!(action_request.rotation, -0.8154669);
    }
}
