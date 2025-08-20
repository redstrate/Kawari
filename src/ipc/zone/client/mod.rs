use binrw::binrw;
use paramacro::opcode_data;

mod action_request;
pub use crate::ipc::zone::client::action_request::ActionRequest;

mod send_chat_message;
pub use send_chat_message::SendChatMessage;

mod client_trigger;
pub use crate::ipc::zone::client::client_trigger::{ClientTrigger, ClientTriggerCommand};

mod event_yield_handler;
pub use crate::ipc::zone::client::event_yield_handler::EventYieldHandler;

mod item_operation;
pub use crate::ipc::zone::client::item_operation::ItemOperation;

pub use super::social_list::{PlayerEntry, SocialList, SocialListRequest, SocialListRequestType};

use super::config::Config;
use crate::common::{CHAR_NAME_MAX_LENGTH, Position, read_string, write_string};
use crate::opcodes::ClientZoneIpcType;
use crate::packet::IPC_HEADER_SIZE;

use crate::common::ObjectTypeId;
use crate::inventory::ContainerType;
use crate::packet::{IpcSegment, ReadWriteIpcOpcode, ReadWriteIpcSegment};

pub type ClientZoneIpcSegment = IpcSegment<ClientZoneIpcType, ClientZoneIpcData>;

impl ReadWriteIpcSegment for ClientZoneIpcSegment {
    fn calc_size(&self) -> u32 {
        IPC_HEADER_SIZE + self.op_code.calc_size()
    }

    fn get_name(&self) -> &'static str {
        self.op_code.get_name()
    }

    fn get_opcode(&self) -> u16 {
        self.op_code.get_opcode()
    }

    fn get_comment(&self) -> Option<&'static str> {
        self.op_code.get_comment()
    }
}

