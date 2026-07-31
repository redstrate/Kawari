use binrw::binrw;

use crate::{
    common::{
        FestivalId, ObjectId, PlayerStateFlags1, PlayerStateFlags2, PlayerStateFlags3,
        QuestSpecialFlags, read_bool_from, read_string, write_bool_as, write_string,
    },
    constants::{
        ACTIVE_HELP_BITMASK_SIZE, ADVENTURE_BITMASK_SIZE, AETHER_CURRENT_BITMASK_SIZE,
        AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, AETHERYTE_UNLOCK_BITMASK_SIZE,
        BEAST_TRIBE_ARRAY_SIZE, BEGINNER_TRAINING_ARRAY_SIZE, BUDDY_EQUIP_BITMASK_SIZE,
        CAUGHT_FISH_BITMASK_SIZE, CAUGHT_SPEARFISH_BITMASK_SIZE, CHOCOBO_TAXI_STANDS_BITMASK_SIZE,
        CLASSJOB_ARRAY_SIZE, CONTENTS_NOTE_COMPLETION_FLAGS_BITMASK_SIZE,
        CRYSTALLINE_CONFLICT_ARRAY_SIZE, CUTSCENE_SEEN_BITMASK_SIZE,
        DISCOVERY_MAPS_WITH_UP_TO_16_REGIONS_BITMASK_SIZE,
        DISCOVERY_MAPS_WITH_UP_TO_32_REGIONS_BITMASK_SIZE, DUNGEON_ARRAY_SIZE,
        FRAMERS_KIT_BITMASK_SIZE, FRONTLINE_ARRAY_SIZE, GLASSES_STYLES_BITMASK_SIZE,
        GUILDHEST_ARRAY_SIZE, MASKED_CARNIVALE_ARRAY_SIZE, MINION_BITMASK_SIZE,
        MISC_CONTENT_ARRAY_SIZE, MOUNT_BITMASK_SIZE, ORCHESTRION_ROLL_BITMASK_SIZE,
        ORNAMENT_BITMASK_SIZE, RAID_ARRAY_SIZE, SASTISFACTION_SIZE, SPECIAL_CONTENT_ARRAY_SIZE,
        TRIAL_ARRAY_SIZE, TRIPLE_TRIAD_CARDS_BITMASK_SIZE, UNLOCK_BITMASK_SIZE,
        UNLOCKED_FISHING_SPOTS_BITMASK_SIZE, UNLOCKED_SECRET_RECIPE_BOOKS_BITMASK_SIZE,
    },
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerSetup {
    /// The content ID of the player.
    pub content_id: u64,
    /// Not exactly unused but unsure of the purpose.
    pub padding: [u64; 2],
    /// The actor ID of the player.
    pub actor_id: ObjectId,
    pub rested_exp: u32,
    pub companion_current_exp: u32,
    pub unknown1c: u32,
    pub num_fish_caught: u32,
    pub fishing_bait: u32,
    pub num_spearfish_caught: u32,
    pub unknown_pvp2c: u32,
    pub total_frontline_matches: u32,
    pub squadron_mission_completion_timestamp: i32,
    pub squadron_training_completion_timestamp: i32,
    pub unknown_timestamp38: u32,
    pub weekly_bingo_task_status: [u8; 4],
    pub weekly_bingo_flags: u32,
    pub companion_time_left: f32,
    pub unk44a: u32,
    pub unk_tofu_timestamp: u32,
    pub unk44b: [u8; 10],
    pub pvp_series_experience: u16,
    /// How many player commendations you received.
    pub player_commendations: i16,
    pub unknown64: [u16; 2],
    pub frontline_weekly_matches: u16,
    pub unknown2: u16,
    pub active_gc_army_expedition: u16,
    pub active_gc_army_training: u16,
    pub unknown2a: u16,
    pub weekly_bingo_stickers: u16,
    pub pvp_rival_wings_total_matches: u16,
    pub pvp_rival_wings_total_victories: u16,
    pub pvp_rival_wings_weekly_matches: u16,
    pub pvp_rival_wings_weekly_victories: u16,
    /// The maximum attainable level on the account. Unsure of it's in-game effect.
    pub max_level: u8,
    /// Which expansion you have acquired. Unsure of it's in-game effect.
    pub expansion: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub has_premium_saddlebag: bool,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub unknown77: bool,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub unknown78: bool,
    pub race: u8,
    pub tribe: u8,
    pub sex: u8,
    /// Refers to an index in the ClassJob Excel sheet.
    pub current_classjob_id: u8,
    /// I guess the first class of your character, but I'm unsure?
    pub first_class: u8,
    /// The character's chosen deity. Indexed into the GuardianDeity Excel sheet.
    pub guardian_deity: u8,
    pub birth_month: u8,
    pub birth_day: u8,
    /// The character's initial city-state.
    pub start_town: u8,
    /// The Aetheryte used for the Return action. Indexed into the Aetheryte Excel sheet.
    pub home_aetheryte_id: u16,
    pub quest_special_flags: QuestSpecialFlags,
    pub pet_data: u8,
    pub companion_rank: u8,
    pub companion_stars: u8,
    pub companion_skill_points: u8,
    pub companion_active_command: u8,
    pub companion_color: u8,
    pub companion_favorite_feed: u8,
    pub favourite_aetheryte_count: u8,
    pub daily_quest_seed: u8,
    pub unknown97: u8,
    pub weekly_lockout_info: u8,
    pub relic_id: u8,
    pub relic_note_id: u8,
    pub sightseeing_log_unlock_state: u8,
    pub sightseeing_log_unlock_state_ex: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub unknown9e: bool,
    pub unknown9e1: u8,
    pub meister_flag: u8,
    /// Controls whether or not you can challenge other players.
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub can_do_triple_triad_matches: bool,
    pub aether_current_comp_flg_set_bitmask1: u8, // This is the first byte of the full bitmask. It contains the HW zones, The Fringes and The Ruby Sea. Why this one is here and the rest far down, no idea.
    pub unknown_after_aether: u8,
    #[br(map = read_bool_from::<u8>)]
    #[bw(map = write_bool_as::<u8>)]
    pub has_new_gc_army_candidate: bool,
    pub completed_lovm_stages: u8,
    pub unk111: u8,
    pub supply_seed: u8,
    pub gold_saucer_content_status: u8,
    /// Last expansion mentorship was held. Starts at 1 with Shadowbringers.
    pub mentor_version: u8,
    pub unk_hwd: u8,
    pub weekly_bingo_exp_multiplier: u8,
    pub weekly_bingo_unk63: u8,
    pub series_current_rank: u8,
    pub series_claimed_rank: u8,
    pub previous_series_claimed_rank: u8,
    pub previous_series_rank: u8,
    pub unknowna3: [u8; 7],
    /// Current EXP for all classjobs. This doesn't control the class' "unlocked state" in the Character UI.
    #[br(count = CLASSJOB_ARRAY_SIZE)]
    #[bw(pad_size_to = CLASSJOB_ARRAY_SIZE * 4)]
    pub exp: Vec<i32>,
    pub experience_maelstrom: u32,
    pub experience_twin_adder: u32,
    pub experience_immortal_flames: u32,
    #[br(count = 12)]
    #[bw(pad_size_to = 12)]
    pub unknown138: Vec<u8>,
    pub unknown_unix_timestamp: i32,
    /// Current levels for all classjobs. If non-zero, the class is visibly "unlocked" in the Character UI.
    #[br(count = CLASSJOB_ARRAY_SIZE)]
    #[bw(pad_size_to = CLASSJOB_ARRAY_SIZE * 2)]
    pub levels: Vec<u16>,
    pub ui_festival_ids: [FestivalId; 8],
    pub ui_festival_phases: [u16; 8],
    #[br(count = 176)]
    #[bw(pad_size_to = 176)]
    pub unknown194: Vec<u8>,
    pub beast_reputation_value: [u16; BEAST_TRIBE_ARRAY_SIZE],
    pub quest_unk0: u16,
    pub quest_unk2: u16,
    pub quest_unk4: u16,
    pub quest_unk6: u16,
    pub quest_unk8: u16,
    pub unk8: [u8; 6],
    pub supply_satisfcation: [u16; 12],
    #[br(count = 21)]
    #[bw(pad_size_to = 21)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub companion_name: String,
    pub buddy_levels: [u8; 3],
    #[br(count = MOUNT_BITMASK_SIZE)]
    #[bw(pad_size_to = MOUNT_BITMASK_SIZE)]
    pub mounts: Vec<u8>,
    #[br(count = ORNAMENT_BITMASK_SIZE)]
    #[bw(pad_size_to = ORNAMENT_BITMASK_SIZE)]
    pub ornament_mask: Vec<u8>,
    #[br(count = GLASSES_STYLES_BITMASK_SIZE)]
    #[bw(pad_size_to = GLASSES_STYLES_BITMASK_SIZE)]
    pub glasses_styles_mask: Vec<u8>,
    /// Probably unaccounted for glasses styles.
    pub padding_probably_after_glasses_styles: u8,
    #[br(count = FRAMERS_KIT_BITMASK_SIZE)]
    #[bw(pad_size_to = FRAMERS_KIT_BITMASK_SIZE)]
    pub framers_kits_mask: Vec<u8>,
    // NOTE: Only part of this is used for the name, but bytes >=40 may contain the online ID...? I don't have access to any PS4/PS5/Xbox/NS2 captures yet.
    #[br(count = 64)]
    #[bw(pad_size_to = 64)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    /// Unlock bitmask for everything else, mostly for game features.
    /// This might also be referred to as "rewards".
    #[br(count = UNLOCK_BITMASK_SIZE)]
    #[bw(pad_size_to = UNLOCK_BITMASK_SIZE)]
    pub unlocks: Vec<u8>,
    /// Unlock bitmask for Aetherytes.
    #[br(count = AETHERYTE_UNLOCK_BITMASK_SIZE)]
    #[bw(pad_size_to = AETHERYTE_UNLOCK_BITMASK_SIZE)]
    pub aetherytes: Vec<u8>,
    pub favorite_aetheryte_ids: [u16; 4],
    pub free_aetheryte_id: u16,
    /// Free Aetheryte for Playstation Plus members.
    pub free_aetheryte_ps_plus: u16,
    /// Free Aetheryte for Nintendo Switch Online members.
    pub free_aetheryte_id_nso: u16,
    #[br(count = DISCOVERY_MAPS_WITH_UP_TO_16_REGIONS_BITMASK_SIZE)]
    #[bw(pad_size_to = DISCOVERY_MAPS_WITH_UP_TO_16_REGIONS_BITMASK_SIZE * 2)]
    pub maps_with_up_to_16_regions: Vec<u16>,
    #[br(count = DISCOVERY_MAPS_WITH_UP_TO_32_REGIONS_BITMASK_SIZE)]
    #[bw(pad_size_to = DISCOVERY_MAPS_WITH_UP_TO_32_REGIONS_BITMASK_SIZE * 4)]
    pub maps_with_up_to_32_regions: Vec<u32>,
    pub padding_probably_after_discovery_related_unk2: [u8; 4],
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
    /// Most likely unaccounted for buddy equips.
    pub buddy_equip_mask_padding: u8,
    pub buddy_equip_row_ids: [u8; 3],
    #[br(count = 13)]
    #[bw(pad_size_to = 13)]
    pub unknown_mask: Vec<u8>,
    #[br(count = CAUGHT_FISH_BITMASK_SIZE)]
    #[bw(pad_size_to = CAUGHT_FISH_BITMASK_SIZE)]
    pub caught_fish_mask: Vec<u8>,
    pub padding_probably_after_caught_fish: [u8; 2],
    #[br(count = UNLOCKED_FISHING_SPOTS_BITMASK_SIZE)]
    #[bw(pad_size_to = UNLOCKED_FISHING_SPOTS_BITMASK_SIZE)]
    pub unlocked_fishing_spots: Vec<u8>,
    /// Most likely unaccounted for fishing spots.
    pub fishing_spots_padding: u8,
    #[br(count = CAUGHT_SPEARFISH_BITMASK_SIZE)]
    #[bw(pad_size_to = CAUGHT_SPEARFISH_BITMASK_SIZE)]
    pub caught_spearfish_mask: Vec<u8>,
    pub unlocked_spearfishing_notebooks: [u8; 8],
    /// Most likely unaccounted for spearfishing notebooks.
    pub padding_spearfishing: u8,
    pub rank_malestrom: u8,
    pub rank_twin_adder: u8,
    pub rank_immortal_flames: u8,
    pub beast_reputation_rank: [u8; BEAST_TRIBE_ARRAY_SIZE],
    pub content_roulette_completion: [u8; 10],
    pub unknown_mask6f7: [u8; 10],
    pub player_state_flags1: PlayerStateFlags1,
    pub player_state_flags2: PlayerStateFlags2,
    pub player_state_flags3: PlayerStateFlags3,
    pub contents_note_completion_flags: [u8; CONTENTS_NOTE_COMPLETION_FLAGS_BITMASK_SIZE],
    pub unlocked_secret_recipe_books: [u8; UNLOCKED_SECRET_RECIPE_BOOKS_BITMASK_SIZE],
    #[br(count = 28)]
    #[bw(pad_size_to = 28)]
    pub unknown879: Vec<u8>,
    pub relic_monster_progress: [u8; 10],
    pub objective_progress: [u8; 2],
    #[br(count = ADVENTURE_BITMASK_SIZE)]
    #[bw(pad_size_to = ADVENTURE_BITMASK_SIZE)]
    pub adventure_mask: Vec<u8>,
    #[br(count = 124)]
    #[bw(pad_size_to = 124)]
    pub hunting_mark_data: Vec<u8>,
    #[br(count = TRIPLE_TRIAD_CARDS_BITMASK_SIZE)]
    #[bw(pad_size_to = TRIPLE_TRIAD_CARDS_BITMASK_SIZE)]
    pub triple_triad_cards: Vec<u8>,
    #[br(count = 17)]
    #[bw(pad_size_to = 17)]
    pub unknown95a: Vec<u8>,
    // We do -1 because of aether_current_comp_flg_set_bitmask1 being present way earlier.
    #[br(count = AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE - 1)]
    #[bw(pad_size_to = AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE - 1)]
    pub aether_current_comp_flg_set_bitmask2: Vec<u8>, // This is the rest of the full bitmask. The rest of the zones are in here.
    #[br(count = AETHER_CURRENT_BITMASK_SIZE)]
    #[bw(pad_size_to = AETHER_CURRENT_BITMASK_SIZE)]
    pub aether_currents_mask: Vec<u8>,
    pub unlocked_miner_folklore_tomes: [u8; 2],
    pub unlocked_botainst_folklore_tomes: [u8; 2],
    pub unlocked_fisher_folklore_tomes: [u8; 2],
    #[br(count = ORCHESTRION_ROLL_BITMASK_SIZE)]
    #[bw(pad_size_to = ORCHESTRION_ROLL_BITMASK_SIZE)]
    pub orchestrion_roll_mask: Vec<u8>,
    #[br(count = BEGINNER_TRAINING_ARRAY_SIZE)]
    #[bw(pad_size_to = BEGINNER_TRAINING_ARRAY_SIZE)]
    pub completed_beginner_training: Vec<u8>,
    pub unk_completion2: [u8; 11],

    pub weekly_bingo_order_data: [u8; 16],
    pub weekly_bingo_reward_data: [u8; 4],

    pub supply_satisfaction_ranks: [u8; SASTISFACTION_SIZE],
    pub used_supply_allowances: [u8; SASTISFACTION_SIZE],

    #[br(count = SPECIAL_CONTENT_ARRAY_SIZE)]
    #[bw(pad_size_to = SPECIAL_CONTENT_ARRAY_SIZE)]
    pub unlocked_special_content: Vec<u8>,

    // unlocked status
    #[br(count = RAID_ARRAY_SIZE)]
    #[bw(pad_size_to = RAID_ARRAY_SIZE)]
    pub unlocked_raids: Vec<u8>,

    #[br(count = DUNGEON_ARRAY_SIZE)]
    #[bw(pad_size_to = DUNGEON_ARRAY_SIZE)]
    pub unlocked_dungeons: Vec<u8>,

    #[br(count = GUILDHEST_ARRAY_SIZE)]
    #[bw(pad_size_to = GUILDHEST_ARRAY_SIZE)]
    pub unlocked_guildhests: Vec<u8>,

    #[br(count = TRIAL_ARRAY_SIZE)]
    #[bw(pad_size_to = TRIAL_ARRAY_SIZE)]
    pub unlocked_trials: Vec<u8>,

    #[br(count = CRYSTALLINE_CONFLICT_ARRAY_SIZE)]
    #[bw(pad_size_to = CRYSTALLINE_CONFLICT_ARRAY_SIZE)]
    pub unlocked_crystalline_conflict: Vec<u8>,

    #[br(count = FRONTLINE_ARRAY_SIZE)]
    #[bw(pad_size_to = FRONTLINE_ARRAY_SIZE)]
    pub unlocked_frontline: Vec<u8>,

    // cleared status
    #[br(count = RAID_ARRAY_SIZE)]
    #[bw(pad_size_to = RAID_ARRAY_SIZE)]
    pub cleared_raids: Vec<u8>,

    #[br(count = DUNGEON_ARRAY_SIZE)]
    #[bw(pad_size_to = DUNGEON_ARRAY_SIZE)]
    pub cleared_dungeons: Vec<u8>,

    #[br(count = GUILDHEST_ARRAY_SIZE)]
    #[bw(pad_size_to = GUILDHEST_ARRAY_SIZE)]
    pub cleared_guildhests: Vec<u8>,

    #[br(count = TRIAL_ARRAY_SIZE)]
    #[bw(pad_size_to = TRIAL_ARRAY_SIZE)]
    pub cleared_trials: Vec<u8>,

    #[br(count = CRYSTALLINE_CONFLICT_ARRAY_SIZE)]
    #[bw(pad_size_to = CRYSTALLINE_CONFLICT_ARRAY_SIZE)]
    pub cleared_crystalline_conflict: Vec<u8>,

    #[br(count = FRONTLINE_ARRAY_SIZE)]
    #[bw(pad_size_to = FRONTLINE_ARRAY_SIZE)]
    pub cleared_frontline: Vec<u8>,

    #[br(count = MASKED_CARNIVALE_ARRAY_SIZE)]
    #[bw(pad_size_to = MASKED_CARNIVALE_ARRAY_SIZE)]
    pub cleared_masked_carnivale: Vec<u8>,

    pub completed_vvd_notebook_contents: [u8; 7],

    #[br(count = MISC_CONTENT_ARRAY_SIZE)]
    #[bw(pad_size_to = MISC_CONTENT_ARRAY_SIZE)]
    pub unlocked_misc_content: Vec<u8>,

    pub unk_after_misc_content: u8,

    #[br(count = MISC_CONTENT_ARRAY_SIZE)]
    #[bw(pad_size_to = MISC_CONTENT_ARRAY_SIZE)]
    pub cleared_misc_content: Vec<u8>,

    pub unknown949: [u8; 2],
}
