use binrw::binrw;

use crate::{
    AETHERYTE_UNLOCK_BITMASK_SIZE, CLASSJOB_ARRAY_SIZE, DUNGEON_ARRAY_SIZE, GUILDHEST_ARRAY_SIZE,
    PVP_ARRAY_SIZE, RAID_ARRAY_SIZE, TRIAL_ARRAY_SIZE, UNLOCK_BITMASK_SIZE,
    common::{CHAR_NAME_MAX_LENGTH, read_string, write_string},
};

#[binrw]
#[derive(Debug, Clone, Default)]
pub struct PlayerStatus {
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
    pub exp: [u32; CLASSJOB_ARRAY_SIZE],
    pub pvp_total_exp: u32,
    pub unknown_pvp124: u32,
    pub pvp_exp: u32,
    pub pvp_frontline_overall_ranks: [u32; 3],
    #[br(count = 16)]
    #[bw(pad_size_to = 16)]
    pub unknown138: Vec<u8>,
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
    #[br(count = 33)]
    #[bw(pad_size_to = 33)]
    pub mount_guide_mask: Vec<u8>,
    pub ornament_mask: [u8; 4],
    #[br(count = 50)]
    #[bw(pad_size_to = 50)]
    pub unknown281: Vec<u8>,
    #[br(count = CHAR_NAME_MAX_LENGTH)]
    #[bw(pad_size_to = CHAR_NAME_MAX_LENGTH)]
    #[br(map = read_string)]
    #[bw(map = write_string)]
    pub name: String,
    #[brw(pad_before = 32)]
    #[br(count = UNLOCK_BITMASK_SIZE)]
    #[bw(pad_size_to = UNLOCK_BITMASK_SIZE)]
    pub unlocks: Vec<u8>,
    #[br(count = AETHERYTE_UNLOCK_BITMASK_SIZE)]
    #[bw(pad_size_to = AETHERYTE_UNLOCK_BITMASK_SIZE)]
    pub aetherytes: Vec<u8>,
    pub favorite_aetheryte_ids: [u16; 4],
    //#[brw(pad_before = 4)] // TODO: This seems to have been removed in 7.3?
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
    #[br(count = 161)]
    #[bw(pad_size_to = 161)]
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
    // meh, this is where i put all of the new data
    #[br(count = 152)]
    #[bw(pad_size_to = 152)]
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

    // cleared status
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

// TODO: update testdata for 7.3
/*#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use binrw::BinRead;

    use super::*;

    #[test]
    fn read_playerspawn() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/player_setup.bin");

        let buffer = read(d).unwrap();
        let mut buffer = Cursor::new(&buffer);

        let player_setup = PlayerStatus::read_le(&mut buffer).unwrap();
        assert_eq!(player_setup.content_id, 0x004000174c50560d);
        assert_eq!(player_setup.char_id, 0x107476e7);
        assert_eq!(player_setup.name, "Lavenaa Warren");
        assert_eq!(player_setup.max_level, 100);
        assert_eq!(player_setup.gender, 1);
        assert_eq!(player_setup.race, 1);
        assert_eq!(player_setup.tribe, 2);
        assert_eq!(player_setup.expansion, 5);
        assert_eq!(player_setup.current_job, 1); // gladiator
        assert_eq!(player_setup.current_class, 1); // ditto
        assert_eq!(
            player_setup.levels,
            [
                0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0
            ]
        ); // only GLA
    }
}*/