#[opcode_data(ClientZoneIpcType)]
#[binrw]
#[br(import(magic: &ClientZoneIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientZoneIpcData {
    InitRequest {
        #[brw(pad_before = 40)] // seems to be empty?
        #[brw(pad_size_to = 32)]
        #[br(count = 32)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        unk1: String,
        #[br(count = 48)]
        #[brw(pad_size_to = 48)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        unk2: String,
    },
    FinishLoading {
        // TODO: full of possibly interesting information
        unk: [u8; 72],
    },
    ClientTrigger(ClientTrigger),
    Unk3 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    Unk4 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    SetSearchInfoHandler {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    Unk5 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    SocialListRequest(SocialListRequest),
    UpdatePositionHandler {
        /// In radians.
        #[brw(pad_after = 4)] // empty
        rotation: f32,
        #[brw(pad_after = 4)] // empty
        position: Position,
    },
    LogOut {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    Disconnected {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    SendChatMessage(SendChatMessage),
    GMCommand {
        command: u32,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        #[brw(pad_after = 4)]
        target: u64,
    },
    ZoneJump {
        exit_box: u32,
        position: Position,
        #[brw(pad_after = 4)] // padding
        landset_index: i32,
    },
    ActionRequest(ActionRequest),
    Unk16 {
        unk: [u8; 8], // TODO: unknown
    },
    PingSync {
        timestamp: u32,
        /// Sapphire calls it this, but it never seems to have the player's actor id or any values resembling ids of any sort in it?
        origin_entity_id: u32,
        #[brw(pad_before = 4)]
        position: Position,
        #[brw(pad_after = 4)]
        rotation: f32,
    },
    Unk18 {
        unk: [u8; 8], // TODO: unknown
    },
    EventRelatedUnk {
        unk1: u32,
        unk2: u16,
        #[brw(pad_before = 2)]
        unk3: u32,
        unk4: u32,
    },
    Unk19 {
        unk: [u8; 16], // TODO: unknown
    },
    ItemOperation(ItemOperation),
    StartTalkEvent {
        actor_id: ObjectTypeId,
        #[brw(pad_after = 4)] // padding
        event_id: u32,
    },
    GilShopTransaction {
        event_id: u32,
        /// Seems to always be 0x300000a at gil shops
        unk1: u32,
        /// 1 is buy, 2 is sell
        buy_sell_mode: u32,
        /// Index into the shopkeeper's or the player's inventory
        item_index: u32,
        /// Quantity of items being bought or sold
        item_quantity: u32,
        /// unk 2: Flags? These change quite a bit when dealing with stackable items, but are apparently always 0 when buying non-stackable
        /// Observed values so far: 0xDDDDDDDD (when buying 99 of a stackable item), 0xFFFFFFFF, 0xFFE0FFD0, 0xfffefffe, 0x0000FF64
        unk2: u32,
    },
    StandardControlsPivot {
        /// Set to 4 when beginning to pivot.
        /// Set to 0 when pivoting ends.
        #[brw(pad_after = 4)]
        is_pivoting: u32,
    },
    EventYieldHandler(EventYieldHandler<2>),
    EventYieldHandler8(EventYieldHandler<8>),
    Config(Config),
    EventUnkRequest {
        event_id: u32,
        unk1: u16,
        unk2: u8,
        #[brw(pad_after = 8)]
        unk3: u8,
    },
    UnkCall2 {
        unk1: [u8; 8],
    },
    ContentFinderRegister {
        unk1: [u8; 8],
        flags: u32,
        unk2: [u8; 4],
        language_flags: u8, // TODO: turn this into a readable bitflag
        unk3: u8,
        classjob_id: u8,
        unk4: [u8; 7],
        #[brw(pad_after = 4)] // seems to empty
        content_ids: [u16; 5],
    },
    EquipGearset {
        /// Sapphire calls this a context id but it was observed as an actual index into the list of gearsets that the client keeps on its side.
        gearset_index: u32,
        /// In order: weapon, off-hand, head, body, hands, invalid/waist, legs, feet, earrings, neck, wrist, left ring, right ring, soul crystal
        /// When a container is irrelevant, it is marked as 9999/ContainerType::Invalid.
        containers: [ContainerType; 14],
        /// Indices into the containers.
        indices: [u16; 14],
        /// For the moment, it is completely unclear what unk1 and unk2 are used for or represent.
        #[brw(pad_before = 6)]
        unk1: u16,
        #[brw(pad_after = 2)]
        unk2: u16,
    },
    StartWalkInEvent {
        event_arg: u32,
        event_id: u32,
        #[brw(pad_after = 4)]
        pos: Position,
    },
    ContentFinderAction {
        unk1: [u8; 8],
    },
    NewDiscovery {
        layout_id: u32,
        pos: Position,
    },
    GMCommandName {
        command: u32,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        unk1: String,
    },
    RequestBlacklist {
        unk: [u8; 8],
    },
    RequestFellowships {
        unk: [u8; 8],
    },
    RequestCrossworldLinkshells {
        unk: [u8; 8],
    },
    SearchFellowships {
        #[br(count = 112)]
        #[bw(pad_size_to = 112)]
        unk: Vec<u8>,
    },
    StartCountdown {
        #[br(count = 40)]
        #[bw(pad_size_to = 40)]
        unk: Vec<u8>,
    },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

impl Default for ClientZoneIpcData {
    fn default() -> Self {
        Self::Unknown {
            unk: Vec::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_zone_ipc_sizes() {
        let ipc_types = [
            (
                ClientZoneIpcType::InitRequest,
                ClientZoneIpcData::InitRequest {
                    unk1: String::default(),
                    unk2: String::default(),
                },
            ),
            (
                ClientZoneIpcType::FinishLoading,
                ClientZoneIpcData::FinishLoading { unk: [0; 72] },
            ),
            (
                ClientZoneIpcType::ClientTrigger,
                ClientZoneIpcData::ClientTrigger(ClientTrigger::default()),
            ),
            (
                ClientZoneIpcType::Unk3,
                ClientZoneIpcData::Unk3 { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::Unk4,
                ClientZoneIpcData::Unk4 { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::SetSearchInfoHandler,
                ClientZoneIpcData::SetSearchInfoHandler { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::Unk5,
                ClientZoneIpcData::Unk5 { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::SocialListRequest,
                ClientZoneIpcData::SocialListRequest(SocialListRequest::default()),
            ),
            (
                ClientZoneIpcType::UpdatePositionHandler,
                ClientZoneIpcData::UpdatePositionHandler {
                    rotation: 0.0,
                    position: Position::default(),
                },
            ),
            (
                ClientZoneIpcType::LogOut,
                ClientZoneIpcData::LogOut { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::Disconnected,
                ClientZoneIpcData::Disconnected { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::SendChatMessage,
                ClientZoneIpcData::SendChatMessage(SendChatMessage::default()),
            ),
            (
                ClientZoneIpcType::GMCommand,
                ClientZoneIpcData::GMCommand {
                    command: 0,
                    arg0: 0,
                    arg1: 0,
                    arg2: 0,
                    arg3: 0,
                    target: 0,
                },
            ),
            (
                ClientZoneIpcType::ZoneJump,
                ClientZoneIpcData::ZoneJump {
                    exit_box: 0,
                    position: Position::default(),
                    landset_index: 0,
                },
            ),
            (
                ClientZoneIpcType::ActionRequest,
                ClientZoneIpcData::ActionRequest(ActionRequest::default()),
            ),
            (
                ClientZoneIpcType::Unk16,
                ClientZoneIpcData::Unk16 { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::PingSync,
                ClientZoneIpcData::PingSync {
                    timestamp: 0,
                    origin_entity_id: 0,
                    position: Position::default(),
                    rotation: 0.0,
                },
            ),
            (
                ClientZoneIpcType::Unk18,
                ClientZoneIpcData::Unk18 { unk: [0; 8] },
            ),
            (
                ClientZoneIpcType::EventRelatedUnk,
                ClientZoneIpcData::EventRelatedUnk {
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                },
            ),
            (
                ClientZoneIpcType::Unk19,
                ClientZoneIpcData::Unk19 { unk: [0; 16] },
            ),
            (
                ClientZoneIpcType::ItemOperation,
                ClientZoneIpcData::ItemOperation(ItemOperation::default()),
            ),
            (
                ClientZoneIpcType::StartTalkEvent,
                ClientZoneIpcData::StartTalkEvent {
                    actor_id: ObjectTypeId::default(),
                    event_id: 0,
                },
            ),
            (
                ClientZoneIpcType::GilShopTransaction,
                ClientZoneIpcData::GilShopTransaction {
                    event_id: 0,
                    unk1: 0,
                    buy_sell_mode: 0,
                    item_index: 0,
                    item_quantity: 0,
                    unk2: 0,
                },
            ),
            (
                ClientZoneIpcType::EventYieldHandler,
                ClientZoneIpcData::EventYieldHandler(EventYieldHandler::<2>::default()),
            ),
            (
                ClientZoneIpcType::EventYieldHandler8,
                ClientZoneIpcData::EventYieldHandler8(EventYieldHandler::<8>::default()),
            ),
            (
                ClientZoneIpcType::EventUnkRequest,
                ClientZoneIpcData::EventUnkRequest {
                    event_id: 0,
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                },
            ),
            (
                ClientZoneIpcType::UnkCall2,
                ClientZoneIpcData::UnkCall2 { unk1: [0; 8] },
            ),
            (
                ClientZoneIpcType::StartWalkInEvent,
                ClientZoneIpcData::StartWalkInEvent {
                    event_arg: 0,
                    event_id: 0,
                    pos: Position {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                },
            ),
            (
                ClientZoneIpcType::NewDiscovery,
                ClientZoneIpcData::NewDiscovery {
                    layout_id: 0,
                    pos: Position {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                },
            ),
        ];

        for (opcode, data) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ClientZoneIpcSegment {
                op_code: opcode.clone(), // doesn't matter for this test
                data: data.clone(),
                ..Default::default()
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{:#?} did not match size!",
                opcode
            );
        }
    }
}
