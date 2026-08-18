use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct DutySupportInformation {
    /// List of indices into the DawnContent Excel sheet.
    #[br(count = Self::INDICE_COUNT)]
    #[bw(pad_size_to = Self::INDICE_COUNT)]
    pub available_content: Vec<u8>,
}

impl DutySupportInformation {
    /// Number of (useful rows in the DawnContent sheet.
    pub const INDICE_COUNT: usize = 80;
}
