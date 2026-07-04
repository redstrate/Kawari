use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{
    ContainerType, DirectorTrigger, DistanceRange, HandlerId, ObjectId, ObjectTypeId,
    read_bool_from, write_bool_as,
};
use crate::ipc::zone::WaymarkPosition;
use crate::ipc::zone::client::HouseId;

/// ExecuteCommandFlag/ExecuteCommandComplexFlag values that are known by the
/// client, but not yet modeled as dedicated `ClientTriggerCommand` variants.
#[binrw]
#[repr(u32)]
#[brw(repr = u32)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, IntoStaticStr)]
pub enum KnownClientTriggerCommand {
    PublicContentCommand206 = 206,
    TeleportCommand207 = 207,
    Unk208 = 208,
    DiveThrough = 209,
    RequestFriendHousingTeleportInfo = 210,
    TeleportToFriendHouse = 211,
    Unk212 = 212,
    UnknownCommand312 = 312,
    CabinetCommand426 = 426,
    RelicSphereCommand440 = 440,
    InventoryOperationCommand449 = 449,
    EmoteLocation = 501,
    EmoteInterruptLocation = 504,
    Unk603 = 603,
    ControlCommand604 = 604,
    ControlCommand605 = 605,
    DiveEnd = 607,
    DiveInvalid = 610,
    CleanupGimmickJumpState611 = 611,
    CleanupGimmickJumpState613 = 613,
    ControlCommand614 = 614,
    ControlCommand615 = 615,
    QuestCommand704 = 704,
    SpearFishingCommand713 = 713,
    SpearFishingCommand714 = 714,
    MarkSpearFishingActionUsage = 715,
    SpearFishingCommand716 = 716,
    SpearFishingCommand717 = 717,
    SpearFishingCommand718 = 718,
    CompanyLeveQuestCommand = 805,
    SaveAnimaWeaponQuestGender = 817,
    FestivalQuestWorkCommand818 = 818,
    QuestRedoCommand821 = 821,
    QuestRedoCommand822 = 822,
    FateCommand829 = 829,
    RequestCutscene831 = 831,
    EventFrameworkCommand832 = 832,
    ActorControlCommand1003 = 1003,
    HousingCommand1116 = 1116,
    FreeCompanyHousingCommand1124 = 1124,
    HousingCommand1127 = 1127,
    RequestHousingOutdoorTerritory = 1128,
    GMCommand1129 = 1129,
    GMCommand1130 = 1130,
    MannequinCommand1132 = 1132,
    HousingCommand1136 = 1136,
    AirshipCommand1139 = 1139,
    AirshipCommand1140 = 1140,
    AirshipCommand1141 = 1141,
    AirshipCommand1142 = 1142,
    HousingCommand1146 = 1146,
    HousingCommand1152 = 1152,
    RequestHousingGuestBook1154 = 1154,
    RequestHousingGuestBook1155 = 1155,
    RequestHousingGuestBook1156 = 1156,
    RequestHousingGuestBook1157 = 1157,
    RequestHousingGuestBook1158 = 1158,
    HousingCommand1159 = 1159,
    OpenHousingRetainerSalesSettingUI = 1160,
    RetainerMarketCommand1161 = 1161,
    HousingCommand1162 = 1162,
    OpenHousingRetainerBuyUI = 1163,
    UpdateHousingRetainerPose = 1164,
    SetHousingRetainerWeapon = 1165,
    ToggleHousingRetainerWeapon = 1166,
    RequestHousing = 1167,
    RequestHousingInteriorDesign = 1168,
    ChangeHousingInteriorDesign = 1169,
    HouseInteriorPatternCommand1170 = 1170,
    BgcArmyAction = 1810,
    Unk2000 = 2000,
    ReviveCommand2204 = 2204,
    ConfirmWondrousTailsSlot = 2253,
    WondrousTails = 2254,
    CliamFashionCheckEntryReward = 2450,
    ClaimFashionCheckBonusReward = 2451,
    ClaimFashionCheckNewGearReward = 2452,
    FashionCheckCommand2453 = 2453,
    RequestEnclave = 2500,
    BuybackEnclaveItem = 2501,
    RequestBlueContentBriefing = 2600,
    RequstBlueNotebook = 2601,
    SendDutySupport = 2654,
    EventFrameworkAction = 2800,
    RequestBozjaWarResultNotebook = 2900,
    AssignBozjaActionFromHolster = 2950,
    RequestBozjaHolsterOutside = 2951,
    PrepareSceneJump = 3000,
    StartSceneJumpLua = 3001,
    CaptureMJIAnimal = 3050,
    RequestItemActionUnlockState = 3100,
    GetServerValue = 3150,
    SetMJIMode = 3250,
    SetMJIModeParam = 3251,
    ToggleMJISettingPanel = 3252,
    RequestMJIWorkshop = 3254,
    RequestMJIWorkshopConsumption = 3255,
    RequestMJIWorkshopAssignment = 3258,
    AssignMJIWorkshop = 3259,
    CancelMJIWorkshopAssignment = 3260,
    SetMJIWorkshopRest = 3261,
    CollectMJIGranary = 3262,
    ViewMJIGranaryDestination = 3263,
    AssignMJIGranary = 3264,
    ReleaseMJIMinion = 3265,
    ReleaseMJIAnimal = 3268,
    CollectMJIAnimalLeaving = 3269,
    CollectMJIAllAnimalLeaving = 3271,
    EntrustMJIAnimal = 3272,
    RecallMJIMinion = 3277,
    EntrustMJIFarm = 3279,
    DismissMJIFarmEntrust = 3280,
    CollectMJIFarm = 3281,
    CollectMJIAllFarm = 3282,
    PlayOrchestrionTrack = 3283,
    RequestMJIWorkshopFavor = 3292,
    RemoveFavoriteAetheryte = 3350,
    RemoveFreeAetheryte = 3351,
    RemoveFreeAetherytePSPlus = 3352,
    RemoveFreeAetheryteNSO = 3353,
    SetWKSMode = 3400,
    FinishWKSInteraction3401 = 3401,
    FinishWKSInteraction3402 = 3402,
    WKSDevelopmentCommand = 3403,
    AcceptWKSMission = 3440,
    FinishWKSMission = 3441,
    AbandonWKSMission = 3442,
    StartWKSLottery = 3450,
    ChooseWKSLotteryType = 3451,
    FinishWKSLottery = 3452,
    RequestWKSSuccesses = 3460,
    RequestWKSMecha = 3478,
    RequestContentInventory = 3500,
    RequestMassivePCContent = 3600,
    QuestCommand4000 = 4000,
    RollDice = 9000,
    RequestYokaiWatchState = 9002,
}

