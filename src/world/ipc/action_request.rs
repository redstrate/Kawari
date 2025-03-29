use binrw::binrw;

use crate::common::ObjectTypeId;

#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, Default)]
#[brw(repr = u8)]
pub enum ActionKind {
    #[default]
    Nothing = 0x0,
    Normal = 0x1,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActionRequest {
    pub exec_proc: u8, // what?
    pub action_kind: ActionKind,
    #[brw(pad_before = 2)] // this ISNT empty
    pub action_id: u32, // See Action Excel sheet
    pub request_id: u32,
    pub dir: u16,
    pub dir_target: u16,
    pub target: ObjectTypeId,
    pub arg: u32,
    pub padding_prob: u32,
}
