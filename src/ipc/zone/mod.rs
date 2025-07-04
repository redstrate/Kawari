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

use crate::common::ObjectTypeId;
use crate::common::Position;
use crate::common::read_string;
use crate::common::write_string;
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
            data: ClientZoneIpcData::InitRequest { unk: [0; 120] },
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
        #[brw(pad_after = 774)]
        #[br(count = 774)]
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
    EventScene(EventScene<2>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene4))]
    EventScene4(EventScene<4>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene8))]
    EventScene8(EventScene<8>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene16))]
    EventScene16(EventScene<16>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene32))]
    EventScene32(EventScene<32>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene64))]
    EventScene64(EventScene<64>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene128))]
    EventScene128(EventScene<128>),
    #[br(pre_assert(*magic == ServerZoneIpcType::EventScene255))]
    EventScene255(EventScene<255>),
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
        // TODO: what is this? a bitmask probably?
        #[br(count = 760)]
        #[bw(pad_size_to = 760)]
        unk1: Vec<u8>,
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
        // TODO: full of possibly interesting information
        unk: [u8; 120],
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
                ServerZoneIpcData::EventScene(EventScene::default()),
            ),
            (
                ServerZoneIpcType::EventScene4,
                ServerZoneIpcData::EventScene4(EventScene::<4>::default()),
            ),
            (
                ServerZoneIpcType::EventScene8,
                ServerZoneIpcData::EventScene8(EventScene::<8>::default()),
            ),
            (
                ServerZoneIpcType::EventScene16,
                ServerZoneIpcData::EventScene16(EventScene::<16>::default()),
            ),
            (
                ServerZoneIpcType::EventScene32,
                ServerZoneIpcData::EventScene32(EventScene::<32>::default()),
            ),
            (
                ServerZoneIpcType::EventScene64,
                ServerZoneIpcData::EventScene64(EventScene::<64>::default()),
            ),
            (
                ServerZoneIpcType::EventScene128,
                ServerZoneIpcData::EventScene128(EventScene::<128>::default()),
            ),
            (
                ServerZoneIpcType::EventScene255,
                ServerZoneIpcData::EventScene255(EventScene::<255>::default()),
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
                ServerZoneIpcType::ActionResult,
                ServerZoneIpcData::ActionResult(ActionResult::default()),
            ),
            (
                ServerZoneIpcType::Equip,
                ServerZoneIpcData::Equip(Equip::default()),
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
        let ipc_types = [(
            ClientZoneIpcType::EventYieldHandler8,
            ClientZoneIpcData::EventYieldHandler8(EventYieldHandler::<8>::default()),
        )];

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
