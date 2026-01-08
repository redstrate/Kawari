use binrw::binrw;
use kawari_core_macro::opcode_data;

use super::OnlineStatusMask;
pub use super::social_list::{
    PlayerEntry, SocialList, SocialListRequest, SocialListRequestType, SocialListUIFlags,
    SocialListUILanguages,
};

mod chara_info;
use chara_info::CharaInfoFromContentIdsData;

mod player_spawn;
pub use player_spawn::PlayerSpawn;

mod status_effect;
pub use status_effect::StatusEffect;

mod update_class_info;
pub use update_class_info::UpdateClassInfo;

mod player_setup;
pub use player_setup::PlayerStatus;

mod player_stats;
pub use player_stats::PlayerStats;

mod actor_control;
pub use actor_control::{ActorControl, ActorControlCategory, ActorControlSelf, ActorControlTarget};

mod init_zone;
pub use init_zone::{InitZone, InitZoneFlags};

mod npc_spawn;
pub use npc_spawn::NpcSpawn;

mod common_spawn;
pub use common_spawn::{
    BattleNpcSubKind, CommonSpawn, DisplayFlag, GameMasterRank, ObjectKind, PlayerSubKind,
};

mod status_effect_list;
pub use status_effect_list::StatusEffectList;

mod weather_change;
pub use weather_change::WeatherChange;

mod container_info;
pub use container_info::ContainerInfo;

mod item_info;
pub use item_info::ItemInfo;

mod event_scene;
pub use event_scene::{EventScene, SceneFlags};

mod event_start;
pub use event_start::{EventStart, EventType};

mod action_result;
pub use action_result::{
    ActionEffect, ActionResult, DamageElement, DamageKind, DamageType, EffectKind,
};

mod warp;
pub use warp::Warp;

mod equip;
pub use equip::Equip;

mod currency_info;
pub use currency_info::CurrencyInfo;

pub use super::config::Config;

mod object_spawn;
pub use object_spawn::ObjectSpawn;

mod quest_active_list;
pub use quest_active_list::{ActiveQuest, QuestActiveList};

mod effect_result;
pub use effect_result::{EffectEntry, EffectResult};

mod condition;
pub use condition::{Condition, Conditions};

mod chat_message;
pub use chat_message::ChatMessage;

mod actor_move;
use crate::constants::{
    COMPLETED_LEVEQUEST_BITMASK_SIZE, COMPLETED_QUEST_BITMASK_SIZE, TITLE_UNLOCK_BITMASK_SIZE,
};
pub use crate::ipc::zone::server::actor_move::ActorMove;

mod server_notice;
pub use server_notice::{ServerNoticeFlags, ServerNoticeMessage};

mod quest_tracker;
pub use quest_tracker::{QuestTracker, TrackedQuest};

mod house_list;
pub use house_list::{House, HouseList};

mod housing_ward;
pub use housing_ward::HousingWardMenuSummaryItem;

mod trust_information;
pub use trust_information::{TrustContent, TrustInformation};

use crate::common::{
    CHAR_NAME_MAX_LENGTH, ContainerType, ItemOperationKind, ObjectId, read_string, write_string,
};
pub use crate::ipc::zone::black_list::{Blacklist, BlacklistedCharacter};
use crate::opcodes::ServerZoneIpcType;
use crate::packet::IpcSegment;
use crate::packet::ServerIpcSegmentHeader;

use crate::ipc::{
    chat::ChatChannel,
    zone::{
        PartyMemberEntry, PartyMemberPositions, PartyUpdateStatus, StrategyBoard,
        StrategyBoardUpdate, WaymarkPlacementMode, WaymarkPreset,
    },
};

use crate::ipc::zone::{InviteReply, InviteType, InviteUpdateType, SearchInfo};

pub type ServerZoneIpcSegment =
    IpcSegment<ServerIpcSegmentHeader<ServerZoneIpcType>, ServerZoneIpcType, ServerZoneIpcData>;

