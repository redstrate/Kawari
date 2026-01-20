diesel::table! {
    character (content_id) {
        content_id -> BigInt,
        service_account_id -> BigInt,
        actor_id -> BigInt,
        gm_rank -> Integer,
        name -> Text,
        time_played_minutes -> BigInt,
    }
}

diesel::table! {
    classjob (content_id) {
        content_id -> BigInt,
        current_class -> Integer,
        levels -> Text,
        exp -> Text,
        first_class -> Integer,
        rested_exp -> Integer,
    }
}

diesel::joinable!(classjob -> character (content_id));

diesel::table! {
    customize (content_id) {
        content_id -> BigInt,
        chara_make -> Text,
        city_state -> Integer,
        remake_mode -> Integer,
    }
}

diesel::joinable!(customize -> character (content_id));

diesel::table! {
    quest (content_id) {
        content_id -> BigInt,
        completed -> Text,
        active -> Text,
    }
}

diesel::joinable!(quest -> character (content_id));

diesel::table! {
    aetheryte (content_id) {
        content_id -> BigInt,
        unlocked -> Text,
        homepoint -> Integer,
        favorite_aetherytes -> Text,
        free_aetheryte -> Integer,
    }
}

diesel::joinable!(aetheryte -> character (content_id));

diesel::table! {
    volatile (content_id) {
        content_id -> BigInt,
        position -> Text,
        rotation -> Double,
        zone_id -> Integer,
        display_flags -> Integer,
        title -> Integer,
    }
}

diesel::joinable!(volatile -> character (content_id));

diesel::table! {
    inventory (content_id) {
        content_id -> BigInt,
        contents -> Text,
    }
}

diesel::joinable!(inventory -> character (content_id));

diesel::table! {
    aether_current (content_id) {
        content_id -> BigInt,
        comp_flg_set -> Text,
        unlocked -> Text,
    }
}

diesel::joinable!(aether_current -> character (content_id));

diesel::table! {
    companion (content_id) {
        content_id -> BigInt,
        unlocked_equip -> Text,
    }
}

diesel::joinable!(companion -> character (content_id));

diesel::table! {
    content (content_id) {
        content_id -> BigInt,
        unlocked_special_content -> Text,
        unlocked_raids -> Text,
        unlocked_dungeons -> Text,
        unlocked_guildhests -> Text,
        unlocked_trials -> Text,
        unlocked_crystalline_conflicts -> Text,
        unlocked_frontlines -> Text,
        cleared_raids -> Text,
        cleared_dungeons -> Text,
        cleared_guildhests -> Text,
        cleared_trials -> Text,
        cleared_crystalline_conflicts -> Text,
        cleared_frontlines -> Text,
        cleared_masked_carnivale -> Text,
        unlocked_misc_content -> Text,
        cleared_misc_content -> Text,
    }
}

diesel::joinable!(content -> character (content_id));

diesel::table! {
    unlock (content_id) {
        content_id -> BigInt,
        unlocks -> Text,
        seen_active_help -> Text,
        minions -> Text,
        mounts -> Text,
        orchestrion_rolls -> Text,
        cutscene_seen -> Text,
        ornaments -> Text,
        caught_fish -> Text,
        caught_spearfish -> Text,
        adventures -> Text,
        triple_triad_cards -> Text,
        glasses_styles -> Text,
        chocobo_taxi_stands -> Text,
        titles -> Text,
    }
}

diesel::joinable!(unlock -> character (content_id));

diesel::allow_tables_to_appear_in_same_query!(
    character,
    classjob,
    customize,
    quest,
    aetheryte,
    volatile,
    inventory,
    aether_current,
    companion,
    content,
    unlock
);
