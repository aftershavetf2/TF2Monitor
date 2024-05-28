pub mod lobby_thread;

use super::steamapi::SteamPlayerBan;
use crate::models::{steamid::SteamID, PlayerFlags};
use chrono::{DateTime, Local};
use std::collections::HashSet;

#[derive(Default, Debug, Clone)]
pub struct Lobby {
    pub players: Vec<Player>,
    pub chat: Vec<LobbyChat>,

    /// Players who no longer show up in the status command output
    /// or in tf_lobby_debug output. Players are kept in here for 1 minute.
    pub recently_left_players: Vec<Player>,
}

#[derive(Default, Debug, Clone)]
pub struct LobbyChat {
    pub when: DateTime<Local>,
    pub steamid: SteamID,
    pub message: String,
    pub dead: bool,
    pub team: bool,
}

#[derive(Debug, Clone)]
pub struct Player {
    /// The player's ID in the lobby, used when votekicking etc
    pub id: u32,
    pub steamid: SteamID,
    pub name: String,
    pub team: Team,
    pub kills: u32,
    pub deaths: u32,
    pub crit_kills: u32,
    pub crit_deaths: u32,
    pub kills_with: Vec<PlayerKill>,

    /// The last time the player was seen in the
    /// status or tf_lobby_debug command output.
    pub last_seen: DateTime<Local>,
    pub steam_info: Option<PlayerSteamInfo>,
    pub friends: Option<HashSet<SteamID>>,
    pub tf2_play_minutes: Option<u32>,
    pub steam_bans: Option<SteamPlayerBan>,

    pub markings: PlayerMarkings,
}

#[derive(Default, Debug, Clone)]
pub struct PlayerMarkings {
    pub markings: Vec<PlayerMarking>,
}

#[derive(Debug, Clone)]
pub struct PlayerMarking {
    /*
       /// The rules file which resulted in this marking
       pub source: String,

       /// Any comment on why the player was marked
       pub reason: String,

       /// Some rules files may suggest a marking, but not enforce it
       pub suggestion: bool,
    */
    /// The actual flags that were set
    pub flags: Vec<PlayerFlags>,
}

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
pub struct PlayerSteamInfo {
    pub steamid: SteamID,
    pub name: String,
    pub avatar: String,
    pub avatarmedium: String,
    pub avatarfull: String,
    pub account_age: Option<DateTime<Local>>,
}

impl Player {
    pub fn has_steam_bans(&self) -> Option<String> {
        if let Some(steam_bans) = &self.steam_bans {
            let has_any_bans = steam_bans.community_banned
                || steam_bans.vac_banned
                || steam_bans.number_of_game_bans > 0;
            if has_any_bans {
                let mut reasons = String::new();

                reasons.push_str("Player has bans: \n");
                if steam_bans.vac_banned {
                    reasons.push_str(
                        format!("- VAC banned {} times\n", steam_bans.number_of_vac_bans).as_str(),
                    );
                }

                if steam_bans.number_of_game_bans > 0 {
                    reasons.push_str(
                        format!("- Game banned {} times\n", steam_bans.number_of_game_bans)
                            .as_str(),
                    );
                }

                if steam_bans.community_banned {
                    reasons.push_str("- Community banned\n");
                }

                return Some(reasons);
            }
        }

        None
    }

    pub fn is_newbie(&self) -> Option<String> {
        let mut is_new_account = false;
        if let Some(steam_info) = &self.steam_info {
            is_new_account = steam_info.is_account_new();
        }

        let mut has_few_hours = false;
        if let Some(tf2_play_minutes) = self.tf2_play_minutes {
            let min_minutes = 60 * 500;
            if tf2_play_minutes > 0 && tf2_play_minutes < min_minutes {
                has_few_hours = true;
            }
        }

        match (is_new_account, has_few_hours) {
            (true, true) => Some(format!(
                "Account is < 1 year old and has only {} hours in TF2",
                self.tf2_play_minutes.unwrap() / 60
            )),
            (true, false) => Some("Account is < 1 year old".to_string()),
            (false, true) => Some(format!(
                "Account has only {} hours in TF2",
                self.tf2_play_minutes.unwrap() / 60
            )),
            _ => None,
        }
    }
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

impl Lobby {
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            chat: Vec::new(),
            recently_left_players: Vec::new(),
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
            if let Some(friends) = &me.friends {
                return friends.contains(&steamid);
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
                if let Some(friends) = &player.friends {
                    return friends.contains(&steamid);
                }

                false
            })
            .collect();

        friends
    }
}
