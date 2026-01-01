use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::player_flags;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = player_flags)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PlayerFlag {
    /// SteamID64 of flagged account (Composite Primary Key, Foreign Key to Account)
    pub steam_id: i64,

    /// Type of flag (e.g., "Cheater", "Bot", "Exploiter", "Racist")
    pub flag_type: String,

    /// Source of the flag (e.g., filename of ruleset that flagged them)
    pub source: String,

    /// UnixTime when flag was first seen
    pub first_seen: i64,

    /// UnixTime when flag was last seen/confirmed
    pub last_seen: i64,

    /// Whether we've notified the user about this flag
    pub notified: bool,
}

#[derive(Clone, Debug, Insertable, AsChangeset)]
#[diesel(table_name = player_flags)]
pub struct NewPlayerFlag {
    pub steam_id: i64,
    pub flag_type: String,
    pub source: String,
    pub first_seen: i64,
    pub last_seen: i64,
    pub notified: bool,
}
