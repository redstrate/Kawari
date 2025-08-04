use binrw::binrw;

pub use super::social_list::{PlayerEntry, SocialList, SocialListRequest, SocialListRequestType};

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
pub use quest_active_list::QuestActiveList;

mod effect_result;
pub use effect_result::{EffectEntry, EffectResult};

mod condition;
pub use condition::{Condition, Conditions};

use crate::COMPLETED_LEVEQUEST_BITMASK_SIZE;
use crate::COMPLETED_QUEST_BITMASK_SIZE;
use crate::TITLE_UNLOCK_BITMASK_SIZE;
use crate::common::read_string;
use crate::common::write_string;
use crate::inventory::{ContainerType, ItemOperationKind};
use crate::opcodes::ServerZoneIpcType;
use crate::packet::IPC_HEADER_SIZE;
use crate::packet::IpcSegment;
use crate::packet::ReadWriteIpcSegment;

mod actor_move;
pub use crate::ipc::zone::server::actor_move::ActorMove;

pub type ServerZoneIpcSegment = IpcSegment<ServerZoneIpcType, ServerZoneIpcData>;

impl ReadWriteIpcSegment for ServerZoneIpcSegment {
    fn calc_size(&self) -> u32 {
        IPC_HEADER_SIZE
            + match &self.op_code {
                ServerZoneIpcType::Unknown(..) => match &self.data {
                    ServerZoneIpcData::Unknown { unk } => unk.len() as u32,
                    _ => panic!("Unknown packet type doesn't have unknown data?"),
                },
                _ => self.op_code.calc_size(),
            }
    }