#[binrw]
#[derive(Debug, PartialEq, Clone, IntoStaticStr)]
pub enum ClientTriggerCommand {
    /// The player sheathes/unsheathes their weapon.
    #[brw(magic = 1u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        immediately: bool,
    },

    /// When toggling auto-attack on and off.
    #[brw(magic = 2u32)]
    ToggleAutoAttack {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        on: bool,
        target: ObjectId,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        is_manual_toggle: bool,
    },

    /// The player looks or stops looking at an actor.
    #[brw(magic = 3u32)]
    SetTarget {
        actor_id: ObjectId,
        /// The client sends value(s) that differ from ObjectTypeKind, so we cannot use it here.
        /// It differs for minions, which the client sends as 2, while 4 is used elsewhere (see ObjectTypeKind).
        actor_type: u32,
    },

    /// The player looks or stops looking at an actor using the soft targetting system.
    #[brw(magic = 4u32)]
    SetSoftTarget {},

    #[brw(magic = 5u32)]
    SendPVPQuickChat {
        /// Correspond to row id of the QuickChat sheet.
        row_id: u32,
        param1: u32,
        param2: u32,
    },

    #[brw(magic = 11u32)]
    GMCommand11 {},

    /// The client is trying to dismount their current mount.
    #[brw(magic = 101u32)]
    Dismount { sequence: u32 },

    /// The client requests a minion to be summoned.
    #[brw(magic = 102u32)]
    SummonMinion { minion_id: u32 },

    /// The client requests a minion to be despawned.
    #[brw(magic = 103u32)]
    DespawnMinion {},

    /// When the player right-clicks their status effect to remove it.
    #[brw(magic = 104u32)]
    ManuallyRemoveEffect {
        effect_id: u32,
        effect_param: u32,
        source_actor_id: ObjectId,
    },

    #[brw(magic = 105u32)]
    CancelCast {},

    /// The client requests to ride pillion with another player.
    #[brw(magic = 106u32)]
    RidePillionRequest {
        /// The target actor to ride with.
        target_actor_id: ObjectId,
        /// The target seat to occupy.
        target_seat_index: u32,
    },

    /// The client requests to ride pillion with another player, and need server to auto-assign an empty seat.
    #[brw(magic = 107u32)]
    RidePillionRequestAuto {
        /// The target actor to ride with.
        target_actor_id: ObjectId,
    },

    #[brw(magic = 108u32)]
    LoadPartyMemberRequest {
        index: u32,
        target_actor_id: ObjectId,
    },

    #[brw(magic = 109u32)]
    WithdrawParasolForced {},

    #[brw(magic = 110u32)]
    WithdrawParasol {},

    #[brw(magic = 111u32)]
    UpdateParasolState {
        /// 0 means withdraw current parasol
        parasol_id: u32,
    },

    #[brw(magic = 112u32)]
    SetAutoUseParasol {
        /// 0 means disable auto use
        parasol_id: u32,
    },

    #[brw(magic = 200u32)]
    Revive {
        /// 5 - Accept revival; 8 - Respawn to spawn-point
        action: u32,
    },

    /// The client has loaded enough after a territory or in-zone position
    /// transport to begin fading in.
    /// This is named StartTerritoryTransport in some client-side enums, but in
    /// the initial zone-in flow it is the signal that the client is ready to spawn.
    #[brw(magic = 201u32)]
    FinishZoning {
        /// 1 - NPC Warp
        /// 3 - Transition through zone divider
        /// 4 - Normal teleportation
        /// 7 - Using return action
        /// 15 - In-town aetheryte
        /// 20 - Housing zone
        location_change_type: u32,
        /// 1 - By cutscene
        /// 2 - Back to safe-zone
        /// 25 - Transition in duty
        /// 26 - Diving
        position_change_type: u32,
    },

    /// The player selects a teleport destination.
    #[brw(magic = 202u32)]
    TeleportQuery {
        aetheryte_id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        use_aetheryte_ticket: bool,
        sub_aetheryte_id: u32,
        unk3: u32,
        unk4: u32,
    },

    /// The player answers a teleport offer sent by someone in their party.
    #[brw(magic = 203u32)]
    TeleportOfferReply {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        decline_teleport: bool,
    },

    #[brw(magic = 204u32)]
    CancelTeleport {},

    #[brw(magic = 205u32)]
    RejectRevive {},

    #[brw(magic = 213u32)]
    ReturnToSafePointIfNotLalafell {},

    #[brw(magic = 214u32)]
    ReturnIfNotLalafell {},

    /// Examine menu option on characters.
    #[brw(magic = 300u32)]
    ExamineCharacter { target_actor_id: ObjectId },

    /// The client toggles a sign for their current target.
    #[brw(magic = 301u32)]
    ToggleSign {
        #[brw(pad_after = 12)] // Empty/zeroes
        /// The id of the sign to apply. See the Marker Excel sheet.
        sign_id: u32,
        /// These two unknowns contain data but seemingly don't matter to the server. The server response doesn't repeat these at all.
        unk1: u16,
        unk2: u16,
    },

    /// The client sets a specific title.
    #[brw(magic = 302u32)]
    SetTitle { title_id: u32 },

    /// The client requests the player's unlocked titles.
    #[brw(magic = 303u32)]
    RequestTitleList {},

    /// Requests the name of a player by their content ID. Seen used for crafted items.
    #[brw(magic = 305u32)]
    RequestPlayerName {},

    /// The client shows an Active Help pop-up. This is triggered *when* it's shown, not when closed.
    #[brw(magic = 306u32)]
    ShownActiveHelp { id: u32 },

    /// The client has seen this cutscene. This is usually followed up by a ToggleCutsceneSeen ACS.
    #[brw(magic = 307u32)]
    SeenCutscene { id: u32 },

    #[brw(magic = 310u32)]
    RequestContentsNoteCategory { category_index: u32 },

    /// The client clears all waymarks.
    #[brw(magic = 313u32)]
    ClearAllWaymarks {},

    /// The client begins using the Idle Camera or Group Pose feature.
    #[brw(magic = 314u32)]
    GroupPoseOrIdleCamera { unk1: u32, unk2: u32 },

    #[brw(magic = 315u32)]
    SetBlueMageAction {
        /// 0 - Apply valid actions; 1 - Swap valid actions
        action_type: u32,
        /// Start by 0, less than 24
        slot_index: u32,
        /// Can be action id or slot index (start by 0, less than 24)
        action_id: u32,
    },

    /// The client opens the World Visit menu.
    #[brw(magic = 316u32)]
    OpenWorldVisit {},

    /// The client places a waymark.
    #[brw(magic = 317u32)]
    PlaceWaymark {
        /// The waymark's id.
        id: u32,
        /// The waymark's position in the world.
        pos: WaymarkPosition,
    },

    /// The client clears a waymark.
    #[brw(magic = 318u32)]
    ClearWaymark {
        /// The waymark's id.
        id: u32,
    },

    /// When the client requests to reset a striking dummy in the UI.
    #[brw(magic = 319u32)]
    ResetStrikingDummy {
        /// The dummy's object id.
        id: ObjectId,
    },

    #[brw(magic = 400u32)]
    SetRetainerMarketPrice { item_slot: u32, new_price: u32 },

    #[brw(magic = 401u32)]
    RequestMonsterNote {
        category_index: u32,
        level: u32,
        unk3: u32,
    },

    #[brw(magic = 402u32)]
    ClearReclaimNotification {},

    #[brw(magic = 403u32)]
    ReclaimItems {},

    #[brw(magic = 404u32)]
    RequestInventory {
        #[brw(pad_size_to = 4)]
        container_type: ContainerType,
    },

    #[brw(magic = 405u32)]
    MoveItemBetweenInventory {
        #[brw(pad_size_to = 4)]
        src_container_type: ContainerType,
        #[brw(pad_size_to = 4)]
        dst_container_type: ContainerType,
    },

    #[brw(magic = 406u32)]
    NotifyBlockedInventoryOperation {
        #[brw(pad_size_to = 4)]
        src_container_type: ContainerType,
        #[brw(pad_size_to = 4)]
        dst_container_type: ContainerType,
    },

    #[brw(magic = 407u32)]
    EnterMateriaAttachingState { item_id: u32 },

    #[brw(magic = 408u32)]
    FinishedMateriaAttaching {},

    #[brw(magic = 409u32)]
    ExitMateriaAttachingState {},

    #[brw(magic = 410u32)]
    EnterMateriaAttachRequestState { unk1: u32 },

    #[brw(magic = 411u32)]
    ExitMateriaAttachRequestState { unk1: u32, unk2: u32 },

    #[brw(magic = 412u32)]
    SendMateriaAttachRequest { actor_id: ObjectId },

    /// The client requests materia melding from another player.
    #[brw(magic = 413u32)]
    RequestMateriaMeld { actor_id: ObjectId },

    #[brw(magic = 414u32)]
    ToggleFreeCompanyCrestDecal {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        container_type: ContainerType,
        container_index: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        decal_state: bool,
    },

    #[brw(magic = 415u32)]
    ToggleFreeCompanyCrestDecalBatchEquipped {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        decal_state: bool,
    },

    #[brw(magic = 416u32)]
    ToggleFreeCompanyCrestDecalBatch {
        /// 5 - Armoury; 6 - Item
        range: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        decal_state: bool,
    },

    #[brw(magic = 418u32)]
    CancelMateriaAttachRequestForced {},

    #[brw(magic = 419u32)]
    FinishedInventoryOperation {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        container_type: ContainerType,
        container_index: u32,
    },

    /// Client will need to send MoveItemBetweenInventory first
    #[brw(magic = 420u32)]
    DepositFreeCompanyChestGil { amount: u32 },

    /// Client will need to send MoveItemBetweenInventory first
    #[brw(magic = 421u32)]
    WithdrawnFreeCompanyChestGil { amount: u32 },

    #[brw(magic = 422u32)]
    RequestFreeCompanyChestLog {},

    #[brw(magic = 423u32)]
    RequestCabinet {},

    #[brw(magic = 424u32)]
    StoreToCabinet { row_id: u32 },

    #[brw(magic = 425u32)]
    RestoreFromCabinet { row_id: u32 },

    #[brw(magic = 427u32)]
    FinishCabinetRequest {},

    #[brw(magic = 428u32)]
    AcceptMobHuntBill { index: u32, mark_id: u32 },

    #[brw(magic = 429u32)]
    AbandonMobHuntBill { index: u32, mark_id: u32 },

    #[brw(magic = 437u32)]
    ExtractMateria {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        container_type: ContainerType,
        container_index: u32,
    },

    /// The player is preparing to remove materia.
    #[brw(magic = 437u32)]
    PrepareRemoveMateria {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        dst_container_type: ContainerType,
        dst_container_index: u32,
    },

    /// The player is preparing to cast a glamour.
    #[brw(magic = 438u32)]
    PrepareCastGlamour {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        dst_container_type: ContainerType,
        dst_container_index: u32,
        #[brw(pad_size_to = 4)]
        src_container_type: ContainerType,
        src_container_index: u32,
    },

    /// The player is preparing to remove a glamour.
    #[brw(magic = 439u32)]
    PrepareRemoveGlamour {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        dst_container_type: ContainerType,
        dst_container_index: u32,
    },

    #[brw(magic = 441u32)]
    ChangeGearset {},

    #[brw(magic = 442u32)]
    RecoverBlockedItem {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        container_type: ContainerType,
        container_index: u32,
    },

    #[brw(magic = 444u32)]
    OpenChocoboSaddlebag {},

    #[brw(magic = 445u32)]
    RequestEnclaveBuyBack {},

    #[brw(magic = 446u32)]
    FinishRequestEnclaveBuyBack {},

    /// The client requests repair from another player.
    #[brw(magic = 450u32)]
    SendRepairRequest { actor_id: ObjectId },

    #[brw(magic = 451u32)]
    FinishRepairRequest { unk1: u32, unk2: u32, unk3: u32 },

    #[brw(magic = 452u32)]
    StartRepairRequest {},

    #[brw(magic = 453u32)]
    CancelRepairRequest {},

    #[brw(magic = 454u32)]
    ConfirmRepairRequest {},

    /// When equipping using the Facewear window.
    #[brw(magic = 455u32)]
    EquipGlasses { slot: u32, id: u32 },

    /// The player begins an emote.
    #[brw(magic = 500u32)]
    Emote {
        /// The id of the emote.
        emote: u32,
        /// 0/false = text shown, 1/true = text hidden
        #[brw(pad_before = 4)] // blank
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        hide_text: bool,
    },

    /// The player interrupts their normal emote.
    #[brw(magic = 502u32)]
    EmoteInterrupted {},

    /// The player interrupts their looping emote.
    #[brw(magic = 503u32)]
    LoopingEmoteInterrupted {},

    /// The player explicitly changed their pose.
    #[brw(magic = 505u32)]
    ChangePose { unk1: u32, pose: u32 },

    /// The client is entering or restoring an idle posture. This is also sent
    /// after login with the locally saved pose.
    #[brw(magic = 506u32)]
    ReapplyPose { unk1: u32, pose: u32 },

    #[brw(magic = 507u32)]
    ExitIdlePosture {},

    #[brw(magic = 602u32)]
    GimmickJumpLanded {
        /// Index into the GimmickJump Excel sheet.
        gimmick_jump_type: u32,
    },

    #[brw(magic = 606u32)]
    WalkInTriggerFinished { unk1: u32 },

    /// When the player begins swimming. Seems to have no parameters.
    #[brw(magic = 608u32)]
    BeginSwimming {},

    /// When the player stops swimming (by going back on land, mounting, etc.). Seems to have no parameters.
    #[brw(magic = 609u32)]
    EndSwimming {},

    /// When the player enters an area where mounting is prohibited in a zone that otherwise permits zoning. Commonly seen during Moonfire Faire festivals, and does not seem to have an exit counterpart.
    #[brw(magic = 612u32)]
    EnterMountingProhibitedArea { enabled: u32 },

    /// When the player starts flying on their mount.
    #[brw(magic = 616u32)]
    StartFlying {},

    /// When the player starts crafting.
    #[brw(magic = 700u32)]
    BeginCraft {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        end: bool,
        /// Index into Recipe Excel sheet.
        id: u32,
    },

    /// When the player starts fishing.
    #[brw(magic = 701u32)]
    Fishing { action: u32, param2: u32, unk3: u32 },

    #[brw(magic = 702u32)]
    RequestFishingNote {
        /// Corresponding to row id of the FishingNoteInfo
        row_id: u32,
    },

    #[brw(magic = 703u32)]
    RequestSpearfishNote {
        /// Corresponding to row id of the FishingNoteInfo
        row_id: u32,
    },

    #[brw(magic = 705u32)]
    SetLastReadQuest {
        unk1: u32,
        /// Is acually u16 here
        quest_id: u32,
    },

    /// When the client requests information about a GatheringPoint node that was spawned.
    #[brw(magic = 706u32)]
    RequestGatheringPoint {
        /// Index into the GatheringPoint Excel sheet.
        id: u32,
    },

    #[brw(magic = 708u32)]
    MarkGatherDivisionLevelRangeSeen {
        division_index: u32,
        lv_range_index: u32,
    },

    #[brw(magic = 711u32)]
    MarkCraftDivisionLevelRangeSeen {
        division_index: u32,
        lv_range_index: u32,
    },

    #[brw(magic = 712u32)]
    LeaveQuickSynthesis {},

    #[brw(magic = 800u32)]
    AbandonQuest { quest_id: u32 },

    #[brw(magic = 801u32)]
    RefreshLeveQuest {},

    #[brw(magic = 802u32)]
    AbandonLeveQuest { leve_quest_id: u32 },

    #[brw(magic = 803u32)]
    MarkLeveCanAccept { leve_quest_id: u32 },

    #[brw(magic = 804u32)]
    StartLeveQuest {
        leve_quest_id: u32,
        raise_lv_by: u32,
    },

    /// Various triggers related to instanced content.
    #[brw(magic = 808u32)]
    DirectorTrigger {
        handler_id: HandlerId,
        trigger: DirectorTrigger,
    },

    #[brw(magic = 809u32)]
    StartFate { fate_id: u32, actor_id: ObjectId },

    #[brw(magic = 810u32)]
    LoadFate { fate_id: u32 },

    #[brw(magic = 812u32)]
    EnterFate { fate_id: u32 },

    #[brw(magic = 813u32)]
    SyncToFateLevel {
        fate_id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        state: bool,
    },

    #[brw(magic = 814u32)]
    LoadFateMob { actor_id: ObjectId },

    /// When a player requests an NPC (or player?) to a Triple Triad match.
    #[brw(magic = 815u32)]
    TripleTriadChallenge {
        actor_id: ObjectId, // probably
    },

    #[brw(magic = 816u32)]
    FinishTerritoryTransport {},

    /// When a player requests to abandon their instanced content.
    #[brw(magic = 819u32)]
    AbandonContent {
        /// 0 for normal abandon, 1 for timed out
        is_timeout: u32,
    },

    #[brw(magic = 820u32)]
    SyncTimezoneOffset {
        utc_offset_min: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
    },

    #[brw(magic = 823u32)]
    StartSoloQuestBattle {
        /// 0 - Normal; 1 - Easy; 2 - Very easy
        difficulty: u32,
    },

    #[brw(magic = 824u32)]
    QuestRedo {
        /// 0 for quitting
        quest_id: u32,
    },

    #[brw(magic = 825u32)]
    ContinueQuestRedo {},

    #[brw(magic = 826u32)]
    DeleteQuestRedoSave {},

    #[brw(magic = 827u32)]
    ResetQuestRedoUI {},

    #[brw(magic = 828u32)]
    SyncToFateLevelAuto {
        fate_id: u32,
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        state: bool,
    },

    /// The client requests an inventory refresh.
    #[brw(magic = 830u32)]
    RefreshInventory {},

    #[brw(magic = 833u32)]
    MarkEventTutorialSeen {
        unk1: u32,
        /// Corresponds to row id of EventTutorial sheet.
        row_id: u32,
    },

    #[brw(magic = 1000u32)]
    RequestAchievement {
        /// Corresponds to row id of Achievement sheet.
        row_id: u32,
    },

    #[brw(magic = 1001u32)]
    RequestCompletedAchievement {},

    #[brw(magic = 1002u32)]
    RequestNearlyCompletedAchievement {
        /// Seems like always be 1
        unk1: u32,
    },

    /// When a player opens the Shared FATE window.
    #[brw(magic = 1009u32)]
    OpenSharedFATEWindow {
        /// The page index, starting at 0.
        page: u32,
    },

    #[brw(magic = 1010u32)]
    RequestAllAchievements {},

    #[brw(magic = 1011u32)]
    RequestAllAchievementsSpecial {
        /// Corresponds to row id of Achievement sheet.
        row_id: u32,
    },

    #[brw(magic = 1100u32)]
    BuildHouseOnPlot { ward_index: u32 },

    #[brw(magic = 1101u32)]
    EnterExteriorFixturesState { ward_index: u32 },

    #[brw(magic = 1102u32)]
    EnterInteriorFixturesState { ward_index: u32 },

    #[brw(magic = 1103u32)]
    RemoveHouseFromPlot { ward_index: u32 },

    #[brw(magic = 1104u32)]
    RequestHousingArea {
        /// Seems like always be 255
        unk1: u32,
    },

    #[brw(magic = 1105u32)]
    RequestHousingLotteryInfo {
        zone_id: u32,
        /// ward_index * 256 + plot_index
        ward_plot_index: u32,
    },

    #[brw(magic = 1106u32)]
    RequestHousingPlacard {
        zone_id: u32,
        /// ward_index * 256 + plot_index
        ward_plot_index: u32,
    },

    #[brw(magic = 1107u32)]
    RequestHousingWardInfo {
        /// The zone id of the housing ward.
        zone_id: u32,
        /// 0-based index, so ward number - 1.
        ward_index: u32,
    },

    #[brw(magic = 1108u32)]
    LoadExteriorAppearanceInventory {},

    #[brw(magic = 1109u32)]
    LoadInteriorAppearanceInventory {},

    #[brw(magic = 1110u32)]
    LoadExteriorFurnishInventory {},

    #[brw(magic = 1111u32)]
    LoadInteriorFurnishInventory {},

    // TODO: Need more RE work
    #[brw(magic = 1112u32)]
    MoveHousingItemToStoreRoom {
        /// The house's id.
        house_id: HouseId,
        #[brw(pad_size_to = 4)] // ContainerType is u16
        src_storage_type: ContainerType,
        src_storage_slot: u32,
    },

    /// The client removes a piece of furniture from the world and puts it in their inventory or the storeroom.
    // TODO: Research is still ongoing for this one
    #[brw(magic = 1113u32)]
    MoveHousingItemToInventory {
        /// The house's id.
        house_id: HouseId,
        /// The source container to move the item from.
        storage_id: ContainerType,
        unk1: [u8; 2], // likely padding
        /// The slot that contains the desired item.
        slot: u16,
        /// If the item should be moved to the storeroom or not.
        #[br(map = read_bool_from::<u16>)]
        #[bw(map = write_bool_as::<u16>)]
        to_storeroom: bool, // TODO: This might actually just be a u8
    },

    #[brw(magic = 1114u32)]
    RequestHousingName {
        /// The house's id.
        house_id: HouseId,
    },

    #[brw(magic = 1115u32)]
    RequestHousingGreeting {
        /// The house's id.
        house_id: HouseId,
    },

    #[brw(magic = 1117u32)]
    RequestHousingGuestAccess {
        /// The house's id.
        house_id: HouseId,
    },

    #[brw(magic = 1118u32)]
    SetHousingGuestAccess {
        /// The house's id.
        house_id: HouseId,
        /// Known for now: 1 - Teleport permission; 65536 - Enter permission
        flags: u32,
    },

    #[brw(magic = 1119u32)]
    RequestHousingEstateTag { house_id: HouseId },

    #[brw(magic = 1120u32)]
    SetHousingEstateTag { house_id: HouseId, flags: u32 },

    /// The client requests the housing inventory be sent to them. This happens automatically after opening the Interior Furnishings menu.
    #[brw(magic = 1121u32)]
    RequestHousingInventory {
        /// Which housing inventory to access. If true, the client wants the storeroom's inventory, otherwise, the placed furniture.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        storeroom: bool,
    },

    /// The client requests to warp to the housing interior's front door.
    #[brw(magic = 1122u32)]
    HousingMoveToFrontDoor {},

    /// The client opens or closes the Interior Furnishings menu.
    #[brw(magic = 1123u32)]
    FurnitureMenuToggled {
        /// If the menu was closed or not.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        closed: bool,
    },

    /// The client has requested an apartment building's residents list from either an apartment building entrance or an apartment's front door.
    #[brw(magic = 1125u32)]
    RequestApartmentList {
        /// The desired starting index of apartments.
        starting_index: u32,
    },

    #[brw(magic = 1126u32)]
    ViewHouseDetail {
        zone_id: u32,
        ward_plot_index: u32,
        apartment_index: u32,
    },

    #[brw(magic = 1133u32)]
    RemoveFreeCompanyHouse {},

    #[brw(magic = 1134u32)]
    RequestHousingRetainerList {},

    #[brw(magic = 1135u32)]
    RequestHousingShareHolders { req_type: u32 },

    /// The client sets the interior lighting level.
    #[brw(magic = 1137u32)]
    SetInteriorLightLevel {
        /// See HousingInteriorDetails in housing_interior_furniture.rs for further details, but `level` is actually a level of *darkness*, not light, so this CT is a misnomer, but it's more intuitive to just call it a light level...
        light_level: u32,
        ssao_state: u32, // Seems to be always 1
    },

    #[brw(magic = 1138u32)]
    RequestAirship {},

    #[brw(magic = 1143u32)]
    RequestFCProject {},

    #[brw(magic = 1144u32)]
    RequestSubmarine {},

    #[brw(magic = 1145u32)]
    SetHouseBackgroundMusic {
        /// Corresponds to row id of Orchestrion sheet.
        row_id: u32,
    },

    #[brw(magic = 1147u32)]
    SetOrchestrionPlaylist { playlist_id: u32 },

    #[brw(magic = 1148u32)]
    ToggleOrchestrion {},

    #[brw(magic = 1149u32)]
    PlayNextOrchestrionTrack {},

    /// The client places furniture from the storeroom.
    #[brw(magic = 1150u32)]
    PlaceFurnitureFromStoreroom {
        house_id: HouseId,
        /// The source container to retrieve the item from.
        #[brw(pad_size_to = 4)] // ContainerType is u16
        src_container_type: ContainerType,
        /// The index into the container.
        #[brw(pad_after = 2)]
        src_container_slot: u16,
    },

    #[brw(magic = 1151u32)]
    RequestHousingStoreroom { house_id: HouseId },

    #[brw(magic = 1153u32)]
    RepairSubmarinePart {},

    #[brw(magic = 1200u32)]
    CollectTrophyCrystal {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        last_season: bool,
    },

    #[brw(magic = 1201u32)]
    SelectPVPRoleAction { job_action_index: u32 },

    #[brw(magic = 1301u32)]
    RequestContentsNote {},

    #[brw(magic = 1400u32)]
    RequestRetainerVentureTime {},

    /// The client requests to repair an item at a mender.
    #[brw(magic = 1600u32)]
    RepairItem {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        src_container_type: ContainerType,
        src_container_slot: u32,
        item_id: u32,
    },

    /// The client requests to repair all items at a mender.
    #[brw(magic = 1601u32)]
    RepairAllItems {
        /// 0 - Main hand / Off hand
        /// 1 - Head / Body / Arms
        /// 2 - Leg / Feet
        /// 3 - Earring / Necklace
        /// 4 - Wrist / Ring
        /// 5 - Item
        category: u32,
    },

    #[brw(magic = 1602u32)]
    RepairEquippedItems {
        #[brw(pad_size_to = 4)] // ContainerType is u16
        src_storage_type: ContainerType,
    },

    #[brw(magic = 1700u32)]
    SetBuddyAction {
        /// Corresponds to row id of BuddyAction sheet.
        action_row_id: u32,
    },

    #[brw(magic = 1701u32)]
    SetBuddyEquip {
        /// 0 - Head
        /// 1 - Body
        /// 2 - Leg
        body_parts: u32,
        /// Corresponds to row id of BuddyEquip sheet, 0 to take it off.
        equipment_id: u32,
    },

    #[brw(magic = 1702u32)]
    LearnBuddySkill {
        /// Corresponds to row id of BuddySkill sheet.
        skill_row_id: u32,
    },

    /// The client is performing a pet action.
    #[brw(magic = 1800u32)]
    PetAction {
        /// Index into the PetAction Excel sheet.
        action_id: u32,
    },

    /// The client opens the General tab in the Gold Saucer window.
    #[brw(magic = 1850u32)]
    OpenGoldSaucerGeneralTab {},

    #[brw(magic = 1900u32)]
    RequestGoldSaucerChocoboInfo {},

    /// The client challengers another player to a normal match.
    #[brw(magic = 1950u32)]
    ChallengeNormalMatch { unk1: u32, unk2: u32 },

    #[brw(magic = 1980u32)]
    BeginContentsReplay {},

    #[brw(magic = 1981u32)]
    EndContentsReplay {},

    #[brw(magic = 2010u32)]
    RequestGoldSaucerVerminion {},

    #[brw(magic = 2011u32)]
    ConfirmVerminionPalette {},

    /// Sent whenever the client tries to begin a Hall of the Novice exercise.
    #[brw(magic = 2050u32)]
    BeginNoviceExercise {
        id: u32, // not specific to a class/job
    },

    /// Sent whenever the client uses the /nastatus command.
    #[brw(magic = 2100u32)]
    ToggleNoviceStatus {},

    #[brw(magic = 2101u32)]
    SetNoviceState {},

    #[brw(magic = 2102u32)]
    SetAutoJoinNoviceNetworkMentor {},

    #[brw(magic = 2103u32)]
    AcceptNoviceNetworkInvitation {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        rejected: bool,
    },

    #[brw(magic = 2104u32)]
    DismissReturnerState {},

    #[brw(magic = 2106u32)]
    RefreshNoviceNetwork {},

    #[brw(magic = 2107u32)]
    JoinNoviceNetworkReturner {},

    /// Sent whenever the client requests to duel another player.
    #[brw(magic = 2200u32)]
    RequestDuel {
        /// Whom to duel with.
        actor_id: ObjectId,
    },

    /// Sent whenever the client presses the buttons on the duel dialog.
    #[brw(magic = 2201u32)]
    RequestDuelResponse {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        cancel: bool,
    },

    /// Sent when the duel opponent accepts or rejects the challenge.
    #[brw(magic = 2202u32)]
    DuelDecision {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        decline: bool,
    },

    #[brw(magic = 2300u32)]
    RequestENPC { enpc_data_id: u32 },

    #[brw(magic = 2350u32)]
    RequestPrismBox {},

    #[brw(magic = 2352u32)]
    RestoreItemFromPrsimBox { item_id: u32 },

    #[brw(magic = 2353u32)]
    RestoreItemSetFromPrsimBox {
        prism_box_index: u32,
        item_maskings: u64,
    },

    #[brw(magic = 2355u32)]
    RequestGlamourPlate {},

    /// Sent whenever the Glamour Plates window is opened or closed.
    #[brw(magic = 2356u32)]
    ToggleGlamourPlatesWindow {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        open: bool,

        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        open_dispell: bool,
    },

    #[brw(magic = 2357u32)]
    ApplyGlamourPlate {},

    #[brw(magic = 2358u32)]
    ApplyGlamourPlateFromPrismBox { plate_index: u32 },

    #[brw(magic = 2359u32)]
    DispellGlamours { item_maskings: u32 },

    /// The client opens the Mahjong tab in the Gold Saucer window.
    #[brw(magic = 2550u32)]
    OpenGoldSaucerMahjongTab {},

    /// The client opens the Trust window.
    #[brw(magic = 2651u32)]
    OpenTrustWindow {},

    /// The client opens the Duty Support window.
    #[brw(magic = 2653u32)]
    OpenDutySupportWindow {},

    /// The client opens the Portrait window.
    #[brw(magic = 3200u32)]
    OpenPortraitsWindow {},

    /// The client is ready to begin loading a zone.
    #[brw(magic = 3201u32)]
    BeginLoading {},

    /// The client opens the Mogpendium or interacts with the Retainer Bell.
    #[brw(magic = 9003u32)]
    OpenUnk1 { unk1: u32, unk2: u32 },

    /// The client tells us how far in the distance we should see actors.
    #[brw(magic = 9005u32)]
    SetDistanceRange { range: DistanceRange },

    /// Known execute command that has not been modeled yet.
    Known {
        command: KnownClientTriggerCommand,
        params: [u32; 5],
    },

    #[doc(hidden)]
    Unknown {
        category: u32,
        unk1: u32,
        unk2: u32,
        unk3: u32,
        unk4: u32,
        unk5: u32,
    },
}

