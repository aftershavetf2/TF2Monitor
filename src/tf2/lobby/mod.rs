pub mod add_steam_info;
pub mod lobby_thread;

use crate::models::steamid::SteamID;
use chrono::{DateTime, Local};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Team {
    Unknown,
    Invaders,
    Defendes,
    Spec,
}

#[derive(Debug, Clone)]
pub struct PlayerKill {
    pub weapon: String,
    pub crit: bool,
}

#[derive(Debug, Clone)]
pub struct Player {
    pub id: u32,
    pub steamid: SteamID,
    pub name: String,
    pub team: Team,
    pub kills: u32,
    pub deaths: u32,
    pub crit_kills: u32,
    pub crit_deaths: u32,
    pub kills_with: Vec<PlayerKill>,
    pub last_seen: DateTime<Local>,

    pub steam_info: Option<PlayerSteamInfo>,
}

#[derive(Debug, Clone)]
pub struct PlayerSteamInfo {
    pub steamid: SteamID,
    pub name: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    pub account_age: Option<DateTime<Local>>,

    pub friends: Option<Vec<SteamID>>,
}

impl PlayerSteamInfo {
    pub fn get_account_created(&self) -> String {
        if self.account_age.is_none() {
            return "Unknown".to_string();
        }

        let account_age = self.account_age.unwrap();

        format!("{}", account_age.format("%Y-%m-%d"))
    }

    pub fn is_account_new(&self) -> bool {
        if self.account_age.is_none() {
            return false;
        }

        let account_age = self.account_age.unwrap();
        let days = (Local::now() - account_age).num_days();

        days < 365
    }
}

#[derive(Default, Debug, Clone)]
pub struct Lobby {
    pub players: Vec<Player>,
    pub chat: Vec<LobbyChat>,
}

#[derive(Default, Debug, Clone)]
pub struct LobbyChat {
    pub when: DateTime<Local>,
    pub steamid: SteamID,
    pub message: String,
    pub dead: bool,
    pub team: bool,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            chat: Vec::new(),
        }
    }

    pub fn get_player(&self, name: Option<&str>, steamid: Option<SteamID>) -> Option<&Player> {
        self.players
            .iter()
            .find(|player| Some(player.name.as_str()) == name || Some(player.steamid) == steamid)
    }

    pub fn get_player_mut(
        &mut self,
        name: Option<&str>,
        steamid: Option<SteamID>,
    ) -> Option<&mut Player> {
        self.players
            .iter_mut()
            .find(|player| Some(player.name.as_str()) == name || Some(player.steamid) == steamid)
    }

    pub fn is_friend_of_self(&self, self_steamid: SteamID, steamid: SteamID) -> bool {
        if let Some(me) = self.get_player(None, Some(self_steamid)) {
            if let Some(steam_info) = &me.steam_info {
                if let Some(friends) = &steam_info.friends {
                    return friends.contains(&steamid);
                }
            }
        }

        false
    }

    pub fn get_friendlist_of(&self, steamid: SteamID) -> Vec<&Player> {
        // TODO: First look up the player, if it has a friendlist, use that
        // Otherwise, check all players' friendlists
        let friends: Vec<&Player> = self
            .players
            .iter()
            .filter(|player| {
                if let Some(steam_info) = &player.steam_info {
                    if let Some(friends) = &steam_info.friends {
                        return friends.contains(&steamid);
                    }
                }

                false
            })
            .collect();

        friends
    }
}
