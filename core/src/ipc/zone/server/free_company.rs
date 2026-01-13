use crate::ipc::zone::server::{read_string, write_string};
use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Debug, Default, Clone)]
pub struct FcHierarchy {
    /// The amount of company members that hold this rank.
    count: u16,
    /// The order to display the rank in on the Rank tab.
    sort_number: u8,
    #[brw(pad_size_to = 45)]
    #[br(count = 45)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    #[brw(pad_after = 7)] // zeroes/empty
    /// The name of the rank.
    rank_name: String,
    /// A bitmask containing the rank's permissions.
    auth_list: u64,
    #[brw(pad_after = 23)] // zeroes/empty
    /// Unknown purpose.
    unk: u16,
}
