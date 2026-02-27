use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{
    CharacterMode, DirectorEvent, EquipDisplayFlag, HandlerId, InvisibilityFlags, ObjectId,
    ObjectTypeId, read_bool_from, read_packed_float, write_bool_as, write_packed_float,
};
use crate::ipc::zone::CommonEmoteInfo;
use crate::ipc::zone::online_status::OnlineStatus;

#[binrw]
#[derive(Debug, PartialEq, Clone)]
pub enum LiveEventType {
    /// Begins a new crafting session.
    #[brw(magic = 8u32)]
    StartCraft {
        /// 0, 1, 2 plays an ActionTimeline (???) and 3, 4, 5 does something else?
        unk1: u32,
        unk2: u32,
        unk3: u32,
    },

    /// Ends the current crafting session.
    #[brw(magic = 12u32)]
    PlayAnimation {
        /// Index into the ActionTimeline Excel sheet.
        animation_start: u32,
        /// Index into the ActionTimeline Excel sheet.
        animation_end: u32,
    },

    /// Ends the current crafting session.
    #[brw(magic = 15u32)]
    EndCraft {},

    /// Sets the main hand weapon model.
    #[brw(magic = 38u32)]
    SetMainHand { model_id: u32, unk1: u32, unk2: u32 },

    /// Sets the off hand weapon model.
    #[brw(magic = 39u32)]
    SetOffHand { model_id: u32, unk1: u32, unk2: u32 },

    Unknown {
        event: u32,
        param1: u32,
        param2: u32,
        param3: u32,
    },
}

