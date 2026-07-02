use binrw::binrw;

use crate::common::{Position, read_quantized_rotation, write_quantized_rotation};

/// See <https://github.com/aers/FFXIVClientStructs/blob/main/FFXIVClientStructs/FFXIV/Client/Game/UI/WarpInfo.cs>.
#[binrw]
#[brw(repr = u8)]
#[derive(Debug, Clone, Copy, Default)]
pub enum WarpType {
    /// Instantaneously change to this position.
    #[default]
    None = 0,
    /// `WARP_TYPE_NORMAL` from Lua.
    Normal = 1,
    /// Unknown, needs a better name but fades out the screen.
    Unk2 = 2,
    Translate = 3,
    Teleport = 4,
    Unk5 = 5,
    Unk6 = 6,
    Return = 7,
    Resurrection = 8,
    /// `WARP_TYPE_RENTAL_CHOCOBO` from Lua.
    RentalChocobo = 9,
    /// `WARP_TYPE_CHOCOBO_TAXI` from Lua.
    ChocoboTaxi = 10,
    Unk11 = 11,
    EnterInstanceContent = 12,
    LeaveInstanceContent = 13,
    Unk14 = 14,
    /// `WARP_TYPE_TOWN_TRANSLATE` from Lua.
    TownTranslate = 15,
    Unk16 = 16,
    Login = 17,
    Unk18 = 18,
    Unk19 = 19,
    HousingTeleport = 20,
    Unk21 = 21,
    Unk22 = 22,
    Unk23 = 23,
    Unk24 = 24,
    /// Seen during Mt Gulg, assuming it applies to all instanced content.
    Event = 25,
    Dive = 26,
    WorldTransfer = 27,
    Unk28 = 28,
    Unk29 = 29,
    Unk30 = 30,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorSetPos {
    /// The direction to which the player should be facing.
    #[br(map = read_quantized_rotation)]
    #[bw(map = write_quantized_rotation)]
    pub rotation: f32,
    /// What kind of warp this is.
    pub warp_type: WarpType,
    /// Argument based on `warp_type`.
    pub warp_type_arg: u8,
    /// Unknown purpose.
    pub transition_territory_filter_key: u32,
    /// The position to warp the player to.
    #[brw(pad_after = 4)] // padding
    pub position: Position,
}
