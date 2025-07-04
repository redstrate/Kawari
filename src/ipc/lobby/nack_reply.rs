#![allow(unused_variables)] // whoop binrw

use binrw::binrw;

use crate::common::{read_string, write_string};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct NackReply {
    pub sequence: u64,
    /// The error code shown in the client.
    pub error: u32,
    /// Value is specific to each error, e.g. your position in queue (i think)
    pub value: u32,
    /// The ID of a row in the Error Excel sheet.
    pub exd_error_id: u16,
    #[br(temp)]
    #[bw(calc = message.len() as u16)]
    pub message_size: u16,
    /// Seems to be unused
    #[brw(pad_after = 4)] // garbage
    #[bw(pad_size_to = 512)]
    #[br(count = 512)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub message: String,
}
