use binrw::binrw;

use crate::{
    ACTIVE_HELP_BITMASK_SIZE, ADVENTURE_BITMASK_SIZE, AETHER_CURRENT_BITMASK_SIZE,
    AETHERYTE_UNLOCK_BITMASK_SIZE, BUDDY_EQUIP_BITMASK_SIZE, CAUGHT_FISH_BITMASK_SIZE,
    CAUGHT_SPEARFISH_BITMASK_SIZE, CHOCOBO_TAXI_STANDS_BITMASK_SIZE, CLASSJOB_ARRAY_SIZE,
    CUTSCENE_SEEN_BITMASK_SIZE, DUNGEON_ARRAY_SIZE, GLASSES_STYLES_BITMASK_SIZE,
    GUILDHEST_ARRAY_SIZE, MINION_BITMASK_SIZE, MOUNT_BITMASK_SIZE, ORCHESTRION_ROLL_BITMASK_SIZE,
    ORNAMENT_BITMASK_SIZE, PVP_ARRAY_SIZE, RAID_ARRAY_SIZE, TRIAL_ARRAY_SIZE,
    TRIPLE_TRIAD_CARDS_BITMASK_SIZE, UNLOCK_BITMASK_SIZE,
    common::{CHAR_NAME_MAX_LENGTH, read_string, write_string},
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerStatus {
    /// The content ID of the player.
    pub content_id: u64,
    pub crest: u64,
    pub unknown10: u64,
    /// The actor ID of the player.
    pub actor_id: u32,
    pub rested_exp: u32,
    pub companion_current_exp: u32,
    pub unknown1c: u32,
    pub fish_caught: u32,
    pub use_bait_catalog_id: u32,
    pub unknown28: u32,
    pub unknown_pvp2c: u16,
    pub unknown2e: u16,
    /// How many Frontline campaigns you participated in.
    pub pvp_frontline_overall_campaigns: u32,
    pub unknown_timestamp34: u32,
    pub unknown_timestamp38: u32,
    pub unknown3c: u32,
    pub unknown40: u32,
    pub unknown44: u32,
    pub companion_time_passed: f32,
    pub unknown4c: u32,
    pub unknown50: u16,
    pub unknown_pvp52: [u16; 4],
    pub pvp_series_exp: u16,
    /// How many player commendations you received.
    pub player_commendations: u16,
    pub unknown64: [u16; 8],
    pub pvp_rival_wings_total_matches: u16,
    pub pvp_rival_wings_total_victories: u16,
    pub pvp_rival_wings_weekly_matches: u16,
    pub pvp_rival_wings_weekly_victories: u16,
    /// The maximum attainable level on the account. Unsure of it's in-game effect.
    pub max_level: u8,
    /// Which expansion you have acquired. Unsure of it's in-game effect.
    pub expansion: u8,
    pub unknown76: u8,
    pub unknown77: u8,
    pub unknown78: u8,
    pub race: u8,
    pub tribe: u8,
    pub gender: u8,
    pub current_job: u8,
    pub current_class: u8,
    /// The character's chosen deity. Indexed into the GuardianDeity Excel sheet.
    pub deity: u8,
    pub nameday_month: u8,
    pub nameday_day: u8,
    /// The character's initial city-state.
    pub city_state: u8,
    /// The Aetheryte used for the Return action. Indexed into the Aetheryte Excel sheet.
    pub homepoint: u8,
    pub unknown8d: [u8; 3],
    pub companion_rank: u8,
    pub companion_stars: u8,
    pub companion_sp: u8,
    pub companion_unk93: u8,
    pub companion_color: u8,
    pub companion_fav_feed: u8,
    pub fav_aetheryte_count: u8,
    pub unknown97: [u8; 5],
    pub sightseeing21_to_80_unlock: u8, // TODO: might be SightseeingLogUnlockState in ClientStructs?
    pub sightseeing_heavensward_unlock: u8, // TODO: might be SightseeingLogUnlockStateEx in ClientStructs?
    pub unknown9e: [u8; 30],
    /// Current EXP for all classjobs. This doesn't control the class' "unlocked state" in the Character UI.
    pub exp: [u32; CLASSJOB_ARRAY_SIZE],
    pub unknown_pvp124: u32,
    pub pvp_exp: u32,
    pub pvp_frontline_overall_ranks: [u32; 3],
    #[br(count = 16)]
    #[bw(pad_size_to = 16)]
    pub unknown138: Vec<u8>,
    /// Current levels for all classjobs. If non-zero, the class is visibly "unlocked" in the Character UI.
    pub levels: [u16; CLASSJOB_ARRAY_SIZE],
    #[br(count = 8)]
    #[bw(pad_size_to = 8)]
    pub unknown186: Vec<u8>,
    #[br(count = 268)]
    #[bw(pad_size_to = 268)]
    pub unknown194: Vec<u8>,
    #[br(count = 21)]
    #[bw(pad_size_to = 21)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub companion_name: String,
    pub companion_def_rank: u8,
    pub companion_att_rank: u8,
    pub companion_heal_rank: u8,
    #[br(count = MOUNT_BITMASK_SIZE)]
    #[bw(pad_size_to = MOUNT_BITMASK_SIZE)]
    pub mount_guide_mask: Vec<u8>,
    #[br(count = ORNAMENT_BITMASK_SIZE)]
    #[bw(pad_size_to = ORNAMENT_BITMASK_SIZE)]
    pub ornament_mask: Vec<u8>,
    pub unknown281: u8,
    #[br(count = GLASSES_STYLES_BITMASK_SIZE)]
    #[bw(pad_size_to = GLASSES_STYLES_BITMASK_SIZE)]
    pub glasses_styles_mask: Vec<u8>,
    #[br(count = 33)]
    #[bw(pad_size_to = 33)]
    pub unknown302: Vec<u8>,
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    /// Unlock bitmask for everything else, mostly for game features.
    #[brw(pad_before = 32)]
    #[br(count = UNLOCK_BITMASK_SIZE)]
    #[bw(pad_size_to = UNLOCK_BITMASK_SIZE)]
    pub unlocks: Vec<u8>,
    /// Unlock bitmask for Aetherytes.
    #[br(count = AETHERYTE_UNLOCK_BITMASK_SIZE)]
    #[bw(pad_size_to = AETHERYTE_UNLOCK_BITMASK_SIZE)]
    pub aetherytes: Vec<u8>,
    pub favorite_aetheryte_ids: [u16; 4],
    pub free_aetheryte_id: u16,
    pub ps_plus_free_aetheryte_id: u16,
    #[br(count = 482)]
    #[bw(pad_size_to = 482)]
    pub discovery: Vec<u8>,
    pub unknown554: [u8; 27],
    /// Which Active Help guides the player has seen.
    #[br(count = ACTIVE_HELP_BITMASK_SIZE)]
    #[bw(pad_size_to = ACTIVE_HELP_BITMASK_SIZE)]
    pub seen_active_help: Vec<u8>,
    /// Unlock bitmask for minions.
    #[br(count = MINION_BITMASK_SIZE)]
    #[bw(pad_size_to = MINION_BITMASK_SIZE)]
    pub minions: Vec<u8>,
    #[br(count = CHOCOBO_TAXI_STANDS_BITMASK_SIZE)]
    #[bw(pad_size_to = CHOCOBO_TAXI_STANDS_BITMASK_SIZE)]
    pub chocobo_taxi_stands_mask: Vec<u8>,
    #[br(count = CUTSCENE_SEEN_BITMASK_SIZE)]
    #[bw(pad_size_to = CUTSCENE_SEEN_BITMASK_SIZE)]
    pub cutscene_seen_mask: Vec<u8>,
    pub unknown6ff: u8,
    #[br(count = BUDDY_EQUIP_BITMASK_SIZE)]
    #[bw(pad_size_to = BUDDY_EQUIP_BITMASK_SIZE)]
    pub buddy_equip_mask: Vec<u8>,
    pub companion_equipped_head: u8,
    pub companion_equipped_body: u8,
    pub companion_equipped_legs: u8,
    #[br(count = 15)]
    #[bw(pad_size_to = 15)]
    pub unknown_mask: Vec<u8>,
    #[br(count = CAUGHT_FISH_BITMASK_SIZE)]
    #[bw(pad_size_to = CAUGHT_FISH_BITMASK_SIZE)]
    pub caught_fish_mask: Vec<u8>,
    #[br(count = 42)]
    #[bw(pad_size_to = 42)]
    pub unknown7e2: Vec<u8>,
    #[br(count = CAUGHT_SPEARFISH_BITMASK_SIZE)]
    #[bw(pad_size_to = CAUGHT_SPEARFISH_BITMASK_SIZE)]
    pub caught_spearfish_mask: Vec<u8>,
    pub pose: [u8; 2], // TODO: when spearfish_caught_mask was added, size went from 7 to 2, so this is wrong either in size or position (or both)
    pub unknown6df: [u8; 3],
    pub challenge_log_complete: [u8; 13],
    pub secret_recipe_book_mask: [u8; 12],
    pub unknown_mask6f7: [u8; 29],
    pub relic_completion: [u8; 12],
    #[br(count = 50)]
    #[bw(pad_size_to = 50)]
    pub unknown879: Vec<u8>,
    #[br(count = ADVENTURE_BITMASK_SIZE)]
    #[bw(pad_size_to = ADVENTURE_BITMASK_SIZE)]
    pub adventure_mask: Vec<u8>,
    #[br(count = 46)]
    #[bw(pad_size_to = 46)]
    pub hunting_mark_mask: Vec<u8>, // TODO: when adventure_mask (sightseeing_mask) was fixed, size went from 102 to 46, so this is wrong either in size or position (or both)
    #[br(count = 45)]
    #[bw(pad_size_to = 45)]
    pub unknown895: Vec<u8>,
    pub unknown7d7: [u8; 15],
    pub unknown7d8: u8,
    #[br(count = 17)]
    #[bw(pad_size_to = 17)]
    pub unknown7e6: Vec<u8>,
    #[br(count = TRIPLE_TRIAD_CARDS_BITMASK_SIZE)]
    #[bw(pad_size_to = TRIPLE_TRIAD_CARDS_BITMASK_SIZE)]
    pub triple_triad_cards: Vec<u8>,
    pub regional_folklore_mask: [u8; 6],
    #[br(count = 14)]
    #[bw(pad_size_to = 14)]
    pub unknown95a: Vec<u8>,
    #[br(count = AETHER_CURRENT_BITMASK_SIZE)]
    #[bw(pad_size_to = AETHER_CURRENT_BITMASK_SIZE)]
    pub aether_currents_mask: Vec<u8>,
    pub unknown9d7: [u8; 6], // Maybe reserved for Aether Current?
    #[br(count = ORCHESTRION_ROLL_BITMASK_SIZE)]
    #[bw(pad_size_to = ORCHESTRION_ROLL_BITMASK_SIZE)]
    pub orchestrion_roll_mask: Vec<u8>,
    pub hall_of_novice_completion: [u8; 3],
    pub anima_completion: [u8; 11],
    // meh, this is where i put all of the new data
    #[br(count = 45)]
    #[bw(pad_size_to = 45)]
    pub unknown948: Vec<u8>,

    // unlocked status
    #[br(count = RAID_ARRAY_SIZE)]
    #[bw(pad_size_to = RAID_ARRAY_SIZE)]
    pub unlocked_raids: Vec<u8>,

    // FIXME: some pvp/gold saucer duties are located inside of the raids array?!?! I feel like we are understanding this part wrong...
    #[br(count = DUNGEON_ARRAY_SIZE)]
    #[bw(pad_size_to = DUNGEON_ARRAY_SIZE)]
    pub unlocked_dungeons: Vec<u8>,

    #[br(count = GUILDHEST_ARRAY_SIZE)]
    #[bw(pad_size_to = GUILDHEST_ARRAY_SIZE)]
    pub unlocked_guildhests: Vec<u8>,

    #[br(count = TRIAL_ARRAY_SIZE)]
    #[bw(pad_size_to = TRIAL_ARRAY_SIZE)]
    pub unlocked_trials: Vec<u8>,

    #[br(count = PVP_ARRAY_SIZE)]
    #[bw(pad_size_to = PVP_ARRAY_SIZE)]
    pub unlocked_pvp: Vec<u8>,

    // NOTE: all of the following fields are wrong in some way!
    #[br(count = GUILDHEST_ARRAY_SIZE)]
    #[bw(pad_size_to = GUILDHEST_ARRAY_SIZE)]
    pub cleared_guildhests: Vec<u8>,

    #[br(count = TRIAL_ARRAY_SIZE)]
    #[bw(pad_size_to = TRIAL_ARRAY_SIZE)]
    pub cleared_trials: Vec<u8>,

    #[br(count = DUNGEON_ARRAY_SIZE)]
    #[bw(pad_size_to = DUNGEON_ARRAY_SIZE)]
    pub cleared_dungeons: Vec<u8>,

    #[br(count = RAID_ARRAY_SIZE)]
    #[bw(pad_size_to = RAID_ARRAY_SIZE)]
    pub cleared_raids: Vec<u8>,

    #[br(count = PVP_ARRAY_SIZE)]
    #[bw(pad_size_to = PVP_ARRAY_SIZE)]
    pub cleared_pvp: Vec<u8>, // TODO: i don't think this is actually a thing?

    #[br(count = 11)]
    #[bw(pad_size_to = 11)]
    pub unknown949: Vec<u8>,
}
