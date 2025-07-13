mod chat_message;
use binrw::binrw;
pub use chat_message::ChatMessage;

mod social_list;
pub use social_list::PlayerEntry;
pub use social_list::SocialList;
pub use social_list::SocialListRequest;
pub use social_list::SocialListRequestType;

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
pub use init_zone::InitZone;

mod npc_spawn;
pub use npc_spawn::NpcSpawn;

mod common_spawn;
pub use common_spawn::{
    BattleNpcSubKind, CharacterMode, CommonSpawn, DisplayFlag, GameMasterRank, ObjectKind,
    OnlineStatus, PlayerSubKind,
};

mod status_effect_list;
pub use status_effect_list::StatusEffectList;

mod weather_change;
pub use weather_change::WeatherChange;

mod action_request;
pub use action_request::ActionRequest;

mod container_info;
pub use container_info::ContainerInfo;

mod item_info;
pub use item_info::ItemInfo;

mod event_scene;
pub use event_scene::EventScene;

mod event_start;
pub use event_start::EventStart;

mod action_result;
pub use action_result::{
    ActionEffect, ActionResult, DamageElement, DamageKind, DamageType, EffectKind,
};

mod r#move;
pub use r#move::Move;

mod warp;
pub use warp::Warp;

mod item_operation;
pub use item_operation::ItemOperation;

mod equip;
pub use equip::Equip;

mod client_trigger;
pub use client_trigger::{ClientTrigger, ClientTriggerCommand};

mod currency_info;
pub use currency_info::CurrencyInfo;

mod config;
pub use config::Config;

mod event_yield_handler;
pub use event_yield_handler::EventYieldHandler;

mod object_spawn;
pub use object_spawn::ObjectSpawn;

mod quest_active_list;
pub use quest_active_list::QuestActiveList;

use crate::COMPLETED_LEVEQUEST_BITMASK_SIZE;
use crate::COMPLETED_QUEST_BITMASK_SIZE;
use crate::TITLE_UNLOCK_BITMASK_SIZE;
use crate::common::ObjectTypeId;
use crate::common::Position;
use crate::common::read_string;
use crate::common::write_string;
use crate::inventory::{ContainerType, ItemOperationKind};
use crate::opcodes::ClientZoneIpcType;
use crate::opcodes::ServerZoneIpcType;
use crate::packet::IPC_HEADER_SIZE;
use crate::packet::IpcSegment;
use crate::packet::ReadWriteIpcSegment;

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
}

// TODO: make generic
impl Default for ClientZoneIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ClientZoneIpcType::InitRequest,
            option: 0,
            timestamp: 0,
            data: ClientZoneIpcData::InitRequest {
                unk1: String::default(),
                unk2: String::default(),
            },
        }
    }
}

pub type ServerZoneIpcSegment = IpcSegment<ServerZoneIpcType, ServerZoneIpcData>;

impl ReadWriteIpcSegment for ServerZoneIpcSegment {
    fn calc_size(&self) -> u32 {
        IPC_HEADER_SIZE + self.op_code.calc_size()
    }

    fn get_name(&self) -> &'static str {
        self.op_code.get_name()
    }

    fn get_opcode(&self) -> u16 {
        self.op_code.get_opcode()
    }
}

// TODO: make generic
impl Default for ServerZoneIpcSegment {
    fn default() -> Self {
        Self {
            unk1: 0x14,
            unk2: 0,
            op_code: ServerZoneIpcType::InitZone,
            option: 0,
            timestamp: 0,
            data: ServerZoneIpcData::InitZone(InitZone::default()),
        }
    }
}

