diesel::table! {
    user (id) {
        id -> BigInt,
        username -> Text,
        password -> Text,
    }
}

diesel::table! {
    session (user_id) {
        user_id -> BigInt,
        time -> Text,
        service -> Text,
        sid -> Text,
    }
}

diesel::joinable!(session -> user (user_id));

diesel::table! {
    service_account (id) {
        id -> BigInt,
        user_id -> BigInt,
        max_ex -> Integer,
    }
}

diesel::joinable!(service_account -> user (user_id));

diesel::allow_tables_to_appear_in_same_query!(user, session, service_account,);
