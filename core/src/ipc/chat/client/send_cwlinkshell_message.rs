use binrw::binrw;
use bstr::BString;

use crate::common::{MESSAGE_MAX_LENGTH, read_sestring, write_sestring};
use crate::ipc::chat::ChatChannel;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct SendCWLinkshellMessage {
    #[brw(pad_after = 1)] // Seems to be empty/zeroes
    pub chatchannel: ChatChannel,

    #[brw(pad_after = 7)] // Seems to be empty/zeroes
    #[br(count = MESSAGE_MAX_LENGTH)]
    #[bw(pad_size_to = MESSAGE_MAX_LENGTH)]
    #[br(map = read_sestring)]
    #[bw(map = write_sestring)]
    pub message: BString, // NOTE: This is a BString due to the fact that SEString macros can appear in its contents.
}
