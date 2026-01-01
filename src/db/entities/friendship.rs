use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::friendship;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = friendship)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Friendship {
    /// SteamID64 of account (Composite Primary Key, Foreign Key to Account)
    pub steam_id: i64,

    /// SteamID64 of friend (Composite Primary Key, Foreign Key to Account)
    pub friend_steam_id: i64,

    /// Name of friend (max 32 visible characters, UTF-8)
    pub friend_name: String,

    /// UnixTime when they first was found to be friends
    pub friend_date: i64,

    /// UnixTime when they no longer were found to be friends (nullable)
    pub unfriend_date: Option<i64>,
}

#[derive(Clone, Debug, Insertable, AsChangeset)]
#[diesel(table_name = friendship)]
pub struct NewFriendship {
    pub steam_id: i64,
    pub friend_steam_id: i64,
    pub friend_name: String,
    pub friend_date: i64,
    pub unfriend_date: Option<i64>,
}

