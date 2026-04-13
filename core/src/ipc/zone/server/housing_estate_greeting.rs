use binrw::binrw;

use crate::common::{HouseId, read_string, write_string};

/// Contains the greeting text for a specific occupied plot
#[binrw]
#[derive(Debug, Default, Clone)]
pub struct HousingEstateGreeting {
    pub id: HouseId,

    #[bw(pad_size_to = 193)]
    #[br(count = 193)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub greeting: String,

    pub unk1: [u8; 7],
}
