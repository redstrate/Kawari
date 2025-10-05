use crate::{
    common::{ObjectTypeId, workdefinitions::RemakeMode},
    inventory::BuyBackList,
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
        arg: u32,
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
    ToggleOrchestrion {
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
        event_type: u8,
        event_arg: u32,
    },
    SetInnWakeup {
        watched: bool,
    },
    ToggleMount {
        id: u32,
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
}
