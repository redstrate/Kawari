use binrw::binrw;

use crate::ipc::zone::client::ContainerType;

#[binrw]
#[derive(Clone, Copy, Default, Debug)]
pub struct MailItemInfo {
    /// Index into the Items Excel sheet.
    pub item_id: u32,
    /// The quantity of this item.
    pub item_quantity: u32,
    /// The container this item can be found in.
    pub src_container: ContainerType,
    /// Where it can be found in that container.
    pub src_container_index: u16,
}

#[binrw]
#[derive(Clone, Copy, Default, Debug)]
pub struct TakeAttachmentsInfo {
    pub item_id: u32,
    pub item_quantity: u32,
}
