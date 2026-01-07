use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::account;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Account {
    /// SteamID64 of account (Primary Key)
    pub steam_id: i64,

    /// Account name (max 32 visible characters, UTF-8)
    pub name: String,

    /// UnixTime when account was created, approximated if private (nullable)
    pub created_date: Option<i64>,

    /// URL to avatar thumb image
    pub avatar_thumb_url: String,

    /// URL to avatar full image
    pub avatar_full_url: String,

    /// Whether profile is public
    pub public_profile: bool,

    /// UnixTime when account data was last updated
    pub last_updated: i64,

    /// UnixTime when friend list was last fetched (nullable)
    pub friends_fetched: Option<i64>,

    /// UnixTime when comments was last fetched (nullable)
    pub comments_fetched: Option<i64>,

    /// UnixTime when playtimes was last fetched (nullable)
    pub playtimes_fetched: Option<i64>,

    /// UnixTime when reputation/sourcebans was last fetched (nullable)
    pub reputation_fetched: Option<i64>,

    /// UnixTime when steam bans (VAC/Game bans) was last fetched (nullable)
    pub steam_bans_last_fetched: Option<i64>,
}

#[derive(Clone, Debug, Insertable, AsChangeset)]
#[diesel(table_name = account)]
pub struct NewAccount {
    pub steam_id: i64,
    pub name: String,
    pub created_date: Option<i64>,
    pub avatar_thumb_url: String,
    pub avatar_full_url: String,
    pub public_profile: bool,
    pub last_updated: i64,
    pub friends_fetched: Option<i64>,
    pub comments_fetched: Option<i64>,
    pub playtimes_fetched: Option<i64>,
    pub reputation_fetched: Option<i64>,
    pub steam_bans_last_fetched: Option<i64>,
}
