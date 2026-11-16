use crate::db::schema::steam_bans;
use diesel::prelude::*;

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
