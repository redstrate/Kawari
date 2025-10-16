use crate::{
    common::{ObjectTypeId, workdefinitions::RemakeMode},
    inventory::BuyBackList,
    ipc::zone::{EventType, ServerZoneIpcSegment},
    packet::PacketSegment,
    world::EventFinishType,
};

#[derive(Clone, Debug)]
pub enum Task {
    ChangeTerritory {
        zone_id: u16,
    },
    SetRemakeMode(RemakeMode),
    Warp {
        warp_id: u32,
    },
    BeginLogOut,
    FinishEvent {
        handler_id: u32,
        finish_type: EventFinishType,
    },
    SetClassJob {
        classjob_id: u8,
    },
    WarpAetheryte {
        aetheryte_id: u32,
    },
    ReloadScripts,
    ToggleInvisibility {
        invisible: bool,
    },
    Unlock {
        id: u32,
    },
    UnlockAetheryte {
        id: u32,
        on: bool,
    },
    SetLevel {
        level: i32,
    },
    ChangeWeather {
        id: u16,
    },
    AddGil {
        amount: u32,
    },
    RemoveGil {
        amount: u32,
        send_client_update: bool,
    },
    GmSetOrchestrion {
        value: bool,
        id: u32,
    },
    AddItem {
        id: u32,
        quantity: u32,
        send_client_update: bool,
    },
    CompleteAllQuests {},
    UnlockContent {
        id: u16,
    },
    UpdateBuyBackList {
        list: BuyBackList,
    },
    AddExp {
        amount: u32,
    },
    StartEvent {
        actor_id: ObjectTypeId,
        event_id: u32,
        event_type: EventType,
        event_arg: u32,
    },
    SetInnWakeup {
        watched: bool,
    },
    ToggleMount {
        id: u32,
    },
    MoveToPopRange {
        id: u32,
    },
    SetHP {
        hp: u32,
    },
    SetMP {
        mp: u16,
    },
    ToggleGlassesStyle {
        id: u32,
    },
    ToggleGlassesStyleAll {},
    ToggleOrnament {
        id: u32,
    },
    ToggleOrnamentAll {},
    UnlockBuddyEquip {
        id: u32,
    },
    UnlockBuddyEquipAll {},
    ToggleChocoboTaxiStand {
        id: u32,
    },
    ToggleChocoboTaxiStandAll {},
    ToggleCaughtFish {
        id: u32,
    },
    ToggleCaughtFishAll {},
    ToggleCaughtSpearfish {
        id: u32,
    },
    ToggleCaughtSpearfishAll {},
    ToggleTripleTriadCard {
        id: u32,
    },
    ToggleTripleTriadCardAll {},
    ToggleAdventure {
        id: u32,
    },
    ToggleAdventureAll {},
    ToggleCutsceneSeen {
        id: u32,
    },
    ToggleCutsceneSeenAll {},
    ToggleMinion {
        id: u32,
    },
    ToggleMinionAll {},
    ToggleAetherCurrent {
        id: u32,
    },
    ToggleAetherCurrentAll {},
    ToggleAetherCurrentCompFlgSet {
        id: u32,
    },
    ToggleAetherCurrentCompFlgSetAll {},
    SetRace {
        race: u8,
    },
    SetTribe {
        tribe: u8,
    },
    SetSex {
        sex: u8,
    },
    // previously, this was kept as a separate thing apart from tasks
    // but I discovered that this doesn't mix well - for example with play_scene (segment-based) and start_event (task)
    // this is because segments were always sent before tasks and there wasn't strong ordering
    // so be careful when changing this system!
    SendSegment {
        segment: PacketSegment<ServerZoneIpcSegment>,
    },
}
