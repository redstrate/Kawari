use crate::{
    common::{read_string, write_string},
    ipc::zone::{OnlineStatusMask, SocialListUILanguages},
};
use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Clone, Default, Debug)]
pub struct SearchInfo {
    pub online_status: OnlineStatusMask,
    pub unk1: [u8; 9],
    pub selected_languages: SocialListUILanguages,
    #[brw(pad_size_to = 60)]
    #[br(count = 60)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub comment: String,
    #[br(count = 138)]
    #[bw(pad_size_to = 138)]
    pub unk: Vec<u8>,
}
