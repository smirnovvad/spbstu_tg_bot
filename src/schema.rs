table! {
    groups (id) {
        id -> Integer,
        name -> Text,
        api_id -> Text,
    }
}

table! {
    users (id) {
        id -> Integer,
        tg_id -> Integer,
        tg_name -> Text,
        notify -> Bool,
        group_id -> Integer,
    }
}

allow_tables_to_appear_in_same_query!(
    groups,
    users,
);
