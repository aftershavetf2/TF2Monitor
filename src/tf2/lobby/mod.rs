pub mod friendships;
pub mod lobby_thread;

use super::steamapi::SteamPlayerBan;
use crate::models::steamid::SteamID;
use chrono::{DateTime, Local};
use friendships::Friendships;
use std::collections::{HashMap, HashSet};

#[derive(Default, Debug, Clone)]
pub struct Lobby {
    pub self_steamid: SteamID,
    pub players: Vec<Player>,
    pub chat: Vec<LobbyChat>,
    pub friendships: Friendships,

    /// Players who no longer show up in the status command output
    /// or in tf_lobby_debug output. Players are kept in here for 1 minute.
    pub recently_left_players: Vec<Player>,
}

#[derive(Default, Debug, Clone)]
pub struct LobbyChat {
    pub when: DateTime<Local>,
    pub steamid: SteamID,

    /// The name of the player who sent the message,
    /// Used when the player has left the lobby.
    pub player_name: String,

    pub message: String,
    pub dead: bool,
    pub team: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum AccountAge {
    /// Steam info is not loaded yet
    #[default]
    Loading,

    /// Steam info has been loaded and the account age is known.
    Loaded(DateTime<Local>),

    /// Steam info has been loaded but the profile is private.
    /// This will trigger an approximation of the account age.
    Private,

    /// Account age has been approximated
    Approx(DateTime<Local>),

    /// Unknown account age, due to private profile and failed approximation
    Unknown,
}

#[derive(Default, Debug, Clone)]
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

    pub account_age: AccountAge,

    // This is the PlayerFlags(Cheater etc) for the player
    // The String is the source of the flags.
    // The source is filename of the rules file that set the flags.
    pub flags: HashMap<String, PlayerMarking>,
}

#[derive(Debug, Clone)]
pub struct PlayerMarking {
    /// The rules file which resulted in this marking
    pub source: String,

    /// Some rules files may suggest a marking,
    /// but it is not enforced unless the user
    /// adds the rule file as trusted
    pub suggestion: bool,

    /*
       /// Any comment on why the player was marked
       pub reason: String,

    */
    /// The actual flags that were set
    pub flags: HashSet<PlayerFlag>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum PlayerFlag {
    Cool,
    Cheater,
    Bot,
    Suspicious,
    Toxic,
    Exploiter,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Team {
    #[default]
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
    pub public_profile: bool,
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
                    reasons.push_str(
                        format!("- {} days since last ban\n", steam_bans.days_since_last_ban)
                            .as_str(),
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
    pub fn new(self_steamid: SteamID) -> Self {
        Self {
            self_steamid,
            players: Vec::new(),
            chat: Vec::new(),
            friendships: Friendships::default(),
            recently_left_players: Vec::new(),
        }
    }

    pub fn get_me(&self) -> Option<&Player> {
        self.get_player(None, Some(self.self_steamid))
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

    fn update_friendships(&mut self) {
        self.friendships = Friendships::from_lobby(self);
    }
}

pub fn flag_description(flag: PlayerFlag) -> &'static str {
    match flag {
        PlayerFlag::Cool => "Cool",
        PlayerFlag::Cheater => "Cheater",
        PlayerFlag::Bot => "Bot",
        PlayerFlag::Suspicious => "Suspicious",
        PlayerFlag::Toxic => "Toxic",
        PlayerFlag::Exploiter => "Exploiter",
    }
}
