use binrw::binrw;

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct TrustContent {
    /// Index into the DawnContent Excel sheet.
    pub trust_content_id: u8,
    /// The last selected characters. If no character should be specified, it will be 0xFF.
    /// Index into the DawnGrowMember Excel sheet.
    pub last_selected_characters: [u8; 16],
}

impl TrustContent {
    pub const SIZE: usize = 17;
}

#[binrw]
#[derive(Debug, Clone)]
pub struct TrustInformation {
    #[br(count = Self::NUM_INDICES)]
    #[bw(pad_size_to = TrustContent::SIZE * Self::NUM_INDICES)]
    /// Which Trust content that you have available.
    /// There must be at least one valid TrustContent, otherwise the window will never show.
    pub available_content: Vec<TrustContent>,
    /// Levels for each Trust character.
    pub levels: [u8; Self::NUM_CHARACTERS],
    /// Current EXP for each Trust character.
    #[brw(pad_after = 32)]
    pub exp: [u32; Self::NUM_CHARACTERS],
}

impl TrustInformation {
    /// Number of rows of "story content". This is loosely based on the number of <200 rows in the DawnContent Excel sheet.
    pub const NUM_INDICES: usize = 33;

    /// Number of rows of "story content". This is loosely based on the number of <200 rows in the DawnContent Excel sheet.
    pub const NUM_CHARACTERS: usize = 27;
}

impl Default for TrustInformation {
    fn default() -> Self {
        Self {
            available_content: Default::default(),
            levels: [0; Self::NUM_CHARACTERS],
            exp: [0; Self::NUM_CHARACTERS],
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::common::ensure_size;

    use super::*;

    #[test]
    fn trust_content_size() {
        ensure_size::<TrustContent, { TrustContent::SIZE }>();
    }
}