#[binrw]
#[br(import(magic: &ServerZoneIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ServerZoneIpcData {
    /// Sent by the server as response to ZoneInitRequest.
    #[br(pre_assert(*magic == ServerZoneIpcType::InitResponse))]
    InitResponse {
        unk1: u64,
        character_id: u32,
        unk2: u32,
    },
    /// Sent by the server that tells the client which zone to load
    #[br(pre_assert(*magic == ServerZoneIpcType::InitZone))]
    InitZone(InitZone),
    /// Sent by the server for... something
    #[br(pre_assert(*magic == ServerZoneIpcType::ActorControlSelf))]
    ActorControlSelf(ActorControlSelf),
    /// Sent by the server containing character stats
    #[br(pre_assert(*magic == ServerZoneIpcType::PlayerStats))]
    PlayerStats(PlayerStats),
    /// Sent by the server to setup the player on the client
    #[br(pre_assert(*magic == ServerZoneIpcType::PlayerStatus))]
    PlayerStatus(PlayerStatus),
    /// Sent by the server to setup class info
    #[br(pre_assert(*magic == ServerZoneIpcType::UpdateClassInfo))]
    UpdateClassInfo(UpdateClassInfo),
    /// Sent by the server to spawn the player in
    #[br(pre_assert(*magic == ServerZoneIpcType::PlayerSpawn))]
    PlayerSpawn(PlayerSpawn),
    /// Sent by the server to indicate the log out is complete
    #[br(pre_assert(*magic == ServerZoneIpcType::LogOutComplete))]
    LogOutComplete {
        // TODO: guessed
        unk: [u8; 8],
    },
    /// Sent by the server to modify the client's position
    #[br(pre_assert(*magic == ServerZoneIpcType::Warp))]
    Warp(Warp),
    /// Sent by the server when they send a chat message
    #[br(pre_assert(*magic == ServerZoneIpcType::ServerChatMessage))]
    ServerChatMessage {
        /*
         * bits (properties will apply when set, but a final base 10 value of zero defaults to chat log only):
         * 76543210
         * xxxxxSxC
         * x = don't care/unused
         * S = on-screen
         * C = chat log
         * all other bits are unused, therefore some possible examples are (base 10 values follow):
         * 1 = chat log only
         * 4 = on-screen only
         * 5 = both
         * ref: https://github.com/SapphireServer/Sapphire/blob/bf3368224a00c180cbb7ba413b52395eba58ec0b/src/common/Network/PacketDef/Zone/ServerZoneDef.h#L250
         */
        param: u8,
        #[brw(pad_size_to = 775)]
        #[br(count = 775)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        message: String,
    },
    /// Unknown, but seems to contain information on cross-world linkshells
    #[br(pre_assert(*magic == ServerZoneIpcType::LinkShellInformation))]
    LinkShellInformation { unk: [u8; 456] },
    /// Sent by the server when it wants the client to... prepare to zone?
    #[br(pre_assert(*magic == ServerZoneIpcType::PrepareZoning))]
    PrepareZoning { unk: [u32; 4] },
    /// Sent by the server
    #[br(pre_assert(*magic == ServerZoneIpcType::ActorControl))]
    ActorControl(ActorControl),
    /// Sent by the server
    #[br(pre_assert(*magic == ServerZoneIpcType::Move))]
    Move(Move),
    /// Sent by the server in response to SocialListRequest
    #[br(pre_assert(*magic == ServerZoneIpcType::SocialList))]
    SocialList(SocialList),
    /// Sent by the server to spawn an NPC
    #[br(pre_assert(*magic == ServerZoneIpcType::NpcSpawn))]
    NpcSpawn(NpcSpawn),
    /// Sent by the server to update an actor's status effect list
    #[br(pre_assert(*magic == ServerZoneIpcType::StatusEffectList))]
    StatusEffectList(StatusEffectList),
    /// Sent by the server when it's time to change the weather
    #[br(pre_assert(*magic == ServerZoneIpcType::WeatherId))]
    WeatherId(WeatherChange),
    /// Sent to inform the client of an inventory item
    #[br(pre_assert(*magic == ServerZoneIpcType::UpdateItem))]
    UpdateItem(ItemInfo),
    /// Sent to inform the client of container status
    #[br(pre_assert(*magic == ServerZoneIpcType::ContainerInfo))]
    ContainerInfo(ContainerInfo),
    /// Sent to tell the client to play a scene
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene))]
    #[brw(little)]
    EventScene {
        #[brw(args { max_params: 2 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene4))]
    #[brw(little)]
    EventScene4 {
        #[brw(args { max_params: 4 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene8))]
    #[brw(little)]
    EventScene8 {
        #[brw(args { max_params: 8 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene16))]
    #[brw(little)]
    EventScene16 {
        #[brw(args { max_params: 16 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene32))]
    #[brw(little)]
    EventScene32 {
        #[brw(args { max_params: 32 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene64))]
    #[brw(little)]
    EventScene64 {
        #[brw(args { max_params: 64 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene128))]
    #[brw(little)]
    EventScene128 {
        #[brw(args { max_params: 128 } )]
        data: EventScene,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene255))]
    #[brw(little)]
    EventScene255 {
        #[brw(args { max_params: 255 } )]
        data: EventScene,
    },
    /// Sent to tell the client to load a scene, but not play it
    #[br(pre_assert(*magic == ServerZoneIpcType::EventStart))]
    EventStart(EventStart),
    /// Sent to update an actor's hp & mp values
    #[br(pre_assert(*magic == ServerZoneIpcType::UpdateHpMpTp))]
    UpdateHpMpTp {
        hp: u32,
        mp: u16,
        unk: u16, // it's filled with... something
    },
    /// Sent to inform the client the consequences of their actions
    #[br(pre_assert(*magic == ServerZoneIpcType::ActionResult))]
    ActionResult(ActionResult),
    /// Sent to to the client to update their appearance
    #[br(pre_assert(*magic == ServerZoneIpcType::Equip))]
    Equip(Equip),
    /// Sent to the client to free up a spawn index
    #[br(pre_assert(*magic == ServerZoneIpcType::Delete))]
    Delete {
        spawn_index: u8,
        #[brw(pad_before = 3)] // padding
        actor_id: u32,
    },
    /// Sent to the client to stop their currently playing event.
    #[br(pre_assert(*magic == ServerZoneIpcType::EventFinish))]
    EventFinish {
        handler_id: u32,
        event: u8,
        result: u8,
        #[brw(pad_before = 2)] // padding
        #[brw(pad_after = 4)] // padding
        arg: u32,
    },
    /// Sent after EventFinish? it un-occupies the character lol
    #[br(pre_assert(*magic == ServerZoneIpcType::Unk18))]
    Unk18 {
        unk: [u8; 16], // all zero...
    },
    /// Used to control target information
    #[br(pre_assert(*magic == ServerZoneIpcType::ActorControlTarget))]
    ActorControlTarget(ActorControlTarget),
    /// Used to update the player's currencies
    #[br(pre_assert(*magic == ServerZoneIpcType::CurrencyCrystalInfo))]
    CurrencyCrystalInfo(CurrencyInfo),
    /// Used to update an actor's equip display flags
    #[br(pre_assert(*magic == ServerZoneIpcType::Config))]
    Config(Config),
    /// Unknown, seen in haircut event
    #[br(pre_assert(*magic == ServerZoneIpcType::EventUnkReply))]
    EventUnkReply {
        event_id: u32,
        unk1: u16,
        unk2: u8,
        #[brw(pad_after = 8)]
        unk3: u8,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::InventoryActionAck))]
    InventoryActionAck {
        sequence: u32,
        #[brw(pad_after = 10)]
        action_type: u16,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::UnkCall))]
    UnkCall {
        unk1: u32,
        #[brw(pad_after = 26)]
        unk2: u16,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::QuestCompleteList))]
    QuestCompleteList {
        #[br(count = COMPLETED_QUEST_BITMASK_SIZE)]
        #[bw(pad_size_to = COMPLETED_QUEST_BITMASK_SIZE)]
        completed_quests: Vec<u8>,
        // TODO: what is in ehre?
        #[br(count = 69)]
        #[bw(pad_size_to = 69)]
        unk2: Vec<u8>,
    },
    /// Unsure the true purpose of this, but it's needed for the Unending Journey to function.
    #[br(pre_assert(*magic == ServerZoneIpcType::UnkResponse2))]
    UnkResponse2 {
        #[brw(pad_after = 7)]
        unk1: u8,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::InventoryTransaction))]
    InventoryTransaction {
        /// This is later reused in InventoryTransactionFinish, so it might be some sort of sequence or context id, but it's not the one sent by the client
        unk1: u32,
        /// Same as the one sent by the client, not the one that the server responds with in InventoryActionAck!
        operation_type: ItemOperationKind,
        #[brw(pad_before = 3)]
        src_actor_id: u32,
        src_storage_id: ContainerType,
        src_container_index: u16,
        #[brw(pad_before = 2)]
        /// On retail, this contains very strange values that don't make sense (in regards to stack size). It's possible this field is a different size or isn't intended for stack size.
        unk2: u32,
        /// On retail, this contains very strange values that don't make sense (in regards to catalog id). It's possible this field is a different size or isn't intended for catalog id.
        unk3: u32,

        /// This is all static as far as I can tell, across two captures and a bunch of discards these never changed
        /// seems to always be 0
        dst_actor_id: u32,
        /// seems to always be 57344/0xE000
        dst_storage_id: u16,
        /// seems to always be 65535/0xFFFF
        dst_container_index: u16,
        /// seems to always be 0x0000FFFF
        #[brw(pad_after = 10)]
        dst_catalog_id: u32,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::InventoryTransactionFinish))]
    InventoryTransactionFinish {
        /// Same value as unk1 in InventoryTransaction.
        unk1: u32,
        /// Repeated unk1 value.
        unk2: u32,
        /// Unknown, seems to always be 0x00000090.
        unk3: u32,
        /// Unknown, seems to always be 0x00000200.
        unk4: u32,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::ContentFinderFound))]
    ContentFinderFound {
        /// 0 = Nothing happens
        /// 1 = Reserving server
        /// 2 = again? ^
        /// 3 = duty ready
        /// 4 = checking member status
        /// nothing appears to happen above 5
        state1: u8,
        classjob_id: u8,
        unk1: [u8; 38],
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::ContentFinderFound2))]
    ContentFinderFound2 { unk1: [u8; 8] },
    #[br(pre_assert(*magic == ServerZoneIpcType::ObjectSpawn))]
    ObjectSpawn(ObjectSpawn),
    #[br(pre_assert(*magic == ServerZoneIpcType::ActorGauge))]
    ActorGauge { classjob_id: u8, data: [u8; 15] },
    #[br(pre_assert(*magic == ServerZoneIpcType::UpdateSearchInfo))]
    UpdateSearchInfo {
        online_status_flags: u64,
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
    #[br(pre_assert(*magic == ServerZoneIpcType::FreeCompanyInfo))]
    FreeCompanyInfo { unk: [u8; 80] },
    #[br(pre_assert(*magic == ServerZoneIpcType::TitleList))]
    TitleList {
        unlock_bitmask: [u8; TITLE_UNLOCK_BITMASK_SIZE],
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::QuestActiveList))]
    QuestActiveList(QuestActiveList),
    #[br(pre_assert(*magic == ServerZoneIpcType::LevequestCompleteList))]
    LevequestCompleteList {
        #[br(count = COMPLETED_LEVEQUEST_BITMASK_SIZE)]
        #[bw(pad_size_to = COMPLETED_LEVEQUEST_BITMASK_SIZE)]
        completed_levequests: Vec<u8>,
        // TODO: what is in ehre?
        #[br(count = 6)]
        #[bw(pad_size_to = 6)]
        unk2: Vec<u8>,
    },
    #[brw(little)]
    #[br(pre_assert(*magic == ServerZoneIpcType::GilShopTransactionAck))]
    GilShopTransactionAck {
        event_id: u32,
        /// Always 0x697 when buying?
        unk1: u32,
        /// Always 3 when buying?
        unk2: u32,
        item_id: u32,
        item_quantity: u32,
        #[brw(pad_after = 8)]
        total_sale_cost: u32,
    },
    #[brw(little)]
    #[br(pre_assert(*magic == ServerZoneIpcType::UpdateInventorySlot))]
    UpdateInventorySlot {
        /// Starts from zero and increases by one for each of these packets during this gameplay session
        sequence: u32,
        #[brw(pad_before = 4)]
        /// On the first pass, it's 0x07D0, and on the second pass it's the destination storage id
        unk1_and_dst_storage_id: u16,
        /// On the first pass it's 0x0000, and on the second pass it's the slot number within the destination storage
        unk2_and_dst_slot: u16,
        /// On the first pass, it's the player's remaining gil after the purchase, and on the second pass it's the number of items in that slot after purchase
        gil_and_item_quantity: u32,
        /// On the first pass it's 0x0000_0001 and on the second pass it's the item id
        unk3_and_item_id: u32,
        #[brw(pad_before = 12, pad_after = 28)]
        /// Always 0x7530_0000
        unk4: u32,
    },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

