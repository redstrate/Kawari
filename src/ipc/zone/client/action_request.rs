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

    use super::*;

    #[test]
    fn read_actionrequest() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/action_request.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let action_request = ActionRequest::read_le(&mut buffer).unwrap();
        assert_eq!(action_request.target.object_id, ObjectId(0x400097d0));
        assert_eq!(action_request.request_id, 0x2);
        assert_eq!(action_request.rotation, 1.9694216);
    }
}
