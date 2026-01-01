use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::bans;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = bans)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Ban {
    /// Auto-increment primary key
    pub id: i64,

    /// SteamID64 of banned account (Foreign Key to Account)
    pub steam_id: i64,

    /// Source of the ban (e.g., "sourcebans.net", "steamrep.com")
    pub source: String,

    /// Type of ban (e.g., "cheating", "griefing", "scamming")
    pub ban_type: String,

    /// Reason for the ban
    pub reason: Option<String>,

    /// UnixTime when ban was created/discovered
    pub created_date: i64,

    /// UnixTime when ban expires (nullable for permanent bans)
    pub expires_date: Option<i64>,

    /// Whether this is a permanent ban
    pub permanent: bool,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = bans)]
pub struct NewBan {
    pub steam_id: i64,
    pub source: String,
    pub ban_type: String,
    pub reason: Option<String>,
    pub created_date: i64,
    pub expires_date: Option<i64>,
    pub permanent: bool,
}