#[binrw]
#[derive(Debug, Clone)]
pub struct ClientTrigger {
    #[brw(pad_size_to = 24)] // take into account categories without params
    pub trigger: ClientTriggerCommand,

    /// Can be a ObjectTypeId or a content ID.
    #[br(temp)]
    #[bw(calc = {
        if let Some(target) = self.target {
            target.into()
        } else {
            content_id.unwrap_or_default()
        }
    })]
    target_id: u64,

    // TODO: double check in a couple of months to see if the double-fallibility here is a stupid idea
    #[br(calc = ObjectTypeId::try_from(target_id).ok())]
    #[bw(ignore)]
    pub target: Option<ObjectTypeId>,
    #[br(calc = if target.is_none() { Some(target_id) } else { None } )]
    #[bw(ignore)]
    pub content_id: Option<u64>,
}

impl Default for ClientTrigger {
    fn default() -> Self {
        Self {
            trigger: ClientTriggerCommand::SetTarget {
                actor_id: ObjectId::default(),
                actor_type: 0,
            },
            target: None,
            content_id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use binrw::BinRead;
    use std::{fs::read, io::Cursor, path::PathBuf};

    use super::*;
    use crate::common::{ObjectId, ObjectTypeId, ObjectTypeKind};

    use crate::client_zone_tests_dir;

    fn client_trigger_bytes(command: u32, params: [u32; 5], target_id: u64) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend(command.to_le_bytes());
        for param in params {
            buffer.extend(param.to_le_bytes());
        }
        buffer.extend(target_id.to_le_bytes());
        buffer
    }

    #[test]
    fn read_toggle_sign() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push(client_zone_tests_dir!("toggle_sign.bin"));

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let toggle_sign = ClientTrigger::read_le(&mut buffer).unwrap();
        assert_eq!(
            toggle_sign.trigger,
            ClientTriggerCommand::ToggleSign {
                sign_id: 4,
                unk1: 0,
                unk2: 0,
            }
        );
        // For this CT command, there should never be a content id in this field.
        assert!(toggle_sign.content_id.is_none());
        assert!(toggle_sign.target.is_some());
        assert_eq!(
            toggle_sign.target.unwrap(),
            ObjectTypeId {
                object_id: ObjectId(1140471), // Random NPC in New Gridania
                object_type: ObjectTypeKind::EObjOrNpc,
            }
        )
    }

    #[test]
    fn read_known_unmodeled_command() {
        let params = [1, 2, 3, 4, 5];
        let buffer = client_trigger_bytes(206, params, 0);
        let mut buffer = Cursor::new(&buffer);

        let trigger = ClientTrigger::read_le(&mut buffer).unwrap();
        assert_eq!(
            trigger.trigger,
            ClientTriggerCommand::Known {
                command: KnownClientTriggerCommand::PublicContentCommand206,
                params,
            }
        );
    }

    #[test]
    fn read_unknown_command() {
        let buffer = client_trigger_bytes(0xFFFF, [1, 2, 3, 4, 5], 0);
        let mut buffer = Cursor::new(&buffer);

        let trigger = ClientTrigger::read_le(&mut buffer).unwrap();
        assert_eq!(
            trigger.trigger,
            ClientTriggerCommand::Unknown {
                category: 0xFFFF,
                unk1: 1,
                unk2: 2,
                unk3: 3,
                unk4: 4,
                unk5: 5,
            }
        );
    }
}
