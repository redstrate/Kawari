use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::{
    common::{
        CharacterMode, DirectorEvent, EquipDisplayFlag, EventState, FateState, HandlerId, ObjectId,
        ObjectTypeId, SharedGroupTimelineState, read_bool_from, read_packed_float, write_bool_as,
        write_packed_float,
    },
    ipc::zone::{online_status::OnlineStatus, server::ContainerType},
};

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

    #[doc(hidden)]
    Unknown {
        event: u32,
        param1: u32,
        param2: u32,
        param3: u32,
    },
}

#[binrw]
#[derive(Debug, PartialEq, Clone, IntoStaticStr)]
pub enum ActorControlCategory {
    #[brw(magic = 0u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        /// This seems to always be set to true. If set to another value, the animation glitches for other clients.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        immediately: bool,
    },

    /// Sets auto-attack mode of this entity.
    #[brw(magic = 1u32)]
    SetAutoAttack {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        auto_attack: bool,
    },

    /// Sets the character mode, such as for mounts.
    #[brw(magic = 2u32)]
    SetMode {
        #[brw(pad_size_to = 4)]
        mode: CharacterMode,
        mode_arg: u32,
    },

    /// Begins an event action.
    #[brw(magic = 3u32)]
    EventAction {
        /// Seems to be 1 to start it?
        unk1: u32,
        /// Index into the EventAction Excel sheet.
        id: u32,
    },

    /// Toggles whether an enemy has a red nameplate or not.
    #[brw(magic = 4u32)]
    SetBattle {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        battle: bool,
    },

    /// Only plays the VFX and nothing else.
    #[brw(magic = 5u32)]
    ClassJobChangeVFX {
        /// Index into the ClassJob Excel sheet.
        classjob_id: u32,
    },

    /// Shows that this enemy was defeated in the log, and probably other stuff.
    #[brw(magic = 6u32)]
    Defeated { id1: ObjectId, id2: ObjectId },

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
    CancelCast {
        /// This corresponds to row id from the LogMessage sheet.
        log_message_id: u32,
        action_type: u32,
        action_id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        interrupted: bool,
    },

    /// Sets the current value of the cooldown timer.
    #[brw(magic = 16u32)]
    SetCooldownTimer {
        /// This corresponds to (CooldownGroup - 1) from the Action sheet.
        cooldown_group: u32,
        elapsed_centisec: u32,
        total_centisec: u32,
    },

    /// Sets the upper bound of the cooldown timer. *Only* has an effect if the cooldown timer is actually running.
    #[brw(magic = 17u32)]
    SetCooldownTimerMax {
        /// This corresponds to (CooldownGroup - 1) from the Action sheet.
        cooldown_group: u32,
        /// Index in the Action Excel sheet.
        action_id: u32,
        /// Cooldown duration in centiseconds (10ms units). This is derived from Action.Recast100ms
        /// after speed scaling and per-group adjustments, not sent in the sheet's raw 100ms unit.
        duration_centisec: u32,
    },

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

    #[brw(magic = 35u32)]
    Tether {
        /// This corresponds to row id from the Channeling sheet.
        tether_id: u32,
        from_actor_id: u32,
        to_actor_id: u32,
        /// Tether progress (in pct), not sure what does it controls
        progress: u32,
    },

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

    #[brw(magic = 47u32)]
    TetherCancel {
        /// This corresponds to row id from the Channeling sheet.
        tether_id: u32,
        target_actor_id: u32,
    },

    #[brw(magic = 50u32)]
    SetTarget {},

    #[brw(magic = 58u32)]
    SetSoftTarget {},

    /// Plays this character's idle animation, I guess?
    #[brw(magic = 60u32)]
    PlayIdleAnimation {},

    /// Toggles whether this actor can be targeted. Retail uses this to make a boss untargetable
    /// while it's off doing a mechanic (e.g. Ifrit jumping away for Crimson Cyclone).
    #[brw(magic = 54u32)]
    Targetable {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        targetable: bool,
    },

    /// Shows or hides the actor (controls its visibility). `visible` = 1 to show, 0 to hide. The
    /// optional `duration` is a raw float controlling the fade animation length — the client divides
    /// `1.0` by it; pass `0.0` for the default/instant.
    #[brw(magic = 414u32)]
    ToggleVisibility {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        visible: bool,
        duration: f32,
    },

    /// Immediately force-refresh the actor's state (pos, rotation etc.).
    #[brw(magic = 415u32)]
    ForceStateRefresh {},

    // Sets the player's HP and seems to deal unique damage(?) Seen while falling off an Eden arena.
    #[brw(magic = 80u32)]
    DamageEffect {
        /// How much damage you take.
        amount: u32,
    },

    #[brw(magic = 98u32)]
    SetName {
        /// This corresponds to row id from the BNpcName sheet.
        name_row_id: u32,
    },

    // Calls into many inventory-related functions, haven't looked too far yet.
    #[brw(magic = 84u32)]
    UnkInventoryRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Set the actor into a state of riding pillion.
    #[brw(magic = 89u32)]
    RidePillion {
        /// The target actor to bind to.
        target_actor_id: ObjectId,
        /// The target seat to ride on the mount.
        target_seat_index: u32,
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

    /// Updates the state for an event object.
    #[brw(magic = 106u32)]
    SetEventState { state: EventState },

    #[brw(magic = 109u32)]
    DirectorEvent {
        handler_id: HandlerId,
        event: DirectorEvent,
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

    /// If enabled, the client sends UpdatePositionHandlerInstance. When disabled (the default) then regular UpdatePositionHandler packets instead.
    #[brw(magic = 142u32)]
    EnableInstancePositionHandler {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        enabled: bool,
    },

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

    /// The player is offered a teleport by someone in their party.
    #[brw(magic = 204u32)]
    TeleportOffered {
        /// If the player is ineligible for this teleport, it'll be set to true.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        ineligible_for_teleport: bool,
        /// The destination aetheryte.
        aetheryte_id: u32,
        /// The party member who offered the teleport. It only affects the name of the offerer, getting it wrong shows either the wrong name or no name at all.
        party_member_index: u32,
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

    /// Apply knockback to player
    #[brw(magic = 225u32)]
    Knockback {
        // Knockback direction, likely `radians * 10000 with PI offset`.
        direction: u32,
        // Knockback distance, likely `distance * 10000`.
        distance: u32,
        // lerp speed?, likely scaled by 100.
        speed: u32,
    },

    // TODO: This may need special handling since they are u32 bruh
    // Coords need to do a `var * (2000d / 65535) - 1000` on client-side, we need to do it reverse (don't know why it converted to ushort in Bossmod)
    // Rotations need to do a `var * (1d / 65535 * 2 * PI) - PI` and convert it to radians on client-side, we need to do it reverse
    /// Overriding player's movement (ActorControlSelf)
    #[brw(magic = 226u32)]
    ForcedMovement {
        coords_x: u32,
        coords_y: u32,
        coords_z: u32,
        rotation: u32,
        duration: u32,
        move_type: u32,
    },

    /// Changes something in UpdatePositionHandler?
    #[brw(magic = 236u32)]
    MovementRelatedUnk { unk1: u32 },

    #[brw(magic = 253u32)]
    CompanionUnlock {
        /// Is companion summoned or not
        is_summoned: u32,
        /// Observed as 1 while syncing a summoned companion.
        force_sync: u32,
        /// Remaining spawn time (in sec)
        remaining_time: u32,
        /// Current companion EXP.
        current_exp: u32,
        /// Companion skill level.
        skill_level: u32,
    },

    #[brw(magic = 254u32)]
    BuddyEquipUnlock {
        id: u32, // Index to BuddyEquip sheet
    },

    /// Also seen during Carbuncle spawning.
    #[brw(magic = 257u32)]
    SetupPet {
        /// The owner's actor ID.
        owner_id: ObjectId,
        /// Index into the Pet Excel sheet. If set to 0, hides the pet hotbar.
        pet_id: u32,
        /// The object ID of the pet.
        pet_actor_id: ObjectId,
        unk2: u32,
        unk3: u32,
    },

    /// Sets up "pets" like carbuncles.
    #[brw(magic = 260u32)]
    SetPetParameters {
        /// Index into the Pet Excel sheet. If set to 0, hides the pet hotbar.
        pet_id: u32,
        unk2: u32,
        unk3: u32,
        /// Usually 7?
        unk4: u32,
        /// Retail toggles this to 1 while the pet owner is mounted, then back to 0 after dismount.
        mount_state: u32,
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
    Emote {
        /// The id of the emote.
        emote: u32,
        /// 0/false = text shown, 1/true = text hidden
        #[brw(pad_before = 4)] // blank
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        hide_text: bool,
    },

    /// Interrupts the currently playing emote.
    #[brw(magic = 291u32)]
    InterruptEmote {},

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

    // Unsure what the purpose of this is, it's sent when someone else rides pillion on the player's mount
    #[brw(magic = 400u32)]
    PillionDriverRelatedUnk {
        target_seat_index: u32,
        from_actor_id: ObjectId,
    },

    /// Plays an animation for a NPC or player.
    #[brw(magic = 407u32)]
    PlayActionTimeline {
        /// See the ActionTimeline Excel sheet.
        timeline_id: u32,
    },

    /// Seen for giant clams.
    #[brw(magic = 409u32)]
    SetSharedGroupTimelineState {
        state: SharedGroupTimelineState,
        /// NOTE: I don't believe it's read by the client. And it looks like nonsense... maybe director id?
        unk2: u32,
        /// p3 == 1 means housing (?) item instead of event obj
        unk3: u32,
        /// housing item id
        unk4: u32,
    },

    /// Plays an animation for a SharedGroup object.
    #[brw(magic = 410u32)]
    PlaySharedGroupTimeline { timeline_id: u32 },

    #[brw(magic = 413u32)]
    EObjAnimation { param1: u32, param2: u32 },

    /// Sets the `companion_owner_id` for a this object.
    #[brw(magic = 417u32)]
    SetCompanionOwnerId { new_id: ObjectId },

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
        from_actor_id: ObjectId,
    },

    #[brw(magic = 504u32)]
    SetStatusIcon { icon: OnlineStatus },

    // adds 75 per 3 seconds (100 per 3 seconds in pvp mode) when in combat
    /// LimitBreakGauge (ActorControlSelf)
    #[brw(magic = 505u32)]
    LimitBreakGauge {
        /// Controlling how many bars for the LB gauge, set to 0 hides it (happened when exiting duty)
        bars: u32,
        /// Current value of the gauge, should be `bars * max_value_per_bar`
        current_value: u32,
        /// Usually 10000 in duty, 3000 in pvp from retail
        max_value_per_bar: u32,
        /// Seems ununsed, always 0 so far
        unk4: u32,
        /// 0 - Normal LB
        /// 1 - PvP LB
        /// 3 - Normal LB but NPC can consume it? (if is the client doing NPC classjob rotation)
        lb_type: u32,
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
        param1: u32,
        param2: u32,
        param3: u32,
        param4: u32,
        param5: u32,
    },

    #[brw(magic = 519u32)]
    SetPartyMemberCutsceneFlags { actor_id: ObjectId, unk2: u32 },

    #[brw(magic = 521u32)]
    SetItemLevel { level: u32 },

    #[brw(magic = 533u32)]
    LogMessage2 {
        log_message: u32, // Index to LogMessage sheet
    },

    /// Tells the client to re-evaluate its gearset list against the current equipment (e.g. to
    /// light up the "Update Gearset" button after equipment/glamour changed). Takes no params —
    /// the client handler just refreshes the relevant UI. Seen after applying a glamour plate.
    #[brw(magic = 804u32)]
    GearSetRefresh {},

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

    #[brw(magic = 600u32)]
    AchievementProgress {
        id: u32,
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 608u32)]
    SetEquipDisplayFlags { display_flag: EquipDisplayFlag },

    #[brw(magic = 609u32)]
    ToggleWireframeRendering(),

    /// ActionRejected (ActorControlSelf)
    #[brw(magic = 700u32)]
    ActionRejected {
        /// This corresponds to row id from the LogMessage sheet.
        log_message_id: u32,
        action_type: u32,
        action_id: u32,
        recast_elapsed_centisec: u32,
        recast_total_centisec: u32,
        source_seq: u32,
    },

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

    /// For the Gold Saucer.
    #[brw(magic = 911u32)]
    UnkGoldSaucerRelated { unk1: u32 },

    /// For the Gold Saucer.
    #[brw(magic = 912u32)]
    SetWeeklyLotOffsetTime { offset_time: u32 },

    /// For the Gold Saucer.
    #[brw(magic = 914u32)]
    SetGoldSaucerFlags { flags: u32 },

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

    /// ServerRequestCallbackResponse (ActorControlSelf)
    #[brw(magic = 925u32)]
    ServerRequestCallbackResponse {
        listener_index: u32,
        listener_req_type: u32,
        data1: u32,
        data2: u32,
        data3: u32,
        data4: u32,
    },

    /// Unsure what this is for, but it's sent when riding as a passenger.
    #[brw(magic = 928u32)]
    PillionPassengerRelatedUnk { unk: u32 },

    // TODO: rename this again if it's apparent that it's not purely for the dismount animation, but it's what actually plays the animation when networked...
    // TODO: What do those 3 unks represent? They don't seem to matter to the client (dismounting still works even if they're all zero).
    #[brw(magic = 930u32)]
    PlayDismountAnimation { unk1: u32, unk2: u32, unk3: u32 },

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

    /// The server acknowledges the client's request to remove an item from the world and put it back into their inventory. Doesn't seem to have any values, but we should keep unknowns for now, just in case...
    #[brw(magic = 1009u32)]
    FurnitureRemovedToInventoryAck {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// The server acknowledges the client's request to place an item. Doesn't seem to have any values, but we should keep unknowns for now, just in case...
    #[brw(magic = 1011u32)]
    FurniturePlacedAck {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// The server displays the Interior Furnishings menu for the client.
    #[brw(magic = 1015u32)]
    FurnitureMenu(),

    /// The server acknowledges the client's request to translate (move or rotate) an item. The purpose of sending the item's container is unclear.
    #[brw(magic = 1017u32)]
    FurnitureTranslatedAck {
        /// What the furniture was stored in.
        storage_id: ContainerType,
        /// Seems to be the plot number of the interior being modified, or zero if in an apartment. Unknown purpose.
        plot_number: u16,
        /// The slot the furniture occupied.
        slot: u32,
    },

    /// The server sets the interior lighting level for an observing client.
    #[brw(magic = 1034u32)]
    InteriorLightLevelForObserver {
        /// The light level set by the resident.
        level: u32,
        /// Same unk from the resident's client trigger. It's always 1.
        unk1: u32,
    },

    /// The server sets the interior lighting level for the client.
    #[brw(magic = 1035u32)]
    InteriorLightLevel {
        unk1: u32,
        /// The light level set by the client.
        level: u32,
        /// This repeats the client trigger's unk back to them. It's always 1.
        unk2: u32,
    },

    #[brw(magic = 1204u32)]
    ToggleTripleTriadCardUnlock {
        id: u32, // Index to TripleTriadCard sheet
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        unlocked: bool,
    },

    /// Called during dueling.
    #[brw(magic = 1504u32)]
    SetPvPState {
        state: u32, // TODO: turn into enum
    },

    /// Called during dueling.
    #[brw(magic = 1506u32)]
    StartDuelCountdown { opponent_id: ObjectId },

    #[brw(magic = 1512u32)]
    SetDutyActionSet {
        /// Should be correspond to row id of the ContentExAction sheet?
        row_id: u32,
    },

    /// SetDutyActionDetails (ActorControlSelf)
    /// cur_charges seems like usually sets to 0, then follow by a SetDutyActionCharges
    #[brw(magic = 1513u32)]
    SetDutyActionDetails {
        slot0_action_id: u32,
        slot0_max_charges: u32,
        slot1_action_id: u32,
        slot1_max_charges: u32,
        slot0_cur_charges: u32,
        slot1_cur_charges: u32,
    },

    #[brw(magic = 1514u32)]
    SetDutyActionPresent { do_present: u32 },

    #[brw(magic = 1515u32)]
    SetDutyActionActive {
        slot0_active: u32,
        slot1_active: u32,
    },

    #[brw(magic = 1516u32)]
    SetDutyActionCharges {
        slot0_cur_charges: u32,
        slot1_cur_charges: u32,
    },

    /// Calls into animation-related functions I think?
    #[brw(magic = 1529u32)]
    UnkAnimationRelated {},

    #[brw(magic = 1536u32)]
    IncrementRecast {
        cooldown_group: u32,
        delta_time_centisec: u32,
    },

    /// Floating heal number shown on screen for a heal-over-time tick. Retail sends this every
    /// 3 seconds for each active HoT (captured category 1540 — one less than the DoT's 1541).
    #[brw(magic = 1540u32)]
    TickHeal {
        /// The Status EXD row id that owns this tick.
        status_id: u32,
        /// The heal amount displayed.
        amount: u32,
        /// The actor that applied the HoT (the caster).
        source_actor_id: ObjectId,
        /// Unknown. Client static analysis shows multiple branches depending on this field; keep
        /// observed values until its meaning is better understood.
        unk2: u32,
        unk3: u32,
    },

    /// Floating damage number shown above an actor for a damage-over-time tick. Retail sends this
    /// every 3 seconds for each active DoT (captured category 1541), which is why DoT ticks render
    /// in the distinct "tick" number style rather than the regular action-hit style.
    #[brw(magic = 1541u32)]
    TickDamage {
        /// The Status EXD row id that owns this tick.
        status_id: u32,
        /// The damage amount displayed.
        amount: u32,
        /// The actor that applied the DoT (the caster).
        source_actor_id: ObjectId,
        /// Unknown. Client static analysis shows multiple branches depending on this field; keep
        /// observed values until its meaning is better understood.
        unk2: u32,
        unk3: u32,
    },

    #[brw(magic = 1545u32)]
    UnkCooldownsRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Calls some method in PvPProfile, unsure what it does yet.
    #[brw(magic = 1610u32)]
    UnkPvPProfileRelated {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Writes a single glamour-dresser/preview entry into the client's MirageManager. The client
    /// stores it into three parallel 800-element arrays keyed by `index` (item ids, stain0, stain1).
    /// Sent while putting items into the dresser and while previewing plate edits.
    #[brw(magic = 1800u32)]
    UpdateGlamourItemInfoAtIndex {
        /// Slot index into the MirageManager arrays. Ignored client-side if >= 800.
        index: u32,
        /// The catalog item id (HQ items high-bit encoded).
        item_id: u32,
        /// First dye/stain. Only the low byte is used client-side.
        stain0: u32,
        /// Second dye/stain. Only the low byte is used client-side.
        stain1: u32,
        /// When 1, resets an internal MirageManager state byte (batch/refresh marker).
        flag: u32,
    },

    /// Finalizes a staged glamour operation on the client: clears the pending MirageManager flag
    /// and, when requested, plays the "glamour applied" effect. These are param1/param2 of the
    /// ActorControl (the case only reads the first two params).
    #[brw(magic = 1801u32)]
    CommitGlamourOperation {
        /// param1 — when non-zero, plays the glamour-completion effect (fires UI event 580).
        play_vfx: u32,
        /// param2 — when non-zero, the effect targets an alternate actor resolved from a global
        /// table instead of self.
        alt_target: u32,
    },

    /// Collection UI stuff.
    #[brw(magic = 2251u32)]
    McGuffinUnk {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2351u32)]
    FateNpc {
        actor_id: ObjectId,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2352u32)]
    UnkFate2 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2353u32)]
    FateInit {
        /// Index into the FATE Excel sheet.
        fate_id: u32,
        /// What state this FATE is in.
        fate_state: FateState,
    },

    #[brw(magic = 2354u32)]
    UnkFate4 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2355u32)]
    UnkFate5 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Creates the FateContext on the client.
    #[brw(magic = 2356u32)]
    CreateFateContext {
        fate_id: u32,
        /// Controls the IsBonus field in some way.
        is_bonus: u32,
    },

    #[brw(magic = 2359u32)]
    SetupMotivationNpc {
        /// Index into the FATE Excel sheet.
        fate_id: u32,
        motivation_npc: ObjectId,
        /// Divided by 1000.0 for map coordinates.
        position_x: u32,
        position_y: u32,
        position_z: u32,
    },

    #[brw(magic = 2363u32)]
    UnkFate6 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Sets progress and other fields too. Unsure why this is used sometimes instead of FateProgress?
    #[brw(magic = 2364u32)]
    UnkFate7 {
        /// Index into the FATE Excel sheet.
        fate_id: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2365u32)]
    UnkFate8 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2366u32)]
    FateProgress {
        /// Index into the FATE Excel sheet.
        fate_id: u32,
        /// From 0-100 assumedly?
        progress: u32,
    },

    #[brw(magic = 2367u32)]
    UnkFate10 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 2369u32)]
    UnkFate11 {
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    /// Sets something for all objects bound to this FATE...
    #[brw(magic = 2370u32)]
    UnkFate12 {
        /// Index into the FATE Excel sheet.
        fate_id: u32,
    },

    #[brw(magic = 2500u32)]
    FauxHollowsData {
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
        param5: u32,
    },
}

impl Default for ActorControlCategory {
    fn default() -> Self {
        Self::Unknown {
            category: 0,
            param1: 0,
            param2: 0,
            param3: 0,
            param4: 0,
            param5: 0,
        }
    }
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorControl {
    #[brw(pad_size_to = 24)] // take into account categories without params
    pub category: ActorControlCategory,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorControlSelf {
    #[brw(pad_size_to = 40)] // take into account categories without params
    pub category: ActorControlCategory,
}

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct ActorControlTarget {
    #[brw(pad_size_to = 24)] // take into account categories without params
    pub category: ActorControlCategory,
    pub target: ObjectTypeId,
}