    fn get_name(&self) -> &'static str {
        self.op_code.get_name()
    }

    fn get_opcode(&self) -> u16 {
        self.op_code.get_opcode()
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
    /// Sent by the server
    #[br(pre_assert(*magic == ServerZoneIpcType::ActorControl))]
    ActorControl(ActorControl),
    /// Sent by the server when actors that aren't the player move.
    #[br(pre_assert(*magic == ServerZoneIpcType::ActorMove))]
    ActorMove(ActorMove),
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
    #[br(pre_assert(*magic == ServerZoneIpcType::Condition))]
    Condition(Conditions),
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
    /// Sent by the server to acknowledge when the client is updating their inventory in some way (typically when interacting with shops).
    #[br(pre_assert(*magic == ServerZoneIpcType::InventoryActionAck))]
    InventoryActionAck {
        sequence: u32,
        #[brw(pad_after = 10)]
        action_type: u16,
    },
    /// Sent by the server in response to PingReply. In prior expansions, it seems to have had the following additional fields:
    /// origin_entity_id: u32,
    /// <4 bytes of padding before position>
    /// position: Position,
    /// rotation: f32,
    /// <4 bytes of padding after rotation>
    /// but those fields are now seemingly deprecated and used as zero-padding.
    #[br(pre_assert(*magic == ServerZoneIpcType::PingSyncReply))]
    PingSyncReply {
        timestamp: u32,
        #[brw(pad_after = 24)]
        transmission_interval: u32,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::QuestCompleteList))]
    QuestCompleteList {
        #[br(count = COMPLETED_QUEST_BITMASK_SIZE)]
        #[bw(pad_size_to = COMPLETED_QUEST_BITMASK_SIZE)]
        completed_quests: Vec<u8>,
        // TODO: what is in here?
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
    /// Sent by the server to inform the client of when the inventory is being updated (typically when interacting with shops).
    #[br(pre_assert(*magic == ServerZoneIpcType::InventoryTransaction))]
    InventoryTransaction {
        /// This is later reused in InventoryTransactionFinish, so it might be some sort of sequence or context id, but it's not the one sent by the client
        sequence: u32,
        /// Same as the one sent by the client, not the one that the server responds with in InventoryActionAck!
        operation_type: ItemOperationKind,
        #[brw(pad_before = 3)]
        src_actor_id: u32,
        #[brw(pad_size_to = 4)]
        src_storage_id: ContainerType,
        src_container_index: u16,
        #[brw(pad_before = 2)]
        src_stack: u32,
        src_catalog_id: u32,

        /// This section was observed to be static, across two captures and a bunch of discards these never changed.
        /// Always set to 0xE000_0000, also known as no/invalid actor.
        dst_actor_id: u32,
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
    /// Sent by the server when a sequence of InventoryTransaction packets have concluded.
    #[br(pre_assert(*magic == ServerZoneIpcType::InventoryTransactionFinish))]
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
    #[br(pre_assert(*magic == ServerZoneIpcType::ContentFinderUpdate))]
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
    #[br(pre_assert(*magic == ServerZoneIpcType::ContentFinderFound))]
    ContentFinderFound {
        unk1: [u8; 28],
        content_id: u16,
        unk2: [u8; 10],
    },
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
    /// Sent by the server when an item is obtained from shops that accept gil.
    #[br(pre_assert(*magic == ServerZoneIpcType::ShopLogMessage))]
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
    /// Sent by the server when an item is obtained in ways other than gil shops (e.g. Poetics shops).
    #[br(pre_assert(*magic == ServerZoneIpcType::ItemObtainedLogMessage))]
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
    /// Sent by the server typically when a shop transaction takes place, usually to update currency.
    #[br(pre_assert(*magic == ServerZoneIpcType::UpdateInventorySlot))]
    UpdateInventorySlot {
        /// Starts from zero and increases by one for each of these packets during this gameplay session
        sequence: u32,
        #[brw(pad_before = 4)]
        dst_storage_id: u16,
        dst_container_index: u16,
        dst_stack: u32,
        dst_catalog_id: u32,
        #[brw(pad_before = 12, pad_after = 28)]
        /// Always 0x7530_0000, this number appears elsewhere in buybacks so it's probably flags, but what they mean is completely unknown for now
        unk1: u32,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::EffectResult))]
    EffectResult(EffectResult),
    /// Sent to give you the green checkmark before entering a CF zone.
    #[br(pre_assert(*magic == ServerZoneIpcType::ContentFinderCommencing))]
    ContentFinderCommencing { unk1: [u8; 24] },
    #[br(pre_assert(*magic == ServerZoneIpcType::StatusEffectList3))]
    StatusEffectList3 { status_effects: [StatusEffect; 30] },
    #[br(pre_assert(*magic == ServerZoneIpcType::CrossworldLinkshells))]
    CrossworldLinkshells {
        // TODO: fill this out, each entry is 57 bytes probably
        unk1: [u8; 456],
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::SetSearchComment))]
    SetSearchComment {
        // TODO: fill this out
        unk1: [u8; 18],
        #[brw(pad_size_to = 32)]
        #[br(count = 32)]
        #[br(map = read_string)]
        #[bw(map = write_string)]
        comment: String,
        unk2: [u8; 166],
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::Unk17))]
    Unk17 {
        // TODO: fill this out
        unk1: [u8; 968],
    },
    /// Sent by the server when walking over a trigger (e.g. the teleport pads in Solution Nine).
    /// All of these fields are currently unknown in meaning.
    #[br(pre_assert(*magic == ServerZoneIpcType::WalkInEvent))]
    WalkInEvent {
        unk1: u32,
        unk2: u16,
        #[brw(pad_before = 2)]
        unk3: u32,
        unk4: u32,
        #[brw(pad_after = 4)]
        unk5: u32,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::GrandCompanyInfo))]
    GrandCompanyInfo {
        active_company_id: u8,
        maelstrom_rank: u8,
        twin_adder_rank: u8,
        #[brw(pad_after = 4)]
        immortal_flames_rank: u8,
    },
    #[br(pre_assert(*magic == ServerZoneIpcType::CraftingLog))]
    CraftingLog { unk1: [u8; 808] },
    #[br(pre_assert(*magic == ServerZoneIpcType::GatheringLog))]
    GatheringLog { unk1: [u8; 104] },
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
            ),
            (
                ServerZoneIpcType::ActorControl,
                ServerZoneIpcData::ActorControl(ActorControl::default()),
            ),
            (
                ServerZoneIpcType::ActorMove,
                ServerZoneIpcData::ActorMove(ActorMove::default()),
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
                ServerZoneIpcType::Condition,
                ServerZoneIpcData::Condition(Conditions::default()),
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
                ServerZoneIpcType::PingSyncReply,
                ServerZoneIpcData::PingSyncReply {
                    timestamp: 0,
                    transmission_interval: 0,
                },
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
                    sequence: 0,
                    operation_type: ItemOperationKind::Move,
                    src_actor_id: 0,
                    src_storage_id: ContainerType::Inventory0,
                    src_container_index: 0,
                    src_stack: 0,
                    src_catalog_id: 0,
                    dst_actor_id: 0,
                    dummy_container: ContainerType::Inventory0,
                    dst_storage_id: ContainerType::Inventory0,
                    dst_container_index: 0,
                    dst_stack: 0,
                    dst_catalog_id: 0,
                },
            ),
            (
                ServerZoneIpcType::InventoryTransactionFinish,
                ServerZoneIpcData::InventoryTransactionFinish {
                    sequence: 0,
                    sequence_repeat: 0,
                    unk1: 0,
                    unk2: 0,
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
                ServerZoneIpcType::ShopLogMessage,
                ServerZoneIpcData::ShopLogMessage {
                    event_id: 0,
                    message_type: 0,
                    params_count: 0,
                    item_id: 0,
                    item_quantity: 0,
                    total_sale_cost: 0,
                },
            ),
            (
                ServerZoneIpcType::ItemObtainedLogMessage,
                ServerZoneIpcData::ItemObtainedLogMessage {
                    event_id: 0,
                    message_type: 0,
                    params_count: 0,
                    item_id: 0,
                    item_quantity: 0,
                },
            ),
            (
                ServerZoneIpcType::UpdateInventorySlot,
                ServerZoneIpcData::UpdateInventorySlot {
                    sequence: 0,
                    dst_storage_id: 0,
                    dst_container_index: 0,
                    dst_stack: 0,
                    dst_catalog_id: 0,
                    unk1: 0,
                },
            ),
            (
                ServerZoneIpcType::EffectResult,
                ServerZoneIpcData::EffectResult(EffectResult::default()),
            ),
            (
                ServerZoneIpcType::StatusEffectList3,
                ServerZoneIpcData::StatusEffectList3 {
                    status_effects: [StatusEffect::default(); 30],
                },
            ),
            (
                ServerZoneIpcType::SetSearchComment,
                ServerZoneIpcData::SetSearchComment {
                    unk1: [0; 18],
                    comment: String::default(),
                    unk2: [0; 166],
                },
            ),
            (
                ServerZoneIpcType::Unk17,
                ServerZoneIpcData::Unk17 { unk1: [0; 968] },
            ),
            (
                ServerZoneIpcType::WalkInEvent,
                ServerZoneIpcData::WalkInEvent {
                    unk1: 0,
                    unk2: 0,
                    unk3: 0,
                    unk4: 0,
                    unk5: 0,
                },
            ),
        ];

        for (opcode, data) in &ipc_types {
            let mut cursor = Cursor::new(Vec::new());

            let ipc_segment = ServerZoneIpcSegment {
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
