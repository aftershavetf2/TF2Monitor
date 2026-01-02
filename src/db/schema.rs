// @generated automatically by Diesel CLI.

diesel::table! {
    account (steam_id) {
        steam_id -> BigInt,
        name -> Text,
        created_date -> Nullable<BigInt>,
        avatar_thumb_url -> Text,
        avatar_full_url -> Text,
        public_profile -> Bool,
        last_updated -> BigInt,
        friends_fetched -> Nullable<BigInt>,
        comments_fetched -> Nullable<BigInt>,
        playtimes_fetched -> Nullable<BigInt>,
        reputation_fetched -> Nullable<BigInt>,
    }
}

diesel::table! {
    bans (id) {
        id -> BigInt,
        steam_id -> BigInt,
        source -> Text,
        ban_type -> Text,
        reason -> Nullable<Text>,
        created_date -> BigInt,
        expires_date -> Nullable<BigInt>,
        permanent -> Bool,
    }
}

diesel::table! {
    ban_sources (name) {
        name -> Text,
        url -> Text,
        parser -> Text,
        last_checked -> Nullable<BigInt>,
        active -> Bool,
    }
}

diesel::table! {
    comments (id) {
        id -> BigInt,
        steam_id -> BigInt,
        writer_steam_id -> BigInt,
        writer_name -> Text,
        comment -> Text,
        created_date -> BigInt,
        deleted_date -> Nullable<BigInt>,
    }
}

diesel::table! {
    friendship (steam_id, friend_steam_id) {
        steam_id -> BigInt,
        friend_steam_id -> BigInt,
        friend_name -> Text,
        friend_date -> BigInt,
        unfriend_date -> Nullable<BigInt>,
    }
}

diesel::table! {
    player_flags (steam_id, flag_type, source) {
        steam_id -> BigInt,
        flag_type -> Text,
        source -> Text,
        first_seen -> BigInt,
        last_seen -> BigInt,
        notified -> Bool,
    }
}

diesel::table! {
    playtime (steam_id, game) {
        steam_id -> BigInt,
        game -> Text,
        play_minutes -> BigInt,
        last_updated -> BigInt,
    }
}

diesel::joinable!(bans -> account (steam_id));
diesel::joinable!(comments -> account (steam_id));
diesel::joinable!(friendship -> account (steam_id));
diesel::joinable!(player_flags -> account (steam_id));
diesel::joinable!(playtime -> account (steam_id));

diesel::allow_tables_to_appear_in_same_query!(
    account,
    bans,
    ban_sources,
    comments,
    friendship,
    player_flags,
    playtime,
);