// See https://github.com/awgil/ffxiv_reverse/blob/f35b6226c1478234ca2b7149f82d251cffca2f56/vnetlog/vnetlog/ServerIPC.cs#L266 for a REALLY useful list of known values
#[binrw]
#[derive(Debug, PartialEq, Clone, IntoStaticStr)]
pub enum ActorControlCategory {
    #[brw(magic = 0u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        /// This seems to always be set to 1. If set to another value, the animation glitches for other clients.
        unk_flag: u32,
    },

    /// Unknown purpose, but seen while emoting.
    #[brw(magic = 2u32)]
    SetMode { mode: CharacterMode, mode_arg: u32 },

    /// Begins an event action.
    #[brw(magic = 3u32)]
    EventAction {
        /// Seems to be 1 to start it?
        unk1: u32,
        /// Index into the EventAction Excel sheet.
        id: u32,
    },

    /// Only plays the VFX and nothing else.
    #[brw(magic = 5u32)]
    ClassJobChangeVFX {
        /// Index into the ClassJob Excel sheet.
        classjob_id: u32,
    },

    /// Only shows the floating message, it doesn't actually update your EXP!
    #[brw(magic = 7u32)]
    EXPFloatingMessage {
        /// The classjob ID to gain experience for.
        classjob_id: u32,
        /// The amount of EXP.
        amount: u32,
        /// Percentage of bonus EXP to display. Note that this *doesn't* apply it client-side, this is simply an indicator.
        bonus_percent: u32,
    },

    /// Only updates the UI, it's wiped on the next ClassInfo update.
    #[brw(magic = 8u32)]
    UnlockClass { classjob_id: u32 },

    /// Only updates the UI, it's wiped on the next ClassInfo update.
    #[brw(magic = 9u32)]
    SetLevel { classjob_id: u32, level: u32 },

    /// Shows the message and also updates some character data.
    /// It's also interesting in that it handles the in-between levels. So you can give it a level of 99 and it'll go through *each* level.
    #[brw(magic = 10u32)]
    LevelUpMessage {
        classjob_id: u32,
        level: u32,
        unk2: u32,
        unk3: u32,
    },

    /// Kills this actor, including playing an animation and setting HP/MP to zero.
    /// Does *not* change the state of the actor.
    #[brw(magic = 14u32)]
    Kill {
        /// Index into the ActionTimeline sheet. If 0, plays the default death animation.
        animation_id: u32,
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

    /// Updates the rested EXP bonus shown in the EXP bar.
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

    /// Seen while dead actors fade away.
    #[brw(magic = 39u32)]
    DeadFadeOut {},

    #[brw(magic = 41u32)]
    ToggleUnlock {
        /// Corresponds to an UnlockLink. Could be a spell, action, emote, etc.
        // See https://github.com/Haselnussbomber/HaselDebug/blob/main/HaselDebug/Tabs/UnlocksTabs/UnlockLinks/UnlockLinksTable.cs
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    /// Sets the EXP, but doesn't display anything visually.
    #[brw(magic = 43u32)]
    SetEXP { classjob_id: u32, amount: u32 },

    #[brw(magic = 50u32)]
    SetTarget {},

    // Calls into many inventory-related functions, haven't looked too far yet.
    #[brw(magic = 84u32)]
    UnkInventoryRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 100u32)]
    InitDirector {
        /// The director to intiailize.
        handler_id: HandlerId,
        /// Content ID for this director. Can be 0xFFFF to indicate there should be an associated content ID.
        #[brw(pad_after = 2)] // padding
        content_id: u16,
        /// If set to 1, enables explorer mode. Probably does a lot more too!
        flags: u32,
    },

    #[brw(magic = 101u32)]
    TerminateDirector {
        /// The director to terminate.
        handler_id: HandlerId,
    },

    /// Updates the invisibility flags for an actor.
    #[brw(magic = 106u32)]
    SetInvisibilityFlags { flags: InvisibilityFlags },

    #[brw(magic = 109u32)]
    DirectorEvent {
        handler_id: HandlerId,
        event: DirectorEvent,
        arg: u32,
        unk1: u32, // not sure the meaning of this one, maybe just an extra arg?
    },

    #[brw(magic = 131u32)]
    UnlockInstanceContent {
        /// Index into InstanceContent Excel sheet
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    /// Calls into some GCSupply function, unsure what it does yet.
    #[brw(magic = 137u32)]
    UnkGCSupplyRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Unknown purpose, maybe positioning related?
    #[brw(magic = 138u32)]
    DisableEventPosRollback { handler_id: HandlerId },

    #[brw(magic = 142u32)]
    SetPvpMoveMode { unk1: u32 },

    #[brw(magic = 142u32)]
    SetImmediateAction { unk1: u32 },

    #[brw(magic = 156u32)]
    ToggleAdventureUnlock {
        id: u32, // Index to Adventure sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        all_vistas_recorded: bool,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 164u32)]
    ToggleAetherCurrentUnlock {
        id: u32, // Index to AetherCurrent sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        attunement_complete: bool, // If true, then "Attunement Complete" will show in the Aether Currents menu, and screen_image_id will show on screen
        // padding, screen_image_id and zone_id are technically a single u32 in the client, but this is more readable
        padding: u8,
        screen_image_id: u16, // Index to ScreenImage sheet. Will only show if attunement_complete is true.
        zone_id: u8,          // Index to AetherCurrentCompFlgSet sheet.
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

    // Calls into some EventFramework function, haven't looked too far yet.
    #[brw(magic = 199u32)]
    UnkEventRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
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

    /// Used for things like the water pads in Gold Saucer.
    #[brw(magic = 220u32)]
    ExecuteGimmickJump {
        /// Y position to land on.
        #[br(map = read_packed_float)]
        #[bw(map = write_packed_float)]
        landing_position_y: f32,
        /// X position to land on.
        #[br(map = read_packed_float)]
        #[bw(map = write_packed_float)]
        landing_position_x: f32,
        /// Z position to land on.
        #[br(map = read_packed_float)]
        #[bw(map = write_packed_float)]
        #[brw(pad_after = 2)] // empty
        landing_position_z: f32,
        /// Index into the GimmickJump Excel sheet.
        gimmick_jump_type: u32,
        unk1: u32,
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
    SetPetParameters {
        /// If set to 0, hides the pet hotbar.
        pet_id: u32,
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
    ToggleMinionUnlock {
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
    SetPetEntityId { unk1: u32 },

    #[brw(magic = 290u32)]
    Emote(CommonEmoteInfo),

    /// Generic catch-all for crafting/gathering actions.
    #[brw(magic = 300u32)]
    LiveEvent {
        #[brw(pad_size_to = 16)] // pad for LiveEvents that don't use all params
        event: LiveEventType,
    },

    #[brw(magic = 324u32)]
    SetCaughtFishBitmask { index: u32, value: u32 },

    #[brw(magic = 343u32)]
    SetCaughtSpearfishBitmask { index: u32, value: u32 },

    #[brw(magic = 378u32)]
    PlayerCurrency {
        unk1: u32,
        /// Index into the Item Excel sheet.
        catalog_id: u32,
        /// Max number of held currency.
        max_count: u32,
    },

    #[brw(magic = 384u32)]
    SetupGatheringPoint {
        /// Index into the GatheringPoint Excel sheet.
        id: u32,
        /// Index into the GatheringPointBase Excel sheet.
        base_id: u32,
        /// The level of this gathering node. This can be read from the GatheringPointBase Excel sheet.
        level: u32,
        /// Count column from the GatheringPoint Excel sheet.
        count: u32,
    },

    /// Plays an animation for a NPC or player.
    #[brw(magic = 407u32)]
    PlayActionTimeline {
        /// See the ActionTimeline Excel sheet.
        timeline_id: u32,
    },

    /// Plays an animation for a SharedGroup object.
    #[brw(magic = 410u32)]
    PlaySharedGroupTimeline { timeline_id: u32 },

    #[brw(magic = 500u32)]
    SetTitle { title_id: u32 },

    #[brw(magic = 501u32)]
    UnlockTitle { title_id: u32 },

    /// Sets or removes a marker on a given target (Ignore Target, Bind Target, etc.).
    #[brw(magic = 502u32)]
    ToggleSign {
        /// The ID of the sign to apply.
        sign_id: u32,
        /// The actor id of the player who marked the target.
        #[brw(pad_after = 12)] // Empty/zeroes
        from_actor_id: ObjectId,
        /// The actor to apply the sign to.
        target_actor_id: ObjectId,
        /// Repeated back to the client. See the corresponding ClientTrigger for more info.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        on: bool,
    },

    #[brw(magic = 504u32)]
    SetStatusIcon { icon: OnlineStatus },

    #[brw(magic = 504u32)]
    SetLimitBreak {
        level: u32,
        amount: u32,
        limit_type: u32,
    },

    /// Controls *something* about limit breaks.
    #[brw(magic = 505u32)]
    UnkLimitBreakController {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 507u32)]
    SetHomepoint { id: u32 },

    #[brw(magic = 509u32)]
    LearnTeleport {
        id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 510u32)]
    ToggleChocoboTaxiStandUnlock {
        id: u32, // id + 1179648 = Index to ChocoboTaxiStand sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 511u32)]
    MapMarkerUpdateBegin { flags: u32 },

    #[brw(magic = 512u32)]
    MapMarkerUpdateEnd {},

    #[brw(magic = 516u32)]
    ToggleCutsceneSeen {
        id: u32, // Index to Cutscene sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    #[brw(magic = 517u32)]
    LogMessage {
        log_message: u32, // Index to LogMessage sheet
        id: u32,          // Index to variable sheet, depending on LogMessage
    },

    #[brw(magic = 519u32)]
    SetPartyMemberCutsceneFlags { unk1: u32, unk2: u32 },

    #[brw(magic = 521u32)]
    SetItemLevel { level: u32 },

    #[brw(magic = 533u32)]
    LogMessage2 {
        log_message: u32, // Index to LogMessage sheet
    },

    /// Calls into some Achievement method, unsure what this does yet.
    #[brw(magic = 538u32)]
    UnkAchievementRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Calls into some MobHunt method, not sure what it does yet.
    #[brw(magic = 583u32)]
    UnkMobHuntRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 608u32)]
    SetEquipDisplayFlags { display_flag: EquipDisplayFlag },

    #[brw(magic = 609u32)]
    ToggleWireframeRendering(),

    #[brw(magic = 801u32)]
    GearSetEquipped { gearset_index: u32 },

    /// Multiple festivals can be set at the same time.
    #[brw(magic = 902u32)]
    SetFestival {
        festival1: u32,
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

    /// Sets some variable in QuestManager, unsure which yet.
    #[brw(magic = 907u32)]
    UnkQuestManagerRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Sets some variable in GoldSaucerManager, unsure which yet.
    #[brw(magic = 911u32)]
    UnkGoldSaucerManagerRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
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

    /// Calls some method in PlayerState, unsure what it does currently.
    #[brw(magic = 924u32)]
    UnkPlayerStateRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
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

    /// Calls some method in PvPProfile, unsure what it does yet.
    #[brw(magic = 1610u32)]
    UnkPvPProfileRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[doc(hidden)]
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
    #[brw(pad_after = 4)]
    #[brw(pad_size_to = 20)] // take into account categories without params
    pub category: ActorControlCategory,
    pub target: ObjectTypeId,
}

impl Default for ActorControlTarget {
    fn default() -> Self {
        Self {
            category: ActorControlCategory::ToggleInvisibility { invisible: false },
            target: ObjectTypeId::default(),
        }
    }
}
