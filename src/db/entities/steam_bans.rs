use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::schema::steam_bans;

#[derive(Clone, Debug, PartialEq, Eq, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = steam_bans)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct SteamBan {
    /// SteamID64 of account (Primary Key)
    /// Note: NOT a foreign key to account.steam_id - allows storing bans
    /// for players that don't yet exist in the account table
    pub steam_id: i64,

    /// Whether the player is community banned
    pub community_banned: bool,

    /// Whether the player has a VAC ban
    pub vac_banned: bool,

    /// Number of VAC bans on record
    pub number_of_vac_bans: i32,

    /// Days since the player's last ban
    pub days_since_last_ban: i32,

    /// Number of game bans on record
    pub number_of_game_bans: i32,

    /// Economy ban status (e.g., "none", "probation", "banned")
    pub economy_ban: String,
}

#[derive(Clone, Debug, Insertable, AsChangeset)]
#[diesel(table_name = steam_bans)]
pub struct NewSteamBan {
    pub steam_id: i64,
    pub community_banned: bool,
    pub vac_banned: bool,
    pub number_of_vac_bans: i32,
    pub days_since_last_ban: i32,
    pub number_of_game_bans: i32,
    pub economy_ban: String,
}
