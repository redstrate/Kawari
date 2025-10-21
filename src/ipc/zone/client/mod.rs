use binrw::binrw;
use paramacro::opcode_data;

mod action_request;
pub use crate::ipc::zone::client::action_request::{ActionKind, ActionRequest};

mod send_chat_message;
pub use send_chat_message::SendChatMessage;

mod client_trigger;
pub use crate::ipc::zone::client::client_trigger::{ClientTrigger, ClientTriggerCommand};

mod event_yield_handler;
pub use crate::ipc::zone::client::event_yield_handler::EventYieldHandler;

mod item_operation;
pub use crate::ipc::zone::client::item_operation::ItemOperation;

mod event_return_handler;
pub use crate::ipc::zone::client::event_return_handler::EventReturnHandler;

use crate::ipc::zone::{InviteReply, InviteType, SearchInfo};

use crate::ipc::zone::black_list::RequestBlacklist;

pub use super::social_list::{PlayerEntry, SocialList, SocialListRequest, SocialListRequestType};

use super::config::Config;
use crate::common::{
    CHAR_NAME_MAX_LENGTH, ClientLanguage, JumpState, MoveAnimationState, MoveAnimationType,
    Position, read_string, write_string,
};
use crate::opcodes::ClientZoneIpcType;
use crate::packet::ServerIpcSegmentHeader;

use crate::common::ObjectTypeId;
use crate::inventory::ContainerType;
use crate::packet::IpcSegment;

