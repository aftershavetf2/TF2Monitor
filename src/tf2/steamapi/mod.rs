// https://developer.valvesoftware.com/wiki/Steam_Web_API
// Functions to interact with the Steam Web API:
// - GetPlayerSummaries
//   - Can take multiple IDs.
//   - For account name, avatar info and account age.
// - GetFriendList
//   - Array of friend's Steam ID64s.
// - GetPlayerBans
//

mod get_bans;
mod get_friendlist;
mod get_player_summaries;
mod get_steam_comments;
mod get_tf2_play_minutes;
pub mod steamapi_thread;

use self::get_player_summaries::get_player_summaries;
use super::lobby::{AccountAge, PlayerSteamInfo, Tf2PlayMinutes};
use crate::{
    models::{app_settings::AppSettings, steamid::SteamID},
    reputation::Reputation,
};
use chrono::{DateTime, Local, TimeZone};
use serde::Deserialize;
use std::{collections::HashSet, error::Error};

#[derive(Debug, Clone)]
pub enum SteamApiMsg {
    PlayerSummary(PlayerSteamInfo),
    FriendsList(SteamID, HashSet<SteamID>),
    Tf2Playtime(SteamID, Tf2PlayMinutes),
    SteamBans(SteamID, SteamPlayerBan),
    ProfileComments(SteamID, Vec<SteamProfileComment>),
    ApproxAccountAge(SteamID, AccountAge),
    Reputation(Reputation),
}

pub struct SteamApi {
    steam_api_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SteamApiPlayer {
    pub steamid: String,
    // pub personaname: String,
    pub communityvisibilitystate: u8,
    // profileurl: String,
    pub avatar: String,
    // pub avatarmedium: String,
    pub avatarfull: String,
    // avatarhash: String,
    pub timecreated: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SteamPlayerBan {
    pub steamid: SteamID,
    pub community_banned: bool,
    pub vac_banned: bool,
    pub number_of_vac_bans: u32,
    pub days_since_last_ban: u32,
    pub number_of_game_bans: u32,
    // pub economy_ban: String,
}

#[derive(Debug, Clone)]
pub struct SteamProfileComment {
    pub name: String,
    pub steamid: SteamID,
    pub comment: String,
}

impl SteamApi {
    pub fn new(app_settings: &AppSettings) -> Self {
        Self {
            steam_api_key: app_settings.steam_api_key.clone(),
        }
    }

    /// Fetches player summaries from the Steam API for a list of steamdids
    pub fn get_player_summaries(
        &mut self,
        steamids: Vec<SteamID>,
    ) -> Result<Vec<SteamApiPlayer>, Box<dyn Error>> {
        get_player_summaries(&self.steam_api_key, steamids)
    }

    pub fn get_friendlist(&self, steamid: SteamID) -> Option<HashSet<SteamID>> {
        get_friendlist::get_friendlist(&self.steam_api_key, steamid)
    }

    pub fn get_tf2_play_minutes(&self, steamid: SteamID) -> Tf2PlayMinutes {
        get_tf2_play_minutes::get_tf2_play_minutes(&self.steam_api_key, steamid)
    }

    pub fn get_bans(&self, steamids: Vec<SteamID>) -> Option<Vec<SteamPlayerBan>> {
        get_bans::get_bans(&self.steam_api_key, steamids)
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
