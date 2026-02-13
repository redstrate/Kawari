use binrw::binrw;
use kawari_core_macro::opcode_data;

use super::OnlineStatusMask;
pub use super::social_list::{SocialList, SocialListUIFlags, SocialListUILanguages};

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

mod free_company;
pub use free_company::FcHierarchy;

mod actor_move;
use crate::common::{HandlerId, LandData};
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

use crate::ipc::zone::social_list::GrandCompany;
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
        handler_id: HandlerId,
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
        handler_id: HandlerId,
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
        handler_id: HandlerId,
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
        handler_id: HandlerId,
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
    DailyQuests {
        unk1: [u8; 56],
    },
    DailyQuestRepeatFlags {
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
        handler_ids: [HandlerId; 2],
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
        handler_id: HandlerId,
        /// Only known one is 1 for explorer mode.
        flags: u8,
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
    PartyMemberPortraits {
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
        /// The placement mode of this waymark.
        placement_mode: WaymarkPlacementMode,
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },
    FreeCompanyHierarchy {
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        leader_name: String,

        #[br(count = 16)]
        #[bw(pad_size_to = 16 * FcHierarchy::SIZE)]
        hierarchy_list: Vec<FcHierarchy>,
    },
    FreeCompanyShortMessage {
        /// The content id of the requested character.
        content_id: u64,
        /// A value the client sends, repeated back to the client.
        sequence: u32,
        /// A 32-bit Unix timestamp indicating when the message was last updated.
        time_last_updated: u32,
        #[brw(pad_size_to = 96)]
        #[br(count = 96)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 8)] // Empty/zeroes
        /// The requested character's FC short message.
        short_message: String,
    },
    FreeCompanyHeader {
        /// The company's ID number. It can also be found in SocialList responses.
        company_id: u64,
        /// The company's crest ID number. Presumably used in places where the company logo is shown.
        crest_id: u64,
        /// Unknown purpose. Possibly for rankings on the Lodestone?
        company_points: u64,
        /// How many company credits the company has to spend on purchases (actions, misc. items, etc.).
        company_credits: u64,
        /// The company's standing with the Grand Company they're allied to.
        reputation: u32,
        /// The amount of points required to rank up.
        next_point: u32,
        /// How many points the company has towards their next rank up.
        current_point: u32,
        /// How many members the company has in total.
        total_members: u16,
        /// How many members in the company are currently online.
        online_members: u16,
        /// The Grand Company this fc is aligned with.
        gc_id: GrandCompany,
        /// The company's current rank (out of 30).
        fc_rank: u8,
        #[brw(pad_size_to = 22)]
        #[br(count = 22)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        /// The company's full name.
        company_name: String,
        #[brw(pad_size_to = 6)]
        #[br(count = 6)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 2)] // Empty/zeroes
        /// The company's short tag.
        company_tag: String,
    },
    FreeCompanyActivityList {
        unk: [u8; 528],
    },
    UnkContentFinder2 {
        unk: [u8; 16],
    },
    Playtime {
        #[brw(pad_after = 4)] // Empty/zeroes
        /// The character's total cumulative playtime, measured in minutes.
        duration: u32,
    },
    Countdown {
        /// The account id of the character that started the countdown.
        account_id: u64,
        /// The content id of the character that started the countdown.
        content_id: u64,
        /// The actor id of the character that started the countdown.
        starter_actor_id: ObjectId,
        unk: u16, // Could be a u8 with padding? Seems to always be 0x5B.
        /// The duration of the countdown in seconds.
        #[brw(pad_after = 3)]
        duration: u16,
        /// The name of the character that started the countdown.
        #[brw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
        #[br(count = CHAR_NAME_MAX_LENGTH)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        #[brw(pad_after = 5)]
        starter_name: String,
    },
    DirectorPopupMessage {
        unk1: u64, // Empty?
        /// Should be the ID of the instance's director.
        handler_id: HandlerId,
        /// See the BNPCName Excel sheet.
        npc_name: u32,
        /// See the InstanceContentTextData Excel sheet.
        text_data_id: u32,
        unk4: u32,
        unk5: u32,
        unk6: u32,
        unk7: u32,
        unk8: u32,
    },
    DirectorSetupMapEffects {
        /// Should be the ID of the instance's director.
        handler_id: HandlerId,
        /// Unsure of what these flags mean.
        unk_flag: u16,
        /// The initial states each map effect should be in.
        states: [u16; 65],
    },
    DirectorMapEffect {
        /// Should be the ID of the instance's director.
        handler_id: HandlerId,
        /// The new state of this map effect.
        state: u16,
        unk1: u16,
        /// The index of the map effect to change.
        index: u8,
        unk2: [u8; 7], // all padding
    },
    UnkHousingRelated {
        unk1: [u8; 9],
        index: u8,
        count: u8,
        unk2: [u8; 2135],
    },
    OwnedHousing {
        #[brw(pad_after = 8)] // believe these are always empty?
        unk1: LandData,
        #[brw(pad_after = 8)]
        unk2: LandData,
        #[brw(pad_after = 8)]
        unk3: LandData,
        unk4: LandData,
        #[brw(pad_after = 8)]
        unk5: LandData,
        /// Your apartment unit.
        #[brw(pad_after = 8)]
        apartment: LandData,
    },
    UpdateFittingShop {
        /// Corresponds to the DisplayId column in the FittingShopCategoryItem Excel sheet.
        #[brw(pad_after = 8)] // empty
        display_ids: [u8; 8],
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
    use crate::common::test_opcodes;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn server_zone_ipc_sizes() {
        test_opcodes::<ServerZoneIpcSegment>();
    }
}
