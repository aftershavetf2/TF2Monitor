// https://developer.valvesoftware.com/wiki/Steam_Web_API
// Functions to interact with the Steam Web API:
// - GetPlayerSummaries
//   - Can take multiple IDs.
//   - For account name, avatar info and account age.
// - GetFriendList
//   - Array of friend's Steam ID64s.
// - GetPlayerBans
//

mod get_player_summariess;

use crate::models::{app_settings::AppSettings, steamid::SteamID};
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;

use self::get_player_summariess::get_player_summaries;

pub struct SteamApi {
    steam_api_key: String,
    self_steamid64: SteamID,
}

#[derive(Debug, Deserialize)]
pub struct SteamApiPlayer {
    pub steamid: String,
    pub personaname: String,
    // profileurl: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    // avatarhash: String,
    pub timecreated: Option<u64>,
}

impl SteamApiPlayer {
    pub fn get_account_age(&self) -> Option<DateTime<Local>> {
        if self.timecreated.is_none() {
            return None;
        }

        let timecreated = self.timecreated.unwrap() as i64;
        match Local.timestamp_opt(timecreated as i64, 0) {
            chrono::offset::LocalResult::Single(x) => Some(x),
            _ => None,
        }

        // format!("{}", timecreated.format("%Y-%m-%d"))
    }
}
impl SteamApi {
    pub fn new(app_settings: &AppSettings) -> Self {
        Self {
            steam_api_key: app_settings.steam_api_key.clone(),
            self_steamid64: app_settings.self_steamid64,
        }
    }

    pub fn get_player_summaries(&mut self, steamids: Vec<SteamID>) -> Option<Vec<SteamApiPlayer>> {
        get_player_summaries(&self.steam_api_key, steamids)
    }
}
