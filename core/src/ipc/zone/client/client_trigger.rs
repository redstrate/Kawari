use binrw::binrw;
use strum_macros::IntoStaticStr;

use crate::common::{
    ContainerType, DirectorTrigger, DistanceRange, HandlerId, ObjectId, ObjectTypeId,
    read_bool_from, write_bool_as,
};
use crate::ipc::zone::WaymarkPosition;
use crate::ipc::zone::client::HouseId;

#[binrw]
#[derive(Debug, PartialEq, Clone, IntoStaticStr)]
pub enum ClientTriggerCommand {
    /// The player sheathes/unsheathes their weapon.
    #[brw(magic = 1u32)]
    ToggleWeapon {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        shown: bool,
        unk_flag: u32,
    },

    /// When toggling auto-attack on and off.
    #[brw(magic = 2u32)]
    ToggleAutoAttack {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        on: bool,
        target: ObjectId,
        unk3: u32,
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

    /// The client requests to ride pillion with another player.
    #[brw(magic = 106u32)]
    RidePillionRequest {
        /// The target actor to ride with.
        target_actor_id: ObjectId,
        /// The target seat to occupy.
        target_seat_index: u32,
    },

    /// The client is finished zoning.
    #[brw(magic = 201u32)]
    FinishZoning {},

    /// The player selects a teleport destination.
    #[brw(magic = 202u32)]
    TeleportQuery {
        aetheryte_id: u32,
        // TODO: fill out the rest
    },

    /// The player answers a teleport offer sent by someone in their party.
    #[brw(magic = 203u32)]
    TeleportOfferReply {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        decline_teleport: bool,
    },

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
        /// The actor to apply the sign to.
        target_actor: ObjectTypeId,
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

    /// The client clears all waymarks.
    #[brw(magic = 313u32)]
    ClearAllWaymarks {},

    /// The client begins using the Idle Camera or Group Pose feature.
    #[brw(magic = 314u32)]
    GroupPoseOrIdleCamera { unk1: u32, unk2: u32 },

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

    /// The client requests materia melding from another player.
    #[brw(magic = 413u32)]
    RequestMateriaMeld { actor_id: ObjectId },

    #[brw(magic = 444u32)]
    OpenChocoboSaddlebag {},

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

    /// The client requests repair from another player.
    #[brw(magic = 450u32)]
    RequestRepair { actor_id: ObjectId },

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

    /// The client is "reapplying" the existing pose, like after idling.
    #[brw(magic = 506u32)]
    ReapplyPose { unk1: u32, pose: u32 },

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
    BeginOrEndFishing {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        end: bool,
    },

    /// When the client requests information about a GatheringPoint node that was spawned.
    #[brw(magic = 706u32)]
    RequestGatheringPoint {
        /// Index into the GatheringPoint Excel sheet.
        id: u32,
    },

    /// Various triggers related to instanced content.
    #[brw(magic = 808u32)]
    DirectorTrigger {
        handler_id: HandlerId,
        trigger: DirectorTrigger,
        arg: u32,
    },

    /// When a player requests an NPC (or player?) to a Triple Triad match.
    #[brw(magic = 815u32)]
    TripleTriadChallenge {
        actor_id: ObjectId, // probably
    },

    /// When a player requests to abandon their instanced content.
    #[brw(magic = 819u32)]
    AbandonContent {
        /// Believe this is 1 for timed out, but I'm unsure.
        unk1: u32,
    },

    #[brw(magic = 1107u32)]
    RequestHousingWardInfo {
        /// The zone id of the housing ward.
        zone_id: u32,
        /// 0-based index, so ward number - 1.
        ward_index: u32,
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

    /// The client requests the housing inventory be sent to them. This happens automatically after opening the Interior Furnishings menu.
    #[brw(magic = 1121u32)]
    RequestHousingInventory {
        /// Which housing inventory to access. If true, the client wants the storeroom's inventory, otherwise, the placed furniture.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        storeroom: bool,
    },

    /// The client opens or closes the Interior Furnishings menu.
    #[brw(magic = 1123u32)]
    FurnitureMenuToggled {
        /// If the menu was closed or not.
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        closed: bool,
    },

    /// The client requests to warp to the housing interior's front door.
    #[brw(magic = 1122u32)]
    HousingMoveToFrontDoor {},

    /// The client has requested an apartment building's residents list from either an apartment building entrance or an apartment's front door.
    #[brw(magic = 1125u32)]
    RequestApartmentList {
        /// The desired starting index of apartments.
        starting_index: u32,
    },

    /// The client sets the interior lighting level.
    #[brw(magic = 1137u32)]
    SetInteriorLightLevel {
        /// See HousingInteriorDetails in housing_interior_furniture.rs for further details, but `level` is actually a level of *darkness*, not light, so this CT is a misnomer, but it's more intuitive to just call it a light level...
        level: u32,
        unk: u32, // Seems to be always 1
    },

    /// The client places furniture from the storeroom.
    #[brw(magic = 1150u32)]
    PlaceFurnitureFromStoreroom {
        /// The house's id.
        house_id: HouseId,
        /// The source container to retrieve the item from.
        container_type: ContainerType,
        /// The index into the container.
        container_index: u16,
        unk: u16, // Just in case, observed as zeroes but with all this stuff lately, you never know.
    },

    /// The client requests to repair and item at a mender.
    #[brw(magic = 1600u32)]
    RepairItem { unk1: u32, unk2: u32, unk3: u32 },

    /// The client is performing a pet action.
    #[brw(magic = 1800u32)]
    PetAction {
        /// Index into the PetAction Excel sheet.
        action_id: u32,
    },

    /// The client opens the General tab in the Gold Saucer window.
    #[brw(magic = 1850u32)]
    OpenGoldSaucerGeneralTab {},

    /// The client challengers another player to a normal match.
    #[brw(magic = 1950u32)]
    ChallengeNormalMatch { unk1: u32, unk2: u32 },

    #[brw(magic = 1980u32)]
    BeginContentsReplay {},

    #[brw(magic = 1981u32)]
    EndContentsReplay {},

    /// Sent whenever the client tries to begin a Hall of the Novice exercise.
    #[brw(magic = 2050u32)]
    BeginNoviceExercise {
        id: u32, // not specific to a class/job
    },

    /// Sent whenever the client uses the /nastatus command.
    #[brw(magic = 2100u32)]
    ToggleNoviceStatus {},

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

    /// Sent whenever the Glamour Plates window is opened or closed.
    #[brw(magic = 2356u32)]
    ToggleGlamourPlatesWindow {
        #[br(map = read_bool_from::<u32>)]
        #[bw(map = write_bool_as::<u32>)]
        open: bool,
    },

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

    #[doc(hidden)]
    Unknown {
        category: u32,
        // seen in haircut event
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
