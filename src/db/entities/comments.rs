use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::comments;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = comments)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Comment {
    /// Auto-increment primary key
    pub id: i64,

    /// SteamID64 of account (Foreign Key to Account)
    pub steam_id: i64,

    /// SteamID64 of writer of comment (Foreign Key to Account)
    pub writer_steam_id: i64,

    /// Name of writer of the comment (max 32 visible characters, UTF-8)
    pub writer_name: String,

    /// Comment text
    pub comment: String,

    /// UnixTime when comment was first seen
    pub created_date: i64,

    /// UnixTime when the comment no longer was found on the account (nullable)
    pub deleted_date: Option<i64>,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = comments)]
pub struct NewComment {
    pub steam_id: i64,
    pub writer_steam_id: i64,
    pub writer_name: String,
    pub comment: String,
    pub created_date: i64,
    pub deleted_date: Option<i64>,
}

