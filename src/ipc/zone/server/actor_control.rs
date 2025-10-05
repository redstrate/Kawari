use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{EquipDisplayFlag, ObjectId, ObjectTypeId, read_bool_from, write_bool_as};
use crate::ipc::zone::common_emote::CommonEmoteInfo;

use super::OnlineStatus;

// TODO: these are all somewhat related, but maybe should be separated?

// See https://github.com/awgil/ffxiv_reverse/blob/f35b6226c1478234ca2b7149f82d251cffca2f56/vnetlog/vnetlog/ServerIPC.cs#L266 for a REALLY useful list of known values
#[binrw]
#[derive(Debug, Eq, PartialEq, Clone, IntoStaticStr)]
pub enum ActorControlCategory {
    #[brw(magic = 0u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        /// This seems to always be set to 1. If set to another value, the animation glitches for other clients.
        unk_flag: u32,
    },

    #[brw(magic = 15u32)]
    CancelCast {},

    #[brw(magic = 17u32)]
    Cooldown { unk1: u32, unk2: u32, unk3: u32 },

    #[brw(magic = 20u32)]
    GainEffect {
        effect_id: u32,
        param: u32,
        source_actor_id: ObjectId,
    },

    #[brw(magic = 21u32)]
    LoseEffect {
        effect_id: u32,
        unk2: u32,
        source_actor_id: ObjectId,
    },

    #[brw(magic = 24u32)]
    UpdateRestedExp { exp: u32 },

    #[brw(magic = 27u32)]
    Flee { speed: u16 },

    #[brw(magic = 38u32)]
    ToggleInvisibility {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        invisible: bool,
    },

    #[brw(magic = 41u32)]
    ToggleUnlock {
        /// Corresponds to an UnlockLink. Could be a spell, action, emote, etc.
        // See https://github.com/Haselnussbomber/HaselDebug/blob/main/HaselDebug/Tabs/UnlocksTabs/UnlockLinks/UnlockLinksTable.cs
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 50u32)]
    SetTarget {
        #[brw(pad_before = 20)] // Blank since there are no params in the ACT
        target: ObjectTypeId,
    },

    #[brw(magic = 131u32)]
    UnlockInstanceContent {
        /// Index into InstanceContent Excel sheet
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 138u32)]
    EventRelatedUnk3 { event_id: u32 },

    #[brw(magic = 164u32)]
    ToggleAetherCurrentUnlock{
        id: u32, // Index to AetherCurrent sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        attunement_complete: bool, // If true, then "Attunement Complete" will show in the Aether Currents menu, and screen_image_id will show on screen
        // padding, screen_image_id and zone_id are technically a single u32 in the client, but this is more readable
        padding: u8,
        screen_image_id: u16, // Index to ScreenImage sheet. Will only show if attunement_complete is true.
        zone_id: u8, // Index to AetherCurrentCompFlgSet sheet.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unk1: bool, // Same value as attunement_complete
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        show_flying_mounts_help: bool, // Will only be used if attunement_complete is true.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        remove_aether_current: bool, // If true, attunement_complete, screen_image_id and show_flying_mounts_help will be ignored.
    },

    #[brw(magic = 271u32)]
    ToggleAdventureUnlock{
        id: u32, // Index to Adventure sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        all_vistas_recorded: bool,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 200u32)]
    ZoneIn {
        /// When set to 1, it slowly fades the character in.
        warp_finish_anim: u32,
        /// When set to 1, it plays the effects *and* animation for raising.
        raise_anim: u32,
        /// If set to 110, then it plays the "we teleported here" animation.
        #[brw(pad_before = 4)] // empty
        unk1: u32,
    },

    #[brw(magic = 203u32)]
    TeleportStart {
        insufficient_gil: u32,
        aetheryte_id: u32,
    },

    #[brw(magic = 236u32)]
    WalkInTriggerRelatedUnk3 { unk1: u32 },

    #[brw(magic = 253u32)]
    CompanionUnlock {
        unk1: u32,
        unk2: u32, // unlocked?
    },

    #[brw(magic = 254u32)]
    BuddyEquipUnlock {
        id: u32, // Index to BuddyEquip sheet
    },

    #[brw(magic = 260u32)]
    WalkInTriggerRelatedUnk2 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        /// Usually 7?
        unk4: u32,
    },

    #[brw(magic = 270u32)]
    MinionSpawnControl {
        /// When set to 0, the player's minion is despawned.
        minion_id: u32,
    },

    #[brw(magic = 271u32)]
    ToggleMinionUnlock{
        minion_id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 274u32)]
    UpdateHater { unk1: u32 },

    #[brw(magic = 295u32)]
    Pose { unk1: u32, pose: u32 },

    #[brw(magic = 263u32)]
    WalkInTriggerRelatedUnk1 { unk1: u32 },

    #[brw(magic = 290u32)]
    Emote(CommonEmoteInfo),

    #[brw(magic = 324u32)]
    SetCaughtFishBitmask{
        index: u32,
        value: u32,
    },

    #[brw(magic = 343u32)]
    SetCaughtSpearfishBitmask {
        index: u32,
        value: u32,
    },

    #[brw(magic = 378u32)]
    PlayerCurrency { unk1: u32, unk2: u32, unk3: u32 },

    #[brw(magic = 504u32)]
    SetStatusIcon { icon: OnlineStatus },

    #[brw(magic = 509u32)]
    LearnTeleport {
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 510u32)]
    ToggleChocoboTaxiStandUnlock{
        id: u32, // id + 1179648 = Index to ChocoboTaxiStand
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 511u32)]
    EventRelatedUnk1 { unk1: u32 },

    #[brw(magic = 512u32)]
    EventRelatedUnk2 { unk1: u32 },

    #[brw(magic = 324u32)]
    ToggleCutsceneSeen{
        id: u32, // Index to FishParameter
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 517u32)]
    LogMessage {
        log_message: u32, // Index to LogMessage sheet
        id: u32, // Index to variable sheet, depending on LogMessage
    },

    #[brw(magic = 521u32)]
    SetItemLevel { level: u32 },

    #[brw(magic = 608u32)]
    SetEquipDisplayFlags { display_flag: EquipDisplayFlag },

    #[brw(magic = 609u32)]
    ToggleWireframeRendering(),

    #[brw(magic = 902u32)]
    SetFestival {
        festival1: u32, // Multiple festivals can be set at the same time.
        festival2: u32,
        festival3: u32,
        festival4: u32,
    },

    #[brw(magic = 903u32)]
    ToggleMountUnlock {
        /// From the Order column from the Excel row.
        order: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
        /// Row ID of the Mount Excel sheet.
        id: u32,
    },

    /// Forces the player off their current mount.
    #[brw(magic = 915u32)]
    Dismount { sequence: u32 },

    #[brw(magic = 919u32)]
    ToggleOrchestrionUnlock {
        song_id: u32, // Index to Orchestrion sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
        item_id: u32, // Index to Item sheet
    },

    /// Unknown purpose, seen during dismounting.
    #[brw(magic = 930u32)]
    UnkDismountRelated { unk1: u32, unk2: u32, unk3: u32 },

    #[brw(magic = 931u32)]
    BeginContentsReplay {
        unk1: u32, // Always 1
    },

    #[brw(magic = 932u32)]
    EndContentsReplay {
        unk1: u32, // Always 1
    },

    #[brw(magic = 938u32)]
    ToggleOrnamentUnlock {
        id: u32, // Index to Ornament sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 945u32)]
    ToggleGlassesStyleUnlock {
        id: u32, // Index to GlassesStyle sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 1204u32)]
    ToggleTripleTriadCardUnlock {
        id: u32, // Index to TripleTriadCard sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    Unknown {
        category: u32,
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct ActorControl {
    #[brw(pad_after = 4)]
    #[brw(pad_size_to = 20)] // take into account categories without params
    pub category: ActorControlCategory,
}

impl Default for ActorControl {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}

// Has more padding than ActorControl?
#[binrw]
#[derive(Debug, Clone)]
pub struct ActorControlSelf {
    #[brw(pad_after = 12)]
    #[brw(pad_size_to = 20)] // take into account categories without params
    pub category: ActorControlCategory,
}

impl Default for ActorControlSelf {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}

// Has more padding than ActorControl?
#[binrw]
#[derive(Debug, Clone)]
pub struct ActorControlTarget {
    #[brw(pad_size_to = 32)]
    // take into account categories without params, TODO: investigate if the last 4 bytes are padding or a possible 7th param
    pub category: ActorControlCategory,
}

impl Default for ActorControlTarget {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
        }
    }
}
