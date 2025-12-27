use super::steamid::SteamID;
use crate::tf2bd::models::PlayerAttribute;
use crate::utils::BoxResult;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::process::exit;

const SETTINGS_FILENAME: &str = "settings.json";

fn get_true() -> bool {
    true
}

fn default_party_notifications_for() -> Vec<PlayerAttribute> {
    vec![PlayerAttribute::Cheater, PlayerAttribute::Bot]
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub log_filename: String,
    pub exe_filename: String,

    pub self_steamid64: SteamID,

    /// Steam API key
    /// Used for fetching info about players from Steam
    /// Go here to crate a new key: https://steamcommunity.com/dev/apikey
    pub steam_api_key: String,

    /// TF2 RCON settings
    pub rcon_password: String,
    pub rcon_ip: String,
    pub rcon_port: u16,

    // View settings
    #[serde(default = "get_true")]
    pub show_friendship_indicators: bool,

    #[serde(default = "get_true")]
    pub show_crits: bool,

    // Auto actions
    #[serde(default = "bool::default")]
    pub kick_cheaters: bool,

    #[serde(default = "get_true")]
    pub kick_bots: bool,

    #[serde(default = "default_party_notifications_for")]
    pub party_notifications_for: Vec<PlayerAttribute>,

    // Window position and size
    #[serde(default)]
    pub window_position: Option<(f32, f32)>,

    #[serde(default)]
    pub window_size: Option<(f32, f32)>,
}

impl Default for AppSettings {
    fn default() -> Self {
        let self_steamid64 = get_current_user_steamid().unwrap_or(SteamID::from_u64(0));

        Self {
            log_filename: get_log_filename(),
            exe_filename: get_exe_filename(),

            self_steamid64,

            steam_api_key: "".to_string(),

            rcon_password: "rconpwd".to_string(),
            rcon_ip: "127.0.0.1".to_string(),
            rcon_port: 40434,

            show_friendship_indicators: true,
            show_crits: true,

            kick_cheaters: false,
            kick_bots: true,

            party_notifications_for: default_party_notifications_for(),

            window_position: None,
            window_size: None,
        }
    }
}

impl AppSettings {
    /// Tries to load the preferences.rust_bot_detector.json file from the current directory.
    /// If the file don't exist, use default values.
    pub fn load_or_default() -> Self {
        match Self::load() {
            Ok(preferences) => preferences,
            Err(error) => {
                log::error!("Error loading settings file: {}.", error);

                let settings = AppSettings::save_default_settings();

                log::warn!(
                    "Please edit the {} file and restart the application.",
                    SETTINGS_FILENAME
                );

                settings
            }
        }
    }

    fn save_default_settings() -> AppSettings {
        let settings = AppSettings::default();

        let json = serde_json::to_string_pretty(&settings).unwrap();
        log::info!("Using default values: {}.", json);

        settings.save();

        settings
    }

    /// Load the settings from the settings.json file.
    /// If the file does not exist, return the error
    /// If the file exists but is invalid, log a warning and exit the application.
    pub fn load() -> BoxResult<AppSettings> {
        let mut f = File::open(SETTINGS_FILENAME)?;
        let mut json = String::new();
        f.read_to_string(&mut json)?;
        let settings: AppSettings = serde_json::from_str(&json).unwrap();

        log::info!("Settings loaded from file {}", SETTINGS_FILENAME);
        log::info!(
            "Settings: \n{}",
            serde_json::to_string_pretty(&settings).unwrap()
        );

        settings.save();

        if !settings.validate_settings() {
            log::info!("Settings are not valid.");
            exit(1);
        }

        Ok(settings)
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut f = File::create(SETTINGS_FILENAME).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        log::info!("Settings saved to file {}", SETTINGS_FILENAME);
    }

    /// Validates the settings and logs warnings if something is wrong.
    /// Returns true if all settings are valid.
    /// Not all problems are fatal, so this function can return true even if there are warnings.
    fn validate_settings(&self) -> bool {
        let mut valid = true;

        if !Path::new(&self.exe_filename).exists() {
            log::warn!("TF2 exe file '{}' does not exist. Please check the path and edit the settings.json file and try again.", self.exe_filename);
            valid = false;
        }

        if !Path::new(&self.log_filename).exists() {
            log::warn!("Log file '{}' does not exist. Maybe path is wrong or you have not yet started TF2?", self.log_filename);
        }

        if !self.self_steamid64.is_valid() {
            log::warn!("SteamID for yourself is empty or not valid. You will not see a white rectangle for youself in the scoreboard.");
        }

        if self.steam_api_key.is_empty() {
            log::warn!("Steam API key is empty. Some features will not work. Go here to crate a new key: https://steamcommunity.com/dev/apikey");
        }

        if self.rcon_password.is_empty() {
            log::warn!("RCON password is empty. RCON will not work.");
            valid = false;
        }

        valid
    }
}

//
// Platform specific functions
// - Try to find the SteadID for the current user
// - To get the default log and exe filenames
//

#[cfg(target_os = "windows")]
fn get_log_filename() -> String {
    r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf\console.log".to_string()
}

#[cfg(target_os = "windows")]
fn get_exe_filename() -> String {
    r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf_win64.exe".to_string()
}

#[cfg(target_os = "linux")]
fn get_log_filename() -> String {
    std::fs::canonicalize(r"~/.local/share/Steam/steamapps/common/Team Fortress 2/tf/console.log")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[cfg(target_os = "linux")]
fn get_exe_filename() -> String {
    std::fs::canonicalize(r"~/.local/share/Steam/steamapps/common/Team Fortress 2/hl2_linux")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

/// Get the SteamID for the current user, if possible.
/// This is used to show the current user in the scoreboard.
#[cfg(target_os = "windows")]
pub fn get_current_user_steamid() -> Option<SteamID> {
    use winreg::enums::*;
    use winreg::RegKey;

    log::info!("Trying to find the SteamID for the current user using the Windows registry.");
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    let Ok(steam_root) = hkcu.open_subkey(r"Software\Valve\Steam\ActiveProcess") else {
        log::warn!("Could not find the Steam registry key. Are you sure Steam is running?");
        return None;
    };

    let mut active_user: Option<String> = None;
    for k in steam_root.enum_values().flatten() {
        if k.0 == "ActiveUser" {
            active_user = Some(format!("{}", k.1));
            continue;
        }
    }

    if active_user.is_none() {
        log::warn!("Could not find the ActiveUser registry key. Are you sure Steam is running?");
        return None;
    }

    let active_user = active_user.unwrap();

    if active_user == "0" {
        log::warn!("Could not find the ActiveUser registry key. Are you sure Steam is running?");
        return None;
    }

    let s = format!("[U:1:{}]", active_user);
    Some(SteamID::from_steam_id32(&s))
}

/// Get the SteamID for the current user, if possible.
/// This is used to show the current user in the scoreboard.
#[cfg(target_os = "linux")]
pub fn get_current_user_steamid() -> Option<SteamID> {
    None
}
