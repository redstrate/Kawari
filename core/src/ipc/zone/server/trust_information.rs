use binrw::binrw;

// TODO: Document Excel sheets better
// TODO: Move some values into constants

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct TrustContent {
    /// Index into the DawnContent Excel sheet.
    pub trust_content_id: u8,
    /// The last selected characters. If no character should be specified, it will be 0xFF.
    pub last_selected_characters: [u8; 16],
}

impl TrustContent {
    pub const SIZE: usize = 17;
}

#[binrw]
#[derive(Debug, Clone)]
pub struct TrustInformation {
    #[br(count = 32)]
    #[bw(pad_size_to = TrustContent::SIZE * 32)]
    /// Which Trust content that you have available.
    /// There must be at least one valid TrustContent, otherwise the window will never show.
    pub available_content: Vec<TrustContent>,
    #[brw(pad_before = 14)] // empty
    /// Levels for each Trust character.
    pub levels: [u8; 34],
    /// Current EXP for each Trust character.
    pub exp: [u32; 34],
}

impl Default for TrustInformation {
    fn default() -> Self {
        Self {
            available_content: Default::default(),
            levels: [0; 34],
            exp: [0; 34],
        }
    }
}
