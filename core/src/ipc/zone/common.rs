use binrw::binrw;

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StrategyBoard {
    /// Apparently compressed data. Completely unknown what's in here, but it seems the server doesn't care about it, unless we want to document it someday!
    #[br(count = 1176)]
    #[bw(pad_size_to = 1176)]
    data: Vec<u8>,
}

#[binrw]
#[brw(little)]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StrategyBoardUpdate {
    /// Unknown data for now. The server seems not to care about it, unless we want to document it someday!
    #[br(count = 64)]
    #[bw(pad_size_to = 64)]
    data: Vec<u8>,
}
