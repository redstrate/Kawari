use diesel::prelude::*;
use kawari::constants::{
    ACTIVE_HELP_BITMASK_SIZE, ADVENTURE_BITMASK_SIZE, AETHER_CURRENT_BITMASK_SIZE,
    AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE, AETHERYTE_UNLOCK_BITMASK_SIZE,
    BUDDY_EQUIP_BITMASK_SIZE, CAUGHT_FISH_BITMASK_SIZE, CAUGHT_SPEARFISH_BITMASK_SIZE,
    CHOCOBO_TAXI_STANDS_BITMASK_SIZE, COMPLETED_QUEST_BITMASK_SIZE,
    CRYSTALLINE_CONFLICT_ARRAY_SIZE, CUTSCENE_SEEN_BITMASK_SIZE, DUNGEON_ARRAY_SIZE,
    FRONTLINE_ARRAY_SIZE, GLASSES_STYLES_BITMASK_SIZE, GUILDHEST_ARRAY_SIZE, MINION_BITMASK_SIZE,
    MOUNT_BITMASK_SIZE, ORCHESTRION_ROLL_BITMASK_SIZE, ORNAMENT_BITMASK_SIZE, RAID_ARRAY_SIZE,
    TITLE_UNLOCK_BITMASK_SIZE, TRIAL_ARRAY_SIZE, TRIPLE_TRIAD_CARDS_BITMASK_SIZE,
    UNLOCK_BITMASK_SIZE,
};

use crate::{Bitmask, QuestBitmask};

#[derive(Insertable, Identifiable, Queryable, Selectable)]
#[diesel(table_name = super::schema::character)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(primary_key(content_id))]
pub struct Character {
    pub content_id: i64,
    pub service_account_id: i64,
    pub actor_id: i64,
    pub gm_rank: i32,
    pub name: String,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::classjob)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct ClassJob {
    pub content_id: i64,
    pub classjob_id: i32,
    pub classjob_levels: String,
    pub classjob_exp: String,
    pub first_classjob: i32,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::customize)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Customize {
    pub content_id: i64,
    pub chara_make: String,
    pub city_state: i32,
    pub remake_mode: i32,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::quest)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Quest {
    pub content_id: i64,
    pub completed: QuestBitmask<COMPLETED_QUEST_BITMASK_SIZE>,
    pub active: String,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::aetheryte)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Aetheryte {
    pub content_id: i64,
    pub unlocked: Bitmask<AETHERYTE_UNLOCK_BITMASK_SIZE>,
    pub homepoint: i32,
    pub favorite_aetherytes: String,
    pub free_aetheryte: i32,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::volatile)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Volatile {
    pub content_id: i64,
    pub pos_x: f64,
    pub pos_y: f64,
    pub pos_z: f64,
    pub rotation: f64,
    pub zone_id: i32,
    pub display_flags: i32,
    pub title: i32,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::inventory)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Inventory {
    pub content_id: i64,
    pub contents: String,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::aether_current)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct AetherCurrent {
    pub content_id: i64,
    pub comp_flg_set: Bitmask<AETHER_CURRENT_COMP_FLG_SET_BITMASK_SIZE>,
    pub unlocked: Bitmask<AETHER_CURRENT_BITMASK_SIZE>,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::companion)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Companion {
    pub content_id: i64,
    pub unlocked_equip: Bitmask<BUDDY_EQUIP_BITMASK_SIZE>,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::content)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Content {
    pub content_id: i64,
    pub unlocked_raids: Bitmask<RAID_ARRAY_SIZE>,
    pub unlocked_dungeons: Bitmask<DUNGEON_ARRAY_SIZE>,
    pub unlocked_guildhests: Bitmask<GUILDHEST_ARRAY_SIZE>,
    pub unlocked_trials: Bitmask<TRIAL_ARRAY_SIZE>,
    pub unlocked_crystalline_conflicts: Bitmask<CRYSTALLINE_CONFLICT_ARRAY_SIZE>,
    pub unlocked_frontlines: Bitmask<FRONTLINE_ARRAY_SIZE>,
    pub cleared_raids: Bitmask<RAID_ARRAY_SIZE>,
    pub cleared_dungeons: Bitmask<DUNGEON_ARRAY_SIZE>,
    pub cleared_guildhests: Bitmask<GUILDHEST_ARRAY_SIZE>,
    pub cleared_trials: Bitmask<TRIAL_ARRAY_SIZE>,
    pub cleared_crystalline_conflicts: Bitmask<CRYSTALLINE_CONFLICT_ARRAY_SIZE>,
    pub cleared_frontlines: Bitmask<FRONTLINE_ARRAY_SIZE>,
}

#[derive(
    Insertable,
    Identifiable,
    Queryable,
    Selectable,
    Associations,
    AsChangeset,
    Debug,
    Default,
    Clone,
)]
#[diesel(table_name = super::schema::unlock)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(Character, foreign_key = content_id))]
#[diesel(primary_key(content_id))]
pub struct Unlock {
    pub content_id: i64,
    pub unlocks: Bitmask<UNLOCK_BITMASK_SIZE>,
    pub seen_active_help: Bitmask<ACTIVE_HELP_BITMASK_SIZE>,
    pub minions: Bitmask<MINION_BITMASK_SIZE>,
    pub mounts: Bitmask<MOUNT_BITMASK_SIZE>,
    pub orchestrion_rolls: Bitmask<ORCHESTRION_ROLL_BITMASK_SIZE>,
    pub cutscene_seen: Bitmask<CUTSCENE_SEEN_BITMASK_SIZE>,
    pub ornaments: Bitmask<ORNAMENT_BITMASK_SIZE>,
    pub caught_fish: Bitmask<CAUGHT_FISH_BITMASK_SIZE>,
    pub caught_spearfish: Bitmask<CAUGHT_SPEARFISH_BITMASK_SIZE>,
    pub adventures: Bitmask<ADVENTURE_BITMASK_SIZE>,
    pub triple_triad_cards: Bitmask<TRIPLE_TRIAD_CARDS_BITMASK_SIZE>,
    pub glasses_styles: Bitmask<GLASSES_STYLES_BITMASK_SIZE>,
    pub chocobo_taxi_stands: Bitmask<CHOCOBO_TAXI_STANDS_BITMASK_SIZE>,
    pub titles: Bitmask<TITLE_UNLOCK_BITMASK_SIZE>,
}
