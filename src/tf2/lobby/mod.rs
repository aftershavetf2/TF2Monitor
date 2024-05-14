use chrono::{DateTime, Local};

use crate::models::steamid::SteamID;

pub mod lobby_thread;

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

    pub fn get_player_by_name(&self, name: &str) -> Option<&Player> {
        for player in self.players.iter() {
            if player.name == name {
                return Some(player);
            }
        }
        None
    }

    pub fn get_player_by_name_mut(&mut self, name: &str) -> Option<&mut Player> {
        for player in self.players.iter_mut() {
            if player.name == name {
                return Some(player);
            }
        }
        None
    }

    pub fn get_player_by_steamid(&self, steamid: &SteamID) -> Option<&Player> {
        for player in self.players.iter() {
            if player.steamid == *steamid {
                return Some(player);
            }
        }
        None
    }

    pub fn get_player_by_steamid_mut(&mut self, steamid: &SteamID) -> Option<&mut Player> {
        for player in self.players.iter_mut() {
            if player.steamid == *steamid {
                return Some(player);
            }
        }
        None
    }
}
