use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::ban_sources;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = ban_sources)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct BanSource {
    pub name: String,
    pub url: String,
    pub parser: String,
    pub last_checked: Option<i64>,
    pub active: bool,
}

#[derive(Clone, Debug, Insertable, AsChangeset)]
#[diesel(table_name = ban_sources)]
pub struct NewBanSource {
    pub name: String,
    pub url: String,
    pub parser: String,
    pub last_checked: Option<i64>,
    pub active: bool,
}