#[binrw]
#[br(import(magic: &ClientZoneIpcType, size: &u32))]
#[derive(Debug, Clone)]
pub enum ClientZoneIpcData {
    /// Sent by the client when they successfully initialize with the server, and they need several bits of information (e.g. what zone to load)
    #[br(pre_assert(*magic == ClientZoneIpcType::InitRequest))]
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
    /// Sent by the client when they're done loading and they need to be spawned in
    #[br(pre_assert(*magic == ClientZoneIpcType::FinishLoading))]
    FinishLoading {
        // TODO: full of possibly interesting information
        unk: [u8; 72],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::ClientTrigger))]
    ClientTrigger(ClientTrigger),
    /// FIXME: 16 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk2))]
    Unk2 {
        // TODO: full of possibly interesting information
        unk: [u8; 16],
    },
    /// FIXME: 8 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk3))]
    Unk3 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// FIXME: 8 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk4))]
    Unk4 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::SetSearchInfoHandler))]
    SetSearchInfoHandler {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// FIXME: 8 bytes of something from the client, not sure what yet
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk5))]
    Unk5 {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// Sent by the client when it requests the friends list and other related info
    #[br(pre_assert(*magic == ClientZoneIpcType::SocialListRequest))]
    SocialListRequest(SocialListRequest),
    #[br(pre_assert(*magic == ClientZoneIpcType::UpdatePositionHandler))]
    UpdatePositionHandler {
        /// In radians.
        #[brw(pad_after = 4)] // empty
        rotation: f32,
        #[brw(pad_after = 4)] // empty
        position: Position,
    },
    /// Sent by the client when the user requests to log out
    #[br(pre_assert(*magic == ClientZoneIpcType::LogOut))]
    LogOut {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// Sent by the client when it's actually disconnecting
    #[br(pre_assert(*magic == ClientZoneIpcType::Disconnected))]
    Disconnected {
        // TODO: full of possibly interesting information
        unk: [u8; 8],
    },
    /// Sent by the client when they send a chat message
    #[br(pre_assert(*magic == ClientZoneIpcType::ChatMessage))]
    ChatMessage(ChatMessage),
    /// Sent by the client when they send a GM command. This can only be sent by the client if they are sent a GM rank.
    #[br(pre_assert(*magic == ClientZoneIpcType::GMCommand))]
    GMCommand {
        command: u32,
        arg0: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
        #[brw(pad_after = 4)]
        target: u64,
    },
    /// Sent by the client when the character walks into a zone transistion
    #[br(pre_assert(*magic == ClientZoneIpcType::ZoneJump))]
    ZoneJump {
        exit_box: u32,
        position: Position,
        #[brw(pad_after = 4)] // padding
        landset_index: i32,
    },
    /// Sent by the client when a character performs an action
    #[br(pre_assert(*magic == ClientZoneIpcType::ActionRequest))]
    ActionRequest(ActionRequest),
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk16))]
    Unk16 {
        unk: [u8; 8], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk17))]
    Unk17 {
        unk1: u32,
        unk2: [u8; 28], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk18))]
    Unk18 {
        unk: [u8; 8], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::EventRelatedUnk))]
    EventRelatedUnk {
        unk1: u32,
        unk2: u16,
        #[brw(pad_before = 2)]
        unk3: u32,
        unk4: u32,
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::Unk19))]
    Unk19 {
        unk: [u8; 16], // TODO: unknown
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::ItemOperation))]
    ItemOperation(ItemOperation),
    #[br(pre_assert(*magic == ClientZoneIpcType::StartTalkEvent))]
    StartTalkEvent {
        actor_id: ObjectTypeId,
        #[brw(pad_after = 4)] // padding
        event_id: u32,
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::GilShopTransaction))]
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
    /// This packet is sent by the client when they pivot left or right on standard controls.
    /// It is sent once when beginning to pivot, and once when pivoting ends.
    #[br(pre_assert(*magic == ClientZoneIpcType::StandardControlsPivot))]
    StandardControlsPivot {
        /// Set to 4 when beginning to pivot.
        /// Set to 0 when pivoting ends.
        #[brw(pad_after = 4)]
        is_pivoting: u32,
    },
    #[br(pre_assert(*magic == ClientZoneIpcType::EventYieldHandler))]
    EventYieldHandler(EventYieldHandler<2>),
    #[br(pre_assert(*magic == ClientZoneIpcType::EventYieldHandler8))]
    EventYieldHandler8(EventYieldHandler<8>),
    #[br(pre_assert(*magic == ClientZoneIpcType::Config))]
    Config(Config),
    #[br(pre_assert(*magic == ClientZoneIpcType::EventUnkRequest))]
    EventUnkRequest {
        event_id: u32,
        unk1: u16,
        unk2: u8,
        #[brw(pad_after = 8)]
        unk3: u8,
    },
    /// Unsure the true purpose of this, but it's needed for the Unending Journey to function.
    #[br(pre_assert(*magic == ClientZoneIpcType::UnkCall2))]
    UnkCall2 { unk1: [u8; 8] },
    #[br(pre_assert(*magic == ClientZoneIpcType::ContentFinderRegister))]
    ContentFinderRegister { unk1: [u8; 40] },
    Unknown {
        #[br(count = size - 32)]
        unk: Vec<u8>,
    },
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use binrw::BinWrite;

    use crate::opcodes::ServerZoneIpcType;

    use super::*;

    /// Ensure that the IPC data size as reported matches up with what we write
    #[test]
    fn server_zone_ipc_sizes() {
        let ipc_types = [
            (
                ServerZoneIpcType::InitResponse,
                ServerZoneIpcData::InitResponse {
                    unk1: 0,
                    character_id: 0,
                    unk2: 0,
                },
            ),
            (
                ServerZoneIpcType::InitZone,
                ServerZoneIpcData::InitZone(InitZone::default()),
            ),
            (
                ServerZoneIpcType::ActorControlSelf,
                ServerZoneIpcData::ActorControlSelf(ActorControlSelf::default()),
            ),
            (
                ServerZoneIpcType::PlayerStats,
                ServerZoneIpcData::PlayerStats(PlayerStats::default()),
            ),
            (
                ServerZoneIpcType::PlayerStatus,
                ServerZoneIpcData::PlayerStatus(PlayerStatus::default()),
            ),
            (
                ServerZoneIpcType::UpdateClassInfo,
                ServerZoneIpcData::UpdateClassInfo(UpdateClassInfo::default()),
            ),
            (
                ServerZoneIpcType::PlayerSpawn,
                ServerZoneIpcData::PlayerSpawn(PlayerSpawn::default()),
            ),
            (
                ServerZoneIpcType::LogOutComplete,
                ServerZoneIpcData::LogOutComplete { unk: [0; 8] },
            ),
            (
                ServerZoneIpcType::Warp,
                ServerZoneIpcData::Warp(Warp::default()),
            ),
            (
                ServerZoneIpcType::ServerChatMessage,
                ServerZoneIpcData::ServerChatMessage {
                    param: 0,
                    message: String::new(),
                },
            ),
            (
                ServerZoneIpcType::LinkShellInformation,
                ServerZoneIpcData::LinkShellInformation { unk: [0; 456] },
            ),
            (
                ServerZoneIpcType::PrepareZoning,
                ServerZoneIpcData::PrepareZoning { unk: [0; 4] },
            ),
            (
                ServerZoneIpcType::ActorControl,
                ServerZoneIpcData::ActorControl(ActorControl::default()),
            ),
            (
                ServerZoneIpcType::Move,
                ServerZoneIpcData::Move(Move::default()),
            ),
            (
                ServerZoneIpcType::SocialList,
                ServerZoneIpcData::SocialList(SocialList::default()),
            ),
            (
                ServerZoneIpcType::NpcSpawn,
                ServerZoneIpcData::NpcSpawn(NpcSpawn::default()),
            ),
            (
                ServerZoneIpcType::StatusEffectList,
                ServerZoneIpcData::StatusEffectList(StatusEffectList::default()),
            ),
            (
                ServerZoneIpcType::WeatherId,
                ServerZoneIpcData::WeatherId(WeatherChange::default()),
            ),
            (
                ServerZoneIpcType::UpdateItem,
                ServerZoneIpcData::UpdateItem(ItemInfo::default()),
            ),
            (
                ServerZoneIpcType::ContainerInfo,
                ServerZoneIpcData::ContainerInfo(ContainerInfo::default()),
            ),
            (
                ServerZoneIpcType::EventScene,
                ServerZoneIpcData::EventScene {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene4,
                ServerZoneIpcData::EventScene4 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene8,
                ServerZoneIpcData::EventScene8 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene16,
                ServerZoneIpcData::EventScene16 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene32,
                ServerZoneIpcData::EventScene32 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene64,
                ServerZoneIpcData::EventScene64 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene128,
                ServerZoneIpcData::EventScene128 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventScene255,
                ServerZoneIpcData::EventScene255 {
                    data: EventScene::default(),
                },
            ),
            (
                ServerZoneIpcType::EventStart,
                ServerZoneIpcData::EventStart(EventStart::default()),
            ),
            (
                ServerZoneIpcType::UpdateHpMpTp,
                ServerZoneIpcData::UpdateHpMpTp {
                    hp: 0,
                    mp: 0,
                    unk: 0,
                },
            ),
            (
                ServerZoneIpcType::Equip,
                ServerZoneIpcData::Equip(Equip::default()),
            ),
            (
                ServerZoneIpcType::ActionResult,
                ServerZoneIpcData::ActionResult(ActionResult::default()),
            ),
            (
                ServerZoneIpcType::Delete,
                ServerZoneIpcData::Delete {
                    spawn_index: 0,
                    actor_id: 0,
                },
            ),
            (
                ServerZoneIpcType::EventFinish,
                ServerZoneIpcData::EventFinish {
                    handler_id: 0,
                    event: 0,
                    result: 0,
                    arg: 0,
                },
            ),
            (
                ServerZoneIpcType::Unk18,
                ServerZoneIpcData::Unk18 { unk: [0; 16] },
            ),
            (
                ServerZoneIpcType::ActorControlTarget,
                ServerZoneIpcData::ActorControlTarget(ActorControlTarget::default()),
            ),
            (
                ServerZoneIpcType::CurrencyCrystalInfo,
                ServerZoneIpcData::CurrencyCrystalInfo(CurrencyInfo::default()),
            ),
            (
                ServerZoneIpcType::EventUnkReply,
                ServerZoneIpcData::EventUnkReply {
                    event_id: 0,
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                },
            ),
            (
                ServerZoneIpcType::InventoryActionAck,
                ServerZoneIpcData::InventoryActionAck {
                    sequence: 0,
                    action_type: 0,
                },
            ),
            (
                ServerZoneIpcType::UnkCall,
                ServerZoneIpcData::UnkCall { unk1: 0, unk2: 0 },
            ),
            (
                ServerZoneIpcType::QuestCompleteList,
                ServerZoneIpcData::QuestCompleteList {
                    completed_quests: Vec::default(),
                    unk2: Vec::default(),
                },
            ),
            (
                ServerZoneIpcType::UnkResponse2,
                ServerZoneIpcData::UnkResponse2 { unk1: 0 },
            ),
            (
                ServerZoneIpcType::InventoryTransaction,
                ServerZoneIpcData::InventoryTransaction {
                    unk1: 0,
                    operation_type: ItemOperationKind::Move,
                    src_actor_id: 0,
                    src_storage_id: ContainerType::Inventory0,
                    src_container_index: 0,
                    unk2: 0,
                    unk3: 0,
                    dst_actor_id: 0,
                    dst_storage_id: 0,
                    dst_container_index: 0,
                    dst_catalog_id: 0,
                },
            ),
            (
                ServerZoneIpcType::InventoryTransactionFinish,
                ServerZoneIpcData::InventoryTransactionFinish {
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                },
            ),
            (
                ServerZoneIpcType::ObjectSpawn,
                ServerZoneIpcData::ObjectSpawn(ObjectSpawn::default()),
            ),
            (
                ServerZoneIpcType::ActorGauge,
                ServerZoneIpcData::ActorGauge {
                    classjob_id: 0,
                    data: [0; 15],
                },
            ),
            (
                ServerZoneIpcType::UpdateSearchInfo,
                ServerZoneIpcData::UpdateSearchInfo {
                    online_status_flags: 0,
                    unk1: 0,
                    unk2: 0,
                    region: 0,
                    message: String::default(),
                },
            ),
            (
                ServerZoneIpcType::TitleList,
                ServerZoneIpcData::TitleList {
                    unlock_bitmask: [0; TITLE_UNLOCK_BITMASK_SIZE],
                },
            ),
            (
                ServerZoneIpcType::FreeCompanyInfo,
                ServerZoneIpcData::FreeCompanyInfo { unk: [0; 80] },
            ),
            (
                ServerZoneIpcType::QuestActiveList,
                ServerZoneIpcData::QuestActiveList(QuestActiveList::default()),
            ),
            (
                ServerZoneIpcType::LevequestCompleteList,
                ServerZoneIpcData::LevequestCompleteList {
                    completed_levequests: Vec::default(),
                    unk2: Vec::default(),
                },
            ),
            (
                ServerZoneIpcType::GilShopTransactionAck,
                ServerZoneIpcData::GilShopTransactionAck {
                    event_id: 0,
                    unk1: 0,
                    unk2: 0,
                    item_id: 0,
                    item_quantity: 0,
                    total_sale_cost: 0,
                },
            ),
            (
                ServerZoneIpcType::UpdateInventorySlot,
                ServerZoneIpcData::UpdateInventorySlot {
                    sequence: 0,
                    unk1_and_dst_storage_id: 0,
                    unk2_and_dst_slot: 0,
                    gil_and_item_quantity: 0,
                    unk3_and_item_id: 0,
                    unk4: 0,
                },
            ),
        ];

        for (opcode, data) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerZoneIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: opcode.clone(), // doesn't matter for this test
                option: 0,
                timestamp: 0,
                data: data.clone(),
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
                ClientZoneIpcType::Unk2,
                ClientZoneIpcData::Unk2 { unk: [0; 16] },
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
                ClientZoneIpcType::ChatMessage,
                ClientZoneIpcData::ChatMessage(ChatMessage::default()),
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
                ClientZoneIpcType::Unk17,
                ClientZoneIpcData::Unk17 {
                    unk1: 0,
                    unk2: [0; 28],
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
        ];

        for (opcode, data) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ClientZoneIpcSegment {
                unk1: 0,
                unk2: 0,
                op_code: opcode.clone(), // doesn't matter for this test
                option: 0,
                timestamp: 0,
                data: data.clone(),
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
