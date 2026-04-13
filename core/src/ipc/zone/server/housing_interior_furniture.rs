use crate::common::{LandId, Position};
use binrw::binrw;

#[binrw]
#[derive(Clone, Debug, Default)]
pub struct FurnitureList {
    /// The LandId this list is for.
    pub land_id: LandId,
    pub unk1: u8,
    /// The current `index` out of `count` packets to be sent.
    pub index: u8,
    /// The number of these lists that will be sent.
    pub count: u8,
    /// Seems to be some sort of outdoor vs indoor flag. If it's 0, it means this is for outdoor furniture, and 100 means it's for an interior.
    pub unk2: u8,
    /// The actual furnishings.
    #[br(count = Furniture::COUNT)]
    #[brw(pad_size_to = Furniture::COUNT * Furniture::SIZE)]
    #[brw(pad_after = 4)] // Seems to be empty/zeroes
    pub furniture: Vec<Furniture>,
}

#[binrw]
#[derive(Copy, Clone, Debug, Default)]
pub struct HousingInteriorDetails {
    /// This interior's window style.
    pub window_style: u16,
    pub unk1: u16, // Sapphire calls this "window color", but windows cannot be dyed?
    /// This interior's door style.
    pub door_style: u16,
    /// This interior door's dye colour. Index into the Stain Excel sheet.
    pub door_stain: u16,
    /// The light level in the interior. Note that this is actually described in terms of a level of *darkness*. When the client sets the UI light level to 5, the client will send value 0 in the ClientTrigger. Other examples: light level 1 will send 4, light level 2 will send 3, and so on.
    pub light_level: u8,
    pub unk2: [u8; 3], // likely just padding
    /// The ground floor's wall style. In an apartment, this along with ground_floor and ground_chandelier dictate what will decorate the apartment, leaving doors, windows, top floor and cellar all zeroes/blank.
    // TODO: It's unclear if these are pairs of u16s or just u32s.
    // NOTE: Be careful when experimenting with these values, as invalid combinations of u16s can crash the client, particularly if the interior is an apartment and top floor or cellar values are changed!
    pub ground_walls: u32,
    /// The ground floor's style/texture.
    pub ground_floor: u32,
    /// THe ground floor's chandelier. Unknown if this is a model id + toggle, or an item id + toggle.
    pub ground_chandelier: u32,
    /// The top floor's wall style/texture.
    pub top_walls: u32,
    /// The top floor's style/texture.
    pub top_floor: u32,
    /// The top floor's chandelier.
    pub top_chandelier: u32,
    /// The cellar's wall style/texture.
    pub cellar_walls: u32,
    /// The cellar's floor syle/texture.
    pub cellar_floor: u32,
    /// The cellar's chandelier.
    pub cellar_chandelier: u32,
    pub unk_interior: u32, // Unclear what this is, it can have data in mansions but not apartments or medium houses?
    unk3: u32,             // Might just be padding, seen as zeroes so far
}

#[binrw]
#[derive(Copy, Clone, Debug, Default)]
pub struct Furniture {
    /// Index into the FurnitureCatalogItemList sheet. If 0, no item is present in this entry. Therefore, this index needs to subtract 1 when indexing into the sheet!
    pub catalog_id: u16,
    pub unk1: u16, // Seems to always be 1 when this item is present.
    /// Index into the Stain sheet. Sets the dye for this item.
    pub stain: u16,
    pub unk2: [u8; 2], // Likely padding, but unsure.
    /// This item's rotation.
    pub rotation: f32,
    /// This item's 3d coordinates in the housing interior.
    pub position: Position,
}

impl Furniture {
    pub const SIZE: usize = 24;
    pub const COUNT: usize = 100;
}
