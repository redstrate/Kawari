use binrw::binrw;

use crate::{
    CHAR_NAME_MAX_LENGTH,
    common::{read_string, write_string},
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerSetup {
    pub content_id: u64,
    pub crest: u64,
    pub unknown10: u64,
    pub char_id: u32,
    pub rested_exp: u32,
    pub companion_current_exp: u32,
    pub unknown1c: u32,
    pub fish_caught: u32,
    pub use_bait_catalog_id: u32,
    pub unknown28: u32,
    pub unknown_pvp2c: u16,
    pub unknown2e: u16,
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
    pub player_commendations: u16,
    pub unknown64: [u16; 8],
    pub pvp_rival_wings_total_matches: u16,
    pub pvp_rival_wings_total_victories: u16,
    pub pvp_rival_wings_weekly_matches: u16,
    pub pvp_rival_wings_weekly_victories: u16,
    pub max_level: u8,
    pub expansion: u8,
    pub unknown76: u8,
    pub unknown77: u8,
    pub unknown78: u8,
    pub race: u8,
    pub tribe: u8,
    pub gender: u8,
    pub current_job: u8,
    pub current_class: u8,
    pub deity: u8,
    pub nameday_month: u8,
    pub nameday_day: u8,
    pub city_state: u8,
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
    pub sightseeing21_to_80_unlock: u8,
    pub sightseeing_heavensward_unlock: u8,
    pub unknown9e: [u8; 26],
    pub exp: [u32; 32],
    pub pvp_total_exp: u32,
    pub unknown_pvp124: u32,
    pub pvp_exp: u32,
    pub pvp_frontline_overall_ranks: [u32; 3],
    pub unknown138: u32,
    pub levels: [u16; 32],
    #[br(count = 218)]
    #[bw(pad_size_to = 218)]
    pub unknown194: Vec<u8>,
    pub companion_name: [u8; 21],
    pub companion_def_rank: u8,
    pub companion_att_rank: u8,
    pub companion_heal_rank: u8,
    #[br(count = 33)]
    #[bw(pad_size_to = 33)]
    pub mount_guide_mask: Vec<u8>,
    pub ornament_mask: [u8; 4],
    #[br(count = 85)]
    #[bw(pad_size_to = 85)]
    pub unknown281: Vec<u8>,
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    pub unknown293: [u8; 16],
    pub unknown2a3: u8,
    #[br(count = 64)]
    #[bw(pad_size_to = 64)]
    pub unlock_bitmask: Vec<u8>,
    pub aetheryte: [u8; 26],
    pub favorite_aetheryte_ids: [u16; 4],
    pub free_aetheryte_id: u16,
    pub ps_plus_free_aetheryte_id: u16,
    #[br(count = 480)]
    #[bw(pad_size_to = 480)]
    pub discovery: Vec<u8>,
    #[br(count = 36)]
    #[bw(pad_size_to = 36)]
    pub howto: Vec<u8>,
    pub unknown554: [u8; 4],
    #[br(count = 60)]
    #[bw(pad_size_to = 60)]
    pub minions: Vec<u8>,
    pub chocobo_taxi_mask: [u8; 12],
    #[br(count = 159)]
    #[bw(pad_size_to = 159)]
    pub watched_cutscenes: Vec<u8>,
    pub companion_barding_mask: [u8; 12],
    pub companion_equipped_head: u8,
    pub companion_equipped_body: u8,
    pub companion_equipped_legs: u8,
    #[br(count = 287)]
    #[bw(pad_size_to = 287)]
    pub unknown_mask: Vec<u8>,
    pub pose: [u8; 7],
    pub unknown6df: [u8; 3],
    pub challenge_log_complete: [u8; 13],
    pub secret_recipe_book_mask: [u8; 12],
    pub unknown_mask6f7: [u8; 29],
    pub relic_completion: [u8; 12],
    #[br(count = 37)]
    #[bw(pad_size_to = 37)]
    pub sightseeing_mask: Vec<u8>,
    #[br(count = 102)]
    #[bw(pad_size_to = 102)]
    pub hunting_mark_mask: Vec<u8>,
    #[br(count = 45)]
    #[bw(pad_size_to = 45)]
    pub triple_triad_cards: Vec<u8>,
    pub unknown895: u8,
    pub unknown7d7: [u8; 15],
    pub unknown7d8: u8,
    #[br(count = 49)]
    #[bw(pad_size_to = 49)]
    pub unknown7e6: Vec<u8>,
    pub regional_folklore_mask: [u8; 6],
    #[br(count = 87)]
    #[bw(pad_size_to = 87)]
    pub orchestrion_mask: Vec<u8>,
    pub hall_of_novice_completion: [u8; 3],
    pub anima_completion: [u8; 11],
    #[br(count = 41)]
    #[bw(pad_size_to = 41)]
    pub unknown85e: Vec<u8>,
    pub unlocked_raids: [u8; 28],
    pub unlocked_dungeons: [u8; 18],
    pub unlocked_guildhests: [u8; 10],
    pub unlocked_trials: [u8; 12],
    pub unlocked_pvp: [u8; 5],
    pub cleared_raids: [u8; 28],
    pub cleared_dungeons: [u8; 18],
    pub cleared_guildhests: [u8; 10],
    pub cleared_trials: [u8; 12],
    pub cleared_pvp: [u8; 5],
    pub unknown948: [u8; 15],
}
