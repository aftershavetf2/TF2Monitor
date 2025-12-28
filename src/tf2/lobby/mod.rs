pub mod friendships;
pub mod lobby_thread;
pub mod shared_lobby;

use super::steamapi::{SteamPlayerBan, SteamProfileComment};
use crate::{
    models::steamid::SteamID,
    reputation::Reputation,
    tf2bd::models::{PlayerAttribute, PlayerInfo},
};
use chrono::{DateTime, Local};
use friendships::Friendships;
use std::collections::HashSet;

#[derive(Default, Debug, Clone)]
pub struct Lobby {
    pub lobby_id: String,
    pub self_steamid: SteamID,
    pub players: Vec<Player>,
    pub chat: Vec<LobbyChat>,
    pub kill_feed: Vec<LobbyKill>,
    pub friendships: Friendships,

    /// Players who no longer show up in the status command output
    /// or in tf_lobby_debug output. Players are kept in here for 1 minute.
    pub recently_left_players: Vec<Player>,

    chat_msg_id: i64,
}

#[derive(Default, Debug, Clone)]
pub struct LobbyKill {
    pub when: DateTime<Local>,
    pub killer: SteamID,
    pub victim: SteamID,
    pub weapon: String,
    pub crit: bool,
}

#[derive(Default, Debug, Clone)]
pub struct LobbyChat {
    pub chat_msg_id: i64,

    pub when: DateTime<Local>,
    pub steamid: SteamID,

    /// The name of the player who sent the message,
    /// Used when the player has left the lobby.
    pub player_name: String,

    pub message: String,
    pub translated_message: Option<String>,
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

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub enum Tf2PlayMinutes {
    #[default]
    Loading,

    PlayMinutes(u32),

    /// Unknown playtime
    Unknown,
}

#[derive(Default, Debug, Clone)]
pub struct Player {
    /// The player's ID in the lobby, used when votekicking etc
    pub id: i64,
    pub steamid: SteamID,
    pub name: String,
    pub team: Team,
    pub alive: bool,
    pub pingms: i64,
    pub score: i64,
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
    pub tf2_play_minutes: Tf2PlayMinutes,
    pub steam_bans: Option<SteamPlayerBan>,
    pub profile_comments: Option<Vec<SteamProfileComment>>,
    pub reputation: Option<Reputation>,

    pub account_age: AccountAge,

    // This is the PlayerFlags(Cheater etc) for the player
    // The String is the source of the flags.
    // The source is filename of the rules file that set the flags.
    pub player_info: Option<PlayerInfo>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Team {
    #[default]
    Unknown,
    Spec,
    Red,
    Blue,
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
    // pub name: String,
    pub avatar: String,
    // pub avatarmedium: String,
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

        let mut hours = 0;
        let has_few_hours: bool = match self.tf2_play_minutes {
            Tf2PlayMinutes::Loading => false,
            Tf2PlayMinutes::PlayMinutes(minutes) => {
                hours = minutes / 60;
                minutes < 60 * 500
            }
            Tf2PlayMinutes::Unknown => false,
        };

        match (is_new_account, has_few_hours) {
            (true, true) => Some(format!(
                "Account is < 1 year old and has only {} hours in TF2",
                hours
            )),
            (true, false) => Some("Account is < 1 year old".to_string()),
            (false, true) => Some(format!("Account has only {} hours in TF2", hours)),
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
            chat_msg_id: 0,
            lobby_id: Local::now().format("%Y-%m-%d").to_string(),
            self_steamid,
            players: Vec::new(),
            chat: Vec::new(),
            kill_feed: Vec::new(),
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

/// Returns text and tooltip for a player attribute
pub fn player_attribute_description(
    player_attribute: PlayerAttribute,
) -> (&'static str, &'static str) {
    match player_attribute {
        PlayerAttribute::Cool => ("★", "Cool"),
        PlayerAttribute::Cheater => ("C", "Cheater"),
        PlayerAttribute::Bot => ("B", "Bot"),
        PlayerAttribute::Suspicious => ("S", "Suspicious"),
        PlayerAttribute::Toxic => ("T", "Toxic"),
        PlayerAttribute::Exploiter => ("E", "Exploiter"),
    }
}