#[opcode_data(ServerZoneIpcType)]
#[binrw]
#[br(import(magic: &ServerZoneIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ServerZoneIpcData {
    InitResponse {
        #[brw(pad_before = 8, pad_after = 4)] // empty
        actor_id: ObjectId,
    },
    InitZone(InitZone),
    ActorControlSelf(ActorControlSelf),
    PlayerStats(PlayerStats),
    PlayerStatus(PlayerStatus),
    UpdateClassInfo(UpdateClassInfo),
    PlayerSpawn(PlayerSpawn),
    LogOutComplete {
        // TODO: guessed
        unk: [u8; 8],
    },
    Warp(Warp),
    ServerNoticeMessage(ServerNoticeMessage),
    LinkShellInformation {
        unk: [u8; 456],
    },
    PrepareZoning {
        log_message: u32,
        target_zone: u16,
        animation: u16,
        param4: u8,
        hide_character: u8,
        fade_out: u8,
        param_7: u8,
        fade_out_time: u8,
        unk1: u8,
        unk2: u16,
    },
    ActorControl(ActorControl),
    ActorMove(ActorMove),
    SocialList(SocialList),
    NpcSpawn(NpcSpawn),
    StatusEffectList(StatusEffectList),
    WeatherId(WeatherChange),
    UpdateItem(ItemInfo),
    ContainerInfo(ContainerInfo),
    EventScene {
        #[brw(args { max_params: 2 } )]
        data: EventScene,
    },
    EventScene4 {
        #[brw(args { max_params: 4 } )]
        data: EventScene,
    },
    EventScene8 {
        #[brw(args { max_params: 8 } )]
        data: EventScene,
    },
    EventScene16 {
        #[brw(args { max_params: 16 } )]
        data: EventScene,
    },
    EventScene32 {
        #[brw(args { max_params: 32 } )]
        data: EventScene,
    },
    EventScene64 {
        #[brw(args { max_params: 64 } )]
        data: EventScene,
    },
    EventScene128 {
        #[brw(args { max_params: 128 } )]
        data: EventScene,
    },
    EventScene255 {
        #[brw(args { max_params: 255 } )]
        data: EventScene,
    },
    EventStart(EventStart),
    UpdateHpMpTp {
        hp: u32,
        mp: u16,
        unk: u16, // it's filled with... something
    },
    ActionResult(ActionResult),
    Equip(Equip),
    DeleteActor {
        spawn_index: u8,
        #[brw(pad_before = 3)] // padding
        actor_id: ObjectId,
    },
    EventFinish {
        handler_id: u32,
        event_type: EventType,
        result: u8,
        #[brw(pad_before = 2)] // padding
        #[brw(pad_after = 4)] // padding
        arg: u32,
    },
    Condition(Conditions),
    ActorControlTarget(ActorControlTarget),
    CurrencyCrystalInfo(CurrencyInfo),
    Config(Config),
    EventUnkReply {
        event_id: u32,
        unk1: u16,
        unk2: u8,
        #[brw(pad_after = 8)]
        unk3: u8,
    },
    InventoryActionAck {
        sequence: u32,
        #[brw(pad_after = 10)]
        action_type: u16,
    },
    PingSyncReply {
        timestamp: u32,
        #[brw(pad_after = 24)]
        transmission_interval: u32,
    },
    QuestCompleteList {
        #[br(count = COMPLETED_QUEST_BITMASK_SIZE)]
        #[bw(pad_size_to = COMPLETED_QUEST_BITMASK_SIZE)]
        completed_quests: Vec<u8>,
        // TODO: what is in here?
        #[br(count = 65)]
        #[bw(pad_size_to = 65)]
        unk2: Vec<u8>,
    },
    UnkResponse2 {
        #[brw(pad_after = 7)]
        unk1: u8,
    },
    InventoryTransaction {
        /// This is later reused in InventoryTransactionFinish, so it might be some sort of sequence or context id, but it's not the one sent by the client
        sequence: u32,
        /// Same as the one sent by the client, not the one that the server responds with in InventoryActionAck!
        operation_type: ItemOperationKind,
        src_actor_id: ObjectId,
        #[brw(pad_size_to = 4)]
        src_storage_id: ContainerType,
        src_container_index: u16,
        #[brw(pad_before = 2)]
        src_stack: u32,
        src_catalog_id: u32,

        /// This section was observed to be static, across two captures and a bunch of discards these never changed.
        /// Always set to 0xE000_0000, also known as no/invalid actor.
        dst_actor_id: ObjectId,
        /// Used in discard operations, both this dummy container and dst_storage_id are set to a container type of 0xFFFF.
        /// While this struct is nearly identical to ItemOperation, it deviates here by not having 2 bytes of padding.
        dummy_container: ContainerType,
        dst_storage_id: ContainerType,
        dst_container_index: u16,
        /// Always set to zero.
        #[brw(pad_before = 2)]
        dst_stack: u32,
        /// Always set to zero.
        dst_catalog_id: u32,
    },
    InventoryTransactionFinish {
        /// Same sequence value as in InventoryTransaction.
        sequence: u32,
        /// Repeated unk1 value. No, it's not a copy-paste error.
        sequence_repeat: u32,
        /// Unknown, seems to always be 0x00000090.
        unk1: u32,
        /// Unknown, seems to always be 0x00000200.
        unk2: u32,
    },
    ContentFinderUpdate {
        /// 0 = Nothing happens
        /// 1 = Reserving server
        /// 2 = again? ^
        /// 3 = duty ready
        /// 4 = checking member status
        /// nothing appears to happen above 5
        state1: u8,
        classjob_id: u8,
        unk1: [u8; 18],
        content_ids: [u16; 5],
        unk2: [u8; 10],
    },
    ContentFinderFound {
        unk1: [u8; 28],
        content_id: u16,
        unk2: [u8; 10],
    },
    ObjectSpawn(ObjectSpawn),
    ActorGauge {
        classjob_id: u8,
        data: [u8; 15],
    },
    UpdateSearchInfo {
        online_status: OnlineStatusMask,
        unk1: u64,
        #[brw(pad_after = 1)] // padding
        unk2: u32,
        region: u8,
        #[brw(pad_after = 1)] // padding
        #[brw(pad_size_to = 193)]
        #[br(count = 193)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        message: String,
    },
    FreeCompanyInfo {
        unk: [u8; 80],
    },
    TitleList {
        #[br(count = TITLE_UNLOCK_BITMASK_SIZE)]
        #[bw(pad_size_to = TITLE_UNLOCK_BITMASK_SIZE)]
        unlock_bitmask: Vec<u8>,
    },
    QuestActiveList(QuestActiveList),
    LevequestCompleteList {
        #[br(count = COMPLETED_LEVEQUEST_BITMASK_SIZE)]
        #[bw(pad_size_to = COMPLETED_LEVEQUEST_BITMASK_SIZE)]
        completed_levequests: Vec<u8>,
        // TODO: what is in ehre?
        #[br(count = 6)]
        #[bw(pad_size_to = 6)]
        unk2: Vec<u8>,
    },
    ShopLogMessage {
        event_id: u32,
        /// When buying: 0x697
        /// When selling: 0x698
        /// When buying back: 0x699
        message_type: u32,
        /// Always 3, regardless of the interactions going on
        params_count: u32,
        item_id: u32,
        item_quantity: u32,
        #[brw(pad_after = 8)]
        total_sale_cost: u32,
    },
    ItemObtainedLogMessage {
        event_id: u32,
        /// Non-stackable item or a single item: 750 / 0x2EE ("You obtained a .")
        /// Stackable item: 751 / 0x2EF ("You obtained .")
        message_type: u32,
        /// Always 2
        params_count: u32,
        item_id: u32,
        #[brw(pad_after = 4)]
        /// Set to zero if only one item was obtained (stackable or not)
        item_quantity: u32,
    },
    UpdateInventorySlot {
        /// Starts from zero and increases by one for each of these packets during this gameplay session
        sequence: u32,
        #[brw(pad_before = 4)]
        dst_storage_id: ContainerType,
        dst_container_index: u16,
        dst_stack: u32,
        dst_catalog_id: u32,
        #[brw(pad_before = 12, pad_after = 28)]
        /// Always 0x7530_0000, this number appears elsewhere in buybacks so it's probably flags, but what they mean is completely unknown for now
        unk1: u32,
    },
    EffectResult(EffectResult),
    ContentFinderCommencing {
        unk1: [u8; 24],
    },
    StatusEffectList3 {
        status_effects: [StatusEffect; 30],
    },
    CrossworldLinkshells {
        // TODO: fill this out, each entry is 57 bytes probably
        unk1: [u8; 456],
    },
    SetSearchInfo(SearchInfo),
    Blacklist(Blacklist),
    WalkInEvent {
        /// Object ID of the ClientPath in the zone.
        path_id: u32,
        unk2: u16,
        #[brw(pad_before = 2)]
        unk3: u16,
        /// In some unknown amount of units.
        speed: u16,
        /// Always seems to be 1.
        constant: u16,
        unk4: u16,
        #[brw(pad_after = 4)]
        unk5: u32,
    },
    GrandCompanyInfo {
        active_company_id: u8,
        maelstrom_rank: u8,
        twin_adder_rank: u8,
        #[brw(pad_after = 4)]
        immortal_flames_rank: u8,
    },
    CraftingLog {
        unk1: [u8; 808],
    },
    GatheringLog {
        unk1: [u8; 104],
    },
    Fellowships {
        unk1: [u8; 808],
    },
    UnkZoneLoad1 {
        unk1: [u8; 56],
    },
    UnkZoneLoad2 {
        unk1: [u8; 8],
    },
    Linkshells {
        // TODO: fill this out, each entry appears to be 56 bytes long.
        unk: [u8; 448],
    },
    ChatMessage(ChatMessage),
    LocationDiscovered {
        map_part_id: u32,
        map_id: u32,
    },
    Mount {
        id: u16,
        unk1: [u8; 14],
    },
    SetOnlineStatus(OnlineStatusMask),
    FreeCompanyGreeting {
        unk: u8, // TODO: What is this? Seems to commonly be 0x01 or 0x02. Could this opcode be used as a general updater? Needs more research.
        #[brw(pad_size_to = 192)]
        #[br(count = 192)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 7)]
        message: String,
    },
    CharaInfoFromContentIds {
        #[brw(pad_before = 8)] // empty
        #[br(count = 10)]
        #[bw(pad_size_to = 10 * CharaInfoFromContentIdsData::SIZE)]
        info: Vec<CharaInfoFromContentIdsData>,
    },
    InviteCharacterResult {
        content_id: u64,
        #[brw(pad_before = 4)]
        world_id: u16,
        unk1: u8, // TODO: One of these unks is likely the InviteType
        unk2: u8,
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        character_name: String,
    },
    InviteReplyResult {
        content_id: u64,
        #[brw(pad_before = 4)]
        invite_type: InviteType,
        response: InviteReply,
        unk1: u8,
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 1)]
        character_name: String,
    },
    InviteUpdate {
        sender_account_id: u64,
        #[brw(pad_after = 8)] // empty
        sender_content_id: u64,
        expiration_timestamp: u32, // usually the packet's timestamp + 300
        world_id: u16,
        #[brw(pad_after = 1)] // Pretty sure this is empty
        invite_type: InviteType,
        update_type: InviteUpdateType,
        unk1: u8, // TODO: Usually 1? What is this?
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 6)] // empty
        sender_name: String,
    },
    PartyUpdate {
        execute_account_id: u64,
        target_account_id: u64,
        execute_content_id: u64,
        target_content_id: u64,
        unk1: u8, // TODO: Usually 1? What is this?
        unk2: u8, // TODO: Usually 1? What is this?
        update_status: PartyUpdateStatus,
        unk3: u8, // TODO: Usually 2? What is this?
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        execute_name: String,
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 3)] // empty
        target_name: String,
    },
    PartyList {
        #[br(count = PartyMemberEntry::NUM_ENTRIES)]
        #[bw(pad_size_to = PartyMemberEntry::NUM_ENTRIES * PartyMemberEntry::SIZE)]
        members: Vec<PartyMemberEntry>,
        party_id: u64,
        party_chatchannel: ChatChannel,
        leader_index: u8,
        #[brw(pad_after = 6)]
        member_count: u8,
    },
    PartyMemberPositions(PartyMemberPositions),
    AcceptQuest {
        /// Row ID - 65535
        #[brw(pad_after = 4)]
        quest_id: u32,
    },
    UpdateQuest {
        // TODO: index into what?
        #[brw(pad_after = 3)]
        index: u8,
        #[brw(pad_after = 8)] // seems empty
        quest: ActiveQuest,
    },
    FinishQuest {
        /// Row ID - 65535
        quest_id: u16,
        flag1: u8,
        #[brw(pad_after = 4)]
        flag2: u8,
    },
    UpdateMapMarkers2 {
        /// How many markers to update.
        marker_count: u32,
        /// Icons to set.
        icon_ids: [u32; 2],
        /// The instance ID in the level.
        layout_ids: [u32; 2],
        /// The event ID to update for, usually a quest ID.
        #[brw(pad_after = 4)] // padding
        handler_ids: [u32; 2],
    },
    QuestTracker(QuestTracker),
    HouseList(HouseList),
    HousingWardInfo {
        #[brw(pad_before = 2)]
        ward_index: u16,
        /// The territory/zone id shifted left by 16 bits and then ORed with the ward number. (could also just be split into two u16s I suppose ^^, was following sapphire at this point)
        land_set_id: u32,
        #[br(count = 60)]
        #[bw(pad_size_to = 60 * HousingWardMenuSummaryItem::SIZE)]
        house_summaries: Vec<HousingWardMenuSummaryItem>,
        #[brw(pad_after = 4)]
        terminator: u32,
    },
    SupportDeskNotification {
        unk1: [u8; 16],
    },
    ScenarioGuide {
        /// Not sure what this controls.
        quest_id_1: u32,
        /// Quest ID (Row ID - 65535) shown in big text. The next job quest is automatically determined.
        next_quest_id: u32,
        /// The game object ID to center on when opening the map.
        #[brw(pad_before = 4, pad_after = 16)] // seems empty
        layout_id: u32,
    },
    LegacyQuestList {
        bitmask: [u8; 40],
    },
    DirectorVars {
        director_id: u32,
        sequence: u8,
        branch: u8,
        data: [u8; 10],
        unk1: u16,
        unk2: u16,
        unk3: u16,
        unk4: u16,
    },
    UnkDirector1 {
        unk: [u8; 32],
    },
    UnkDirector2 {
        unk: [u8; 184],
    },
    FieldMarkerPreset(WaymarkPreset),
    DeleteObject {
        #[brw(pad_after = 7)] // padding
        spawn_index: u8,
    },
    GoldSaucerInformation {
        unk: [u8; 40],
    },
    UnkContentFinder {
        unk: [u8; 16],
    },
    TrustInformation(TrustInformation),
    DutySupportInformation {
        /// Indices into the DawnContent Excel sheet.
        #[br(count = 80)]
        #[bw(pad_size_to = 80)]
        available_content: Vec<u8>,
    },
    PortraitsInformation {
        unk: [u8; 56],
    },
    InitializeObfuscation {
        unk_before: [u8; 6],
        /// Zero means "no obsfucation" (not really, but functionally yes.)
        /// To enable obsfucation, you need to set this to a constant that changes every patch. See lib.rs for the constant.
        obsfucation_mode: u8,
        /// First seed used in deobsfucation on the client side.
        seed1: u8,
        /// Second seed used in deobsfucation on the client side.
        seed2: u8,
        #[brw(pad_before = 3)] // seems empty
        /// Third seed used in deobsfucation on the client side.
        seed3: u32,
    },
    StrategyBoardReceivedAck {
        /// The client ID of the player who received the board.
        content_id: u64,
        #[brw(pad_after = 4)] // Seems to be empty/always zeroes
        /// Unknown, possibly a result value. Observed as 1.
        unk: u32,
    },
    BeginStrategyBoardSession {
        /// All of these unknowns are possibly booleans or bitflags. See zone_connection/social.rs::received_strategy_board.
        unk1: u32,
        unk2: u32,
        #[brw(pad_after = 4)] // Seems to be empty/always zeroes
        unk3: u32,
    },
    StrategyBoard {
        /// The content id of the sending player.
        content_id: u64,
        /// The strategy board data.
        board_data: StrategyBoard,
    },
    StrategyBoardUpdate(StrategyBoardUpdate),
    EndStrategyBoardSession {
        unk: [u8; 16], // Always zeroes?
    },
    WaymarkUpdate {
        /// The id number of this waymark. 0 = A, 1 = B, and so on.
        id: u8,
        #[brw(pad_after = 2)] // Empty/always zeroes?
        /// The placement mode of this waymark. 1 = marker placed, 0 = marker removed.
        placement_mode: WaymarkPlacementMode,
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

impl Default for ServerZoneIpcData {
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

    use crate::{
        common::INVALID_OBJECT_ID,
        opcodes::ServerZoneIpcType,
        packet::{IpcSegmentHeader, ReadWriteIpcOpcode, ReadWriteIpcSegment},
    };

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn server_zone_ipc_sizes() {
        let ipc_types = [
            ServerZoneIpcData::InitResponse {
                actor_id: INVALID_OBJECT_ID,
            },
            ServerZoneIpcData::InitZone(InitZone::default()),
            ServerZoneIpcData::ActorControlSelf(ActorControlSelf::default()),
            ServerZoneIpcData::PlayerStats(PlayerStats::default()),
            ServerZoneIpcData::PlayerStatus(PlayerStatus::default()),
            ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo::default()),
            ServerZoneIpcData::PlayerSpawn(PlayerSpawn::default()),
            ServerZoneIpcData::LogOutComplete { unk: [0; 8] },
            ServerZoneIpcData::Warp(Warp::default()),
            ServerZoneIpcData::ServerNoticeMessage(ServerNoticeMessage::default()),
            ServerZoneIpcData::LinkShellInformation { unk: [0; 456] },
            ServerZoneIpcData::PrepareZoning {
                log_message: 0,
                target_zone: 0,
                animation: 0,
                param4: 0,
                hide_character: 0,
                fade_out: 0,
                param_7: 0,
                fade_out_time: 0,
                unk1: 0,
                unk2: 0,
            },
            ServerZoneIpcData::ActorControl(ActorControl::default()),
            ServerZoneIpcData::ActorMove(ActorMove::default()),
            ServerZoneIpcData::SocialList(SocialList::default()),
            ServerZoneIpcData::NpcSpawn(NpcSpawn::default()),
            ServerZoneIpcData::StatusEffectList(StatusEffectList::default()),
            ServerZoneIpcData::WeatherId(WeatherChange::default()),
            ServerZoneIpcData::UpdateItem(ItemInfo::default()),
            ServerZoneIpcData::ContainerInfo(ContainerInfo::default()),
            ServerZoneIpcData::EventScene {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene4 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene8 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene16 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene32 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene64 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene128 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventScene255 {
                data: EventScene::default(),
            },
            ServerZoneIpcData::EventStart(EventStart::default()),
            ServerZoneIpcData::UpdateHpMpTp {
                hp: 0,
                mp: 0,
                unk: 0,
            },
            ServerZoneIpcData::ActionResult(ActionResult::default()),
            ServerZoneIpcData::Equip(Equip::default()),
            ServerZoneIpcData::ActionResult(ActionResult::default()),
            ServerZoneIpcData::DeleteActor {
                spawn_index: 0,
                actor_id: INVALID_OBJECT_ID,
            },
            ServerZoneIpcData::EventFinish {
                handler_id: 0,
                event_type: EventType::Talk,
                result: 0,
                arg: 0,
            },
            ServerZoneIpcData::Condition(Conditions::default()),
            ServerZoneIpcData::ActorControlTarget(ActorControlTarget::default()),
            ServerZoneIpcData::CurrencyCrystalInfo(CurrencyInfo::default()),
            ServerZoneIpcData::EventUnkReply {
                event_id: 0,
                unk1: 0,
                unk2: 0,
                unk3: 0,
            },
            ServerZoneIpcData::InventoryActionAck {
                sequence: 0,
                action_type: 0,
            },
            ServerZoneIpcData::PingSyncReply {
                timestamp: 0,
                transmission_interval: 0,
            },
            ServerZoneIpcData::QuestCompleteList {
                completed_quests: Vec::default(),
                unk2: Vec::default(),
            },
            ServerZoneIpcData::UnkResponse2 { unk1: 0 },
            ServerZoneIpcData::InventoryTransaction {
                sequence: 0,
                operation_type: ItemOperationKind::Move,
                src_actor_id: INVALID_OBJECT_ID,
                src_storage_id: ContainerType::Inventory0,
                src_container_index: 0,
                src_stack: 0,
                src_catalog_id: 0,
                dst_actor_id: INVALID_OBJECT_ID,
                dummy_container: ContainerType::Inventory0,
                dst_storage_id: ContainerType::Inventory0,
                dst_container_index: 0,
                dst_stack: 0,
                dst_catalog_id: 0,
            },
            ServerZoneIpcData::InventoryTransactionFinish {
                sequence: 0,
                sequence_repeat: 0,
                unk1: 0,
                unk2: 0,
            },
            ServerZoneIpcData::ContentFinderUpdate {
                state1: 0,
                classjob_id: 0,
                unk1: [0; 18],
                content_ids: [0; 5],
                unk2: [0; 10],
            },
            ServerZoneIpcData::ContentFinderFound {
                unk1: [0; 28],
                content_id: 0,
                unk2: [0; 10],
            },
            ServerZoneIpcData::ObjectSpawn(ObjectSpawn::default()),
            ServerZoneIpcData::ActorGauge {
                classjob_id: 0,
                data: [0; 15],
            },
            ServerZoneIpcData::UpdateSearchInfo {
                online_status: OnlineStatusMask::default(),
                unk1: 0,
                unk2: 0,
                region: 0,
                message: String::default(),
            },
            ServerZoneIpcData::TitleList {
                unlock_bitmask: Vec::default(),
            },
            ServerZoneIpcData::FreeCompanyInfo { unk: [0; 80] },
            ServerZoneIpcData::QuestActiveList(QuestActiveList::default()),
            ServerZoneIpcData::LevequestCompleteList {
                completed_levequests: Vec::default(),
                unk2: Vec::default(),
            },
            ServerZoneIpcData::ShopLogMessage {
                event_id: 0,
                message_type: 0,
                params_count: 0,
                item_id: 0,
                item_quantity: 0,
                total_sale_cost: 0,
            },
            ServerZoneIpcData::ItemObtainedLogMessage {
                event_id: 0,
                message_type: 0,
                params_count: 0,
                item_id: 0,
                item_quantity: 0,
            },
            ServerZoneIpcData::UpdateInventorySlot {
                sequence: 0,
                dst_storage_id: ContainerType::Inventory0,
                dst_container_index: 0,
                dst_stack: 0,
                dst_catalog_id: 0,
                unk1: 0,
            },
            ServerZoneIpcData::EffectResult(EffectResult::default()),
            ServerZoneIpcData::ContentFinderCommencing { unk1: [0; 24] },
            ServerZoneIpcData::StatusEffectList3 {
                status_effects: [StatusEffect::default(); 30],
            },
            ServerZoneIpcData::CrossworldLinkshells { unk1: [0; 456] },
            ServerZoneIpcData::SetSearchInfo(SearchInfo::default()),
            ServerZoneIpcData::Blacklist(Blacklist {
                data: vec![BlacklistedCharacter::default(); Blacklist::NUM_ENTRIES],
                sequence: 0,
            }),
            ServerZoneIpcData::WalkInEvent {
                path_id: 0,
                constant: 1,
                unk2: 0,
                unk3: 0,
                speed: 0,
                unk4: 0,
                unk5: 0,
            },
            ServerZoneIpcData::GrandCompanyInfo {
                active_company_id: 0,
                maelstrom_rank: 0,
                twin_adder_rank: 0,
                immortal_flames_rank: 0,
            },
            ServerZoneIpcData::CraftingLog { unk1: [0; 808] },
            ServerZoneIpcData::GatheringLog { unk1: [0; 104] },
            ServerZoneIpcData::Fellowships { unk1: [0; 808] },
            ServerZoneIpcData::UnkZoneLoad1 { unk1: [0; 56] },
            ServerZoneIpcData::UnkZoneLoad2 { unk1: [0; 8] },
            ServerZoneIpcData::Linkshells { unk: [0; 448] },
            ServerZoneIpcData::ChatMessage(ChatMessage::default()),
            ServerZoneIpcData::LocationDiscovered {
                map_part_id: 0,
                map_id: 0,
            },
            ServerZoneIpcData::Mount {
                id: 0,
                unk1: [0; 14],
            },
            ServerZoneIpcData::SetOnlineStatus(OnlineStatusMask::default()),
            ServerZoneIpcData::FreeCompanyGreeting {
                unk: 0,
                message: "".to_string(),
            },
            ServerZoneIpcData::Linkshells { unk: [0; 448] },
            ServerZoneIpcData::CharaInfoFromContentIds {
                info: vec![CharaInfoFromContentIdsData::default(); 10],
            },
            ServerZoneIpcData::InviteCharacterResult {
                content_id: 0,
                world_id: 0,
                character_name: "".to_string(),
                unk1: 0,
                unk2: 0,
            },
            ServerZoneIpcData::InviteReplyResult {
                content_id: 0,
                invite_type: InviteType::Party,
                response: InviteReply::Declined,
                unk1: 0,
                character_name: "".to_string(),
            },
            ServerZoneIpcData::InviteUpdate {
                sender_content_id: 0,
                sender_account_id: 0,
                expiration_timestamp: 0,
                world_id: 0,
                invite_type: InviteType::Party,
                update_type: InviteUpdateType::InviteDeclined,
                sender_name: "".to_string(),
                unk1: 0,
            },
            ServerZoneIpcData::PartyUpdate {
                execute_account_id: 0,
                target_account_id: 0,
                execute_content_id: 0,
                target_content_id: 0,
                unk1: 0,
                unk2: 0,
                update_status: PartyUpdateStatus::None,
                unk3: 0,
                execute_name: "".to_string(),
                target_name: "".to_string(),
            },
            ServerZoneIpcData::PartyList {
                members: vec![PartyMemberEntry::default(); PartyMemberEntry::NUM_ENTRIES],
                party_id: 0,
                party_chatchannel: ChatChannel::default(),
                leader_index: 0,
                member_count: 0,
            },
            ServerZoneIpcData::PartyMemberPositions(PartyMemberPositions::default()),
            ServerZoneIpcData::AcceptQuest { quest_id: 0 },
            ServerZoneIpcData::UpdateQuest {
                index: 0,
                quest: ActiveQuest::default(),
            },
            ServerZoneIpcData::FinishQuest {
                quest_id: 0,
                flag1: 0,
                flag2: 0,
            },
            ServerZoneIpcData::UpdateMapMarkers2 {
                marker_count: 0,
                icon_ids: [0; 2],
                layout_ids: [0; 2],
                handler_ids: [0; 2],
            },
            ServerZoneIpcData::QuestTracker(QuestTracker::default()),
            ServerZoneIpcData::HouseList(HouseList::default()),
            ServerZoneIpcData::HousingWardInfo {
                ward_index: 0,
                land_set_id: 0,
                house_summaries: Vec::default(),
                terminator: 0,
            },
            ServerZoneIpcData::SupportDeskNotification { unk1: [0; 16] },
            ServerZoneIpcData::ScenarioGuide {
                quest_id_1: 0,
                next_quest_id: 0,
                layout_id: 0,
            },
            ServerZoneIpcData::LegacyQuestList { bitmask: [0; 40] },
            ServerZoneIpcData::DirectorVars {
                director_id: 0,
                sequence: 0,
                branch: 0,
                data: [0; 10],
                unk1: 0,
                unk2: 0,
                unk3: 0,
                unk4: 0,
            },
            ServerZoneIpcData::UnkDirector1 { unk: [0; 32] },
            ServerZoneIpcData::UnkDirector2 { unk: [0; 184] },
            ServerZoneIpcData::FieldMarkerPreset(WaymarkPreset::default()),
            ServerZoneIpcData::DeleteObject { spawn_index: 0 },
            ServerZoneIpcData::GoldSaucerInformation { unk: [0; 40] },
            ServerZoneIpcData::UnkContentFinder { unk: [0; 16] },
            ServerZoneIpcData::TrustInformation(TrustInformation::default()),
            ServerZoneIpcData::DutySupportInformation {
                available_content: Vec::default(),
            },
            ServerZoneIpcData::PortraitsInformation { unk: [0; 56] },
            ServerZoneIpcData::InitializeObfuscation {
                unk_before: [0; 6],
                obsfucation_mode: 0,
                seed1: 0,
                seed2: 0,
                seed3: 0,
            },
            ServerZoneIpcData::StrategyBoardReceivedAck {
                content_id: 0,
                unk: 0,
            },
            ServerZoneIpcData::BeginStrategyBoardSession {
                unk1: 0,
                unk2: 0,
                unk3: 0,
            },
            ServerZoneIpcData::StrategyBoard {
                content_id: 0,
                board_data: StrategyBoard::default(),
            },
            ServerZoneIpcData::StrategyBoardUpdate(StrategyBoardUpdate::default()),
            ServerZoneIpcData::EndStrategyBoardSession { unk: [0; 16] },
            ServerZoneIpcData::WaymarkUpdate {
                id: 0,
                placement_mode: WaymarkPlacementMode::Removed,
                unk1: 0,
                unk2: 0,
                unk3: 0,
            },
        ];

        for data in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let opcode: ServerZoneIpcType = ReadWriteIpcOpcode::from_data(data);
            let ipc_segment = ServerZoneIpcSegment {
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
