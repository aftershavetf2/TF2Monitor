// https://developer.valvesoftware.com/wiki/Steam_Web_API
// Functions to interact with the Steam Web API:
// - GetPlayerSummaries
//   - Can take multiple IDs.
//   - For account name, avatar info and account age.
// - GetFriendList
//   - Array of friend's Steam ID64s.
// - GetPlayerBans
//

mod get_friendlist;
mod get_player_summariess;
pub mod steamapi_thread;

use std::collections::HashSet;

use crate::models::{app_settings::AppSettings, steamid::SteamID};
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;

use self::get_player_summariess::get_player_summaries;

use super::lobby::PlayerSteamInfo;

#[derive(Debug, Clone)]
pub enum SteamApiMsg {
    PlayerSummary(PlayerSteamInfo),
    FriendsList(SteamID, HashSet<SteamID>),
}

pub struct SteamApi {
    steam_api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
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

impl SteamApi {
    pub fn new(app_settings: &AppSettings) -> Self {
        Self {
            steam_api_key: app_settings.steam_api_key.clone(),
        }
    }

    /// Fetches player summaries from the Steam API for a list of steamdids
    pub fn get_player_summaries(&mut self, steamids: Vec<SteamID>) -> Option<Vec<SteamApiPlayer>> {
        get_player_summaries(&self.steam_api_key, steamids)
    }

    pub fn get_friendlist(&self, steamid: SteamID) -> Option<HashSet<SteamID>> {
        get_friendlist::get_friendlist(&self.steam_api_key, steamid)
    }

    /// Returns true if the Steam API key is set
    pub fn has_key(&self) -> bool {
        !self.steam_api_key.is_empty()
    }
}

impl SteamApiPlayer {
    pub fn get_account_age(&self) -> Option<DateTime<Local>> {
        self.timecreated?;

        let timecreated = self.timecreated.unwrap() as i64;
        match Local.timestamp_opt(timecreated, 0) {
            chrono::offset::LocalResult::Single(x) => Some(x),
            _ => None,
        }
    }
}