pub type ClientZoneIpcSegment =
    IpcSegment<ServerIpcSegmentHeader<ClientZoneIpcType>, ClientZoneIpcType, ClientZoneIpcData>;

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
        rotation: f32,
        anim_type: MoveAnimationType,
        anim_state: MoveAnimationState,
        #[brw(pad_after = 1)] // empty
        jump_state: JumpState,
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
    EventReturnHandler4(EventReturnHandler<4>),
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
    RequestBlacklist(RequestBlacklist),
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
    RequestPlaytime {
        unk: [u8; 8],
    },
    SetFreeCompanyGreeting {
        #[brw(pad_size_to = 192)]
        #[br(count = 192)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 8)]
        message: String,
    },
    SetClientLanguage {
        #[brw(pad_before = 4)] // empty
        #[brw(pad_after = 3)] // empty
        language: ClientLanguage,
    },
    RequestCharaInfoFromContentIds {
        content_ids: [u64; 10],
    },
    PartyLeave {
        unk: [u8; 8], // seems to always be zeroes?
    },
    PartyDisband {
        unk: [u8; 8], // seems to always be zeroes?
    },
    PartyMemberKick {
        #[brw(pad_after = 4)]
        party_index: u32,
        unk: u16, // Always 0x003F?

        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 6)] // empty
        character_name: String,
    },
    PartyChangeLeader {
        #[brw(pad_after = 4)] // empty
        party_index: u32,
        unk: u16, // Always 0x003F?

        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 6)] // empty
        character_name: String,
    },
    InviteCharacter {
        content_id: u64,
        world_id: u16,
        invite_type: InviteType,
        // TODO: This opcode currently has an issue where garbage data is apparently left in this buffer if the sender's name is longer than the recipient's, and it's also unclear if the name field's length is actually 32 here. A retail capture is needed in this situation.
        #[brw(pad_size_to = 21)]
        #[br(count = 21)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 16)] // empty, but see above
        character_name: String,
    },
    InviteReply {
        sender_content_id: u64, // The inviter's content_id
        sender_world_id: u16,   // The current world id
        invite_type: InviteType,
        #[brw(pad_after = 4)] // empty
        response: InviteReply,
    },
    RequestSearchInfo {
        content_id: u64,
        unk: [u8; 16], // unsure if this is always empty
    },
    RequestAdventurerPlate {
        unk: [u8; 16],
    },
    SearchPlayers {
        #[br(count = 176)]
        #[bw(pad_size_to = 176)]
        unk: Vec<u8>,
    },
    EditSearchInfo(SearchInfo),
    RequestOwnSearchInfo {
        unk: [u8; 8],
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

    use crate::packet::{IpcSegmentHeader, ReadWriteIpcOpcode, ReadWriteIpcSegment};

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_zone_ipc_sizes() {
        let ipc_types = [
            ClientZoneIpcData::InitRequest {
                unk1: String::default(),
                unk2: String::default(),
            },
            ClientZoneIpcData::FinishLoading { unk: [0; 72] },
            ClientZoneIpcData::ClientTrigger(ClientTrigger::default()),
            ClientZoneIpcData::Unk3 { unk: [0; 8] },
            ClientZoneIpcData::Unk4 { unk: [0; 8] },
            ClientZoneIpcData::SetSearchInfoHandler { unk: [0; 8] },
            ClientZoneIpcData::Unk5 { unk: [0; 8] },
            ClientZoneIpcData::SocialListRequest(SocialListRequest::default()),
            ClientZoneIpcData::UpdatePositionHandler {
                rotation: 0.0,
                position: Position::default(),
                anim_type: MoveAnimationType::default(),
                anim_state: MoveAnimationState::default(),
                jump_state: JumpState::default(),
            },
            ClientZoneIpcData::LogOut { unk: [0; 8] },
            ClientZoneIpcData::Disconnected { unk: [0; 8] },
            ClientZoneIpcData::SendChatMessage(SendChatMessage::default()),
            ClientZoneIpcData::GMCommand {
                command: 0,
                arg0: 0,
                arg1: 0,
                arg2: 0,
                arg3: 0,
                target: 0,
            },
            ClientZoneIpcData::ZoneJump {
                exit_box: 0,
                position: Position::default(),
                landset_index: 0,
            },
            ClientZoneIpcData::ActionRequest(ActionRequest::default()),
            ClientZoneIpcData::Unk16 { unk: [0; 8] },
            ClientZoneIpcData::PingSync {
                timestamp: 0,
                origin_entity_id: 0,
                position: Position::default(),
                rotation: 0.0,
            },
            ClientZoneIpcData::Unk18 { unk: [0; 8] },
            ClientZoneIpcData::EventRelatedUnk {
                unk1: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,
            },
            ClientZoneIpcData::Unk19 { unk: [0; 16] },
            ClientZoneIpcData::ItemOperation(ItemOperation::default()),
            ClientZoneIpcData::StartTalkEvent {
                actor_id: ObjectTypeId::default(),
                event_id: 0,
            },
            ClientZoneIpcData::EventReturnHandler4(EventReturnHandler::default()),
            ClientZoneIpcData::StandardControlsPivot { is_pivoting: 0 },
            ClientZoneIpcData::EventYieldHandler(EventYieldHandler::<2>::default()),
            ClientZoneIpcData::EventYieldHandler8(EventYieldHandler::<8>::default()),
            ClientZoneIpcData::Config(Config::default()),
            ClientZoneIpcData::EventUnkRequest {
                event_id: 0,
                unk1: 0,
                unk2: 0,
                unk3: 0,
            },
            ClientZoneIpcData::UnkCall2 { unk1: [0; 8] },
            ClientZoneIpcData::ContentFinderRegister {
                unk1: [0; 8],
                flags: 0,
                unk2: [0; 4],
                language_flags: 0,
                unk3: 0,
                classjob_id: 0,
                unk4: [0; 7],
                content_ids: [0; 5],
            },
            ClientZoneIpcData::EquipGearset {
                gearset_index: 0,
                containers: [ContainerType::Inventory0; 14],
                indices: [0; 14],
                unk1: 0,
                unk2: 0,
            },
            ClientZoneIpcData::StartWalkInEvent {
                event_arg: 0,
                event_id: 0,
                pos: Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
            ClientZoneIpcData::ContentFinderAction { unk1: [0; 8] },
            ClientZoneIpcData::NewDiscovery {
                layout_id: 0,
                pos: Position {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
            },
            ClientZoneIpcData::GMCommandName {
                command: 0,
                arg0: 0,
                arg1: 0,
                arg2: 0,
                arg3: 0,
                unk1: String::default(),
            },
            ClientZoneIpcData::RequestBlacklist(RequestBlacklist::default()),
            ClientZoneIpcData::RequestFellowships { unk: [0; 8] },
            ClientZoneIpcData::RequestCrossworldLinkshells { unk: [0; 8] },
            ClientZoneIpcData::SearchFellowships {
                unk: Vec::default(),
            },
            ClientZoneIpcData::StartCountdown {
                unk: Vec::default(),
            },
            ClientZoneIpcData::RequestPlaytime { unk: [0; 8] },
            ClientZoneIpcData::SetFreeCompanyGreeting {
                message: "".to_string(),
            },
            ClientZoneIpcData::SetClientLanguage {
                language: ClientLanguage::Japanese,
            },
            ClientZoneIpcData::RequestCharaInfoFromContentIds {
                content_ids: [0; 10],
            },
            ClientZoneIpcData::PartyLeave { unk: [0; 8] },
            ClientZoneIpcData::PartyDisband { unk: [0; 8] },
            ClientZoneIpcData::PartyMemberKick {
                party_index: 0,
                unk: 0,
                character_name: "".to_string(),
            },
            ClientZoneIpcData::PartyChangeLeader {
                party_index: 0,
                unk: 0,
                character_name: "".to_string(),
            },
            ClientZoneIpcData::InviteCharacter {
                content_id: 0,
                world_id: 0,
                character_name: "".to_string(),
                invite_type: InviteType::Party,
            },
            ClientZoneIpcData::InviteReply {
                sender_content_id: 0,
                sender_world_id: 0,
                invite_type: InviteType::Party,
                response: InviteReply::Declined,
            },
            ClientZoneIpcData::RequestSearchInfo {
                content_id: 0,
                unk: [0; 16],
            },
            ClientZoneIpcData::RequestAdventurerPlate { unk: [0; 16] },
            ClientZoneIpcData::SearchPlayers { unk: Vec::new() },
            ClientZoneIpcData::EditSearchInfo(SearchInfo::default()),
            ClientZoneIpcData::RequestOwnSearchInfo { unk: [0; 8] },
        ];

        for data in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let opcode: ClientZoneIpcType = ReadWriteIpcOpcode::from_data(data);
            let ipc_segment = ClientZoneIpcSegment {
                header: IpcSegmentHeader::from_opcode(opcode.clone()),
                data: data.clone(),
                ..Default::default()
            };
            ipc_segment.write_le(&mut cursor).unwrap();

            let buffer = cursor.into_inner();

            assert_eq!(
                buffer.len(),
                ipc_segment.calc_size() as usize,
                "{opcode:#?} did not match size!"
            );
        }
    }
}
