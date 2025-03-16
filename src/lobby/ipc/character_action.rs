use binrw::binrw;

#[binrw]
#[derive(Clone, PartialEq, Debug)]
pub enum LobbyCharacterAction {
    #[brw(magic = 0x1u8)]
    ReserveName,
    #[brw(magic = 0x2u8)]
    Create,
    #[brw(magic = 0x3u8)]
    Rename,
    #[brw(magic = 0x4u8)]
    Delete,
    #[brw(magic = 0x5u8)]
    Move,
    #[brw(magic = 0x6u8)]
    RemakeRetainer,
    #[brw(magic = 0x7u8)]
    RemakeChara,
    #[brw(magic = 0x8u8)]
    SettingsUploadBegin,
    #[brw(magic = 0xCu8)]
    SettingsUpload,
    #[brw(magic = 0xEu8)]
    WorldVisit,
    #[brw(magic = 0xFu8)]
    DataCenterToken,
    #[brw(magic = 0x15u8)]
    Request,
}
