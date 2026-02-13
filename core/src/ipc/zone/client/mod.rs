use binrw::binrw;
use kawari_core_macro::opcode_data;

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

mod queue_duties;
pub use queue_duties::{ContentRegistrationFlags, QueueDuties};

use crate::ipc::zone::{
    InviteReply, InviteType, SearchInfo, SocialListUILanguages, StrategyBoard, StrategyBoardUpdate,
    WaymarkPreset,
};

use crate::ipc::zone::black_list::RequestBlacklist;

pub use super::social_list::{PlayerEntry, SocialList, SocialListRequest, SocialListRequestType};

use super::config::Config;
use crate::common::{
    CHAR_NAME_MAX_LENGTH, ClientLanguage, HandlerId, JumpState, MoveAnimationState,
    MoveAnimationType, ObjectId, Position, read_string, write_string,
};
use crate::opcodes::ClientZoneIpcType;
use crate::packet::ServerIpcSegmentHeader;

use crate::common::{ContainerType, ObjectTypeId};
use crate::packet::IpcSegment;

#[binrw]
#[brw(repr = u8)]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ContentFinderUserAction {
    /// Accepted the duty.
    #[default]
    Accepted = 0,
    /// Withdrawn from the duty.
    Withdrawn = 1,
    /// Let the duty announcement time out.
    Timeout = 2,
}

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
    UnkSocialEvent {
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
        arg0: i32,
        arg1: i32,
        arg2: i32,
        arg3: i32,
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
    PingSync {
        timestamp: u32,
        /// Sapphire calls it this, but it never seems to have the player's actor id or any values resembling ids of any sort in it?
        origin_entity_id: u32,
        #[brw(pad_before = 4)]
        position: Position,
        #[brw(pad_after = 4)]
        rotation: f32,
    },
    EventRelatedUnk {
        unk1: u32,
        unk2: u16,
        #[brw(pad_before = 2)]
        unk3: u32,
        unk4: u32,
    },
    ItemOperation(ItemOperation),
    StartTalkEvent {
        actor_id: ObjectTypeId,
        #[brw(pad_after = 4)] // padding
        handler_id: HandlerId,
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
        handler_id: HandlerId,
        unk1: u16,
        unk2: u8,
        #[brw(pad_after = 8)]
        unk3: u8,
    },
    UnkCall2 {
        unk1: [u8; 8],
    },
    QueueDuties(QueueDuties),
    EquipGearset {
        /// Index into the list of gearsets that the client keeps on its side.
        gearset_index: u32,
        /// In order: weapon, off-hand, head, body, hands, invalid/waist, legs, feet, earrings, neck, wrist, left ring, right ring, soul crystal
        /// When a container is irrelevant, it is marked as 9999/ContainerType::Invalid.
        containers: [ContainerType; 14],
        /// Indices into the containers.
        indices: [i16; 14],
        /// For the moment, it is completely unclear what unk1 and unk2 are used for or represent.
        #[brw(pad_before = 6)]
        unk1: u16,
        #[brw(pad_after = 2)]
        unk2: u16,
    },
    EquipGearset2 {
        /// Index into the list of gearsets that the client keeps on its side.
        gearset_index: u32,
        /// In order: weapon, off-hand, head, body, hands, invalid/waist, legs, feet, earrings, neck, wrist, left ring, right ring, soul crystal
        /// When a container is irrelevant, it is marked as 9999/ContainerType::Invalid.
        containers: [ContainerType; 14],
        /// Indices into the containers.
        indices: [i16; 14],
        /// For the moment, it is completely unclear what unk1/unk2/unk3 are used for or represent.
        #[brw(pad_before = 6)]
        unk1: u16,
        #[brw(pad_after = 2)]
        unk2: u16,
        #[br(count = 56)]
        #[bw(pad_size_to = 56)]
        unk3: Vec<u8>,
    },
    StartWalkInEvent {
        event_arg: u32,
        handler_id: HandlerId,
        #[brw(pad_after = 4)]
        pos: Position,
    },
    ContentFinderAction {
        action: ContentFinderUserAction,
        unk1: [u8; 7],
    },
    NewDiscovery {
        layout_id: u32,
        pos: Position,
    },
    GMCommandName {
        command: u32,
        arg0: i32,
        arg1: i32,
        arg2: i32,
        arg3: i32,
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
        /// The actor id of the character who initiated the countdown.
        starter_actor_id: ObjectId,
        /// The duration of the countdown in seconds.
        #[brw(pad_after = 2)] // Empty/zeroes, doesn't appear to have anything?
        duration: u16,
        /// The name of the character who initiated the countdown.
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        starter_name: String,
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
        content_id: u64,
        unk: u16, // Always 0x003F?

        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 6)] // empty
        character_name: String,
    },
    PartyChangeLeader {
        /// The actor id of the new leader.
        content_id: u64,
        unk: u16, // Always 0x003F?

        /// The name of the new leader.
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
        // TODO: The client leaves garbage (probably due to a bug) in the character_name field, so reading it can be a little tricky. Our string parsing had to be updated a little bit to retry when Rusts's String::from_utf8 function fails. Parsing it as a C string (CStr in rust) can work around this issue.
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 5)] // "empty", but see above
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
    WalkOutsideEvent {
        event_arg: u32,
        handler_id: HandlerId,
        #[brw(pad_after = 4)]
        pos: Position,
    },
    EnterTerritoryEvent {
        #[brw(pad_after = 4)] // empty
        handler_id: HandlerId,
    },
    Trade {
        unk: [u8; 16],
    },
    BuyInclusionShop {
        /// ID of a row in the InclusionShop Excel sheet.
        shop_id: u32,
        /// Unknown purpose, I see 1?
        unk1: u32,
        /// The `shop_id` again.
        shop_id_again: u32,
        /// The category as seen in the InclusionShop Excel sheet.
        category: u32,
        /// ID of a row in the SpecialShop Excel sheet.
        special_shop_id: u32,
        /// Which item from the SpecialShop is selected.
        item_index: u32,
        /// Quantity? I see 1.
        unk2: u32,
        /// Unknown purpose, I see 9999?
        unk3: u32,
        // Dunno.
        unk4: [u8; 40],
    },
    ShareStrategyBoard {
        /// When the content id is 0, the client is starting a non-real-time session, or is initiating one but isn't ready yet.
        content_id: u64,
        board_data: StrategyBoard,
    },
    StrategyBoardReceived {
        content_id: u64,
        #[brw(pad_after = 4)] // Seems to be empty/always zeroes
        unk: u32,
    },
    StrategyBoardUpdate(StrategyBoardUpdate),
    RealtimeStrategyBoardFinished {
        /// Both unknowns have data, but it's unclear what they are. They don't appear to be party ids or content ids.
        unk1: u32,
        unk2: u32,
    },
    ApplyFieldMarkerPreset(WaymarkPreset),
    RequestFreeCompanyShortMessage {
        /// The content id of the requested character.
        content_id: u64,
        #[brw(pad_after = 4)]
        /// A sequence value that is repeated by the server later on in FreeCompanyShortMessage.
        sequence: u32,
    },
    QueueRoulette {
        /// See the ContentRoulette Excel sheet.
        roulette_id: u8,
        unk1: [u8; 15],
        /// The languages to match with.
        languages: SocialListUILanguages,
        unk2: [u8; 7],
    },
    PlayGoldSaucerMachine {
        handler_id: HandlerId,
        unk1: u32,
        unk2: u32, // empty?
        unk3: u32,
    },
    InitiateReadyCheck {
        unk: [u8; 8], // Seems to always be zeroes/unused
    },
    ReadyCheckResponse {
        /// The player's response to the ready check. 1 indicates yes, 0 indicates no.
        /// If the player fails to respond before the vote ends, their vote is automatically cast as no.
        #[brw(pad_before = 1, pad_after = 6)] // Seems to be empty/zeroes
        response: u8,
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
    use crate::common::test_opcodes;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn client_zone_ipc_sizes() {
        test_opcodes::<ClientZoneIpcSegment>();
    }
}
