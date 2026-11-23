use super::steamid::SteamID;
use crate::tf2bd::models::PlayerAttribute;
use crate::utils::BoxResult;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, ErrorKind, prelude::*};

const SETTINGS_FILENAME: &str = "settings.json";

fn get_true() -> bool {
    true
}

fn get_false() -> bool {
    false
}

fn default_party_notifications_for() -> Vec<PlayerAttribute> {
    vec![PlayerAttribute::Cheater, PlayerAttribute::Bot]
}

#[cfg(target_os = "windows")]
pub const DEFAULT_LOG_FILENAME: &str =
    "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Team Fortress 2\\tf\\console.log";

#[cfg(target_os = "linux")]
pub const DEFAULT_LOG_FILENAME: &str =
    "~/.local/share/Steam/steamapps/common/Team Fortress 2/tf/console.log";

pub const DEFAULT_LAUNCH_OPTIONS: &str = "-usercon -high +developer 1 +contimes 0 +ip 0.0.0.0 +net_start  +sv_rcon_whitelist_address 127.0.0.1 +rcon_password rconpwd +hostport 40434 +net_start +con_timestamp 1 -condebug -conclearlog -console -g15 -novid -nojoy -nosteamcontroller -nohltv -particles 1 -console";

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
    #[default]
    Fast,
    Beam,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationModel {
    #[default]
    Standard,
    Large,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranslationOptions {
    pub mode: TranslationMode,
    pub model: TranslationModel,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSettings {
    pub log_filename: String,

    pub self_steamid64: SteamID,

    /// Steam API key
    /// Used for fetching info about players from Steam
    /// Go here to crate a new key: https://steamcommunity.com/dev/apikey
    pub steam_api_key: String,

    /// TF2 RCON settings
    pub rcon_password: String,
    pub rcon_ip: String,
    pub rcon_port: u16,

    /// TF2 launch options
    #[serde(default)]
    pub launch_options: String,

    // View settings
    #[serde(default = "get_true")]
    pub show_friendship_indicators: bool,

    #[serde(default = "get_true")]
    pub show_crits: bool,

    #[serde(default = "get_false")]
    pub show_health: bool,

    #[serde(default = "get_true")]
    pub show_ping: bool,

    #[serde(default = "get_true")]
    pub show_weapons: bool,

    #[serde(default)]
    pub translate_chat: bool,
    #[serde(default)]
    pub chat_translation_mode: TranslationMode,
    #[serde(default)]
    pub chat_translation_model: TranslationModel,

    // Auto actions
    #[serde(default = "bool::default")]
    pub kick_cheaters: bool,

    #[serde(default = "get_true")]
    pub kick_bots: bool,

    #[serde(default = "default_party_notifications_for")]
    pub party_notifications_for: Vec<PlayerAttribute>,

    #[serde(default)]
    pub notify_tf2bd: bool,

    #[serde(default)]
    pub notify_sourcebans: bool,

    /// Flag indicating if configuration is complete and valid.
    /// Not saved to settings.json - computed at runtime.
    #[serde(skip)]
    pub config_is_ok: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        let self_steamid64 = get_current_user_steamid().unwrap_or(SteamID::from_u64(0));

        Self {
            log_filename: get_log_filename(),

            self_steamid64,

            steam_api_key: "".to_string(),

            rcon_password: "rconpwd".to_string(),
            rcon_ip: "127.0.0.1".to_string(),
            rcon_port: 40434,

            launch_options: DEFAULT_LAUNCH_OPTIONS.to_string(),

            show_friendship_indicators: true,
            show_crits: true,
            show_health: false,
            show_ping: false,
            show_weapons: true,
            translate_chat: false,
            chat_translation_mode: TranslationMode::default(),
            chat_translation_model: TranslationModel::default(),

            kick_cheaters: false,
            kick_bots: true,

            party_notifications_for: default_party_notifications_for(),
            notify_tf2bd: false,
            notify_sourcebans: false,

            config_is_ok: false,
        }
    }
}

impl AppSettings {
    pub fn chat_translation_options(&self) -> Option<TranslationOptions> {
        self.translate_chat.then_some(TranslationOptions {
            mode: self.chat_translation_mode,
            model: self.chat_translation_model,
        })
    }

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
        let mut settings: AppSettings = serde_json::from_str(&json).map_err(|e| {
            Box::new(io::Error::new(
                ErrorKind::InvalidData,
                format!(
                    "Failed to deserialize settings: {}. Please check your {} file.",
                    e, SETTINGS_FILENAME
                ),
            )) as Box<dyn std::error::Error>
        })?;

        log::info!("Settings loaded from file {}", SETTINGS_FILENAME);
        log::info!(
            "Settings: \n{}",
            serde_json::to_string_pretty(&settings).unwrap()
        );

        // Check if configuration is complete
        settings.config_is_ok = settings.check_config_is_ok();

        settings.save();

        Ok(settings)
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut f = File::create(SETTINGS_FILENAME).unwrap();
        f.write_all(json.as_bytes()).unwrap();

        log::info!("Settings saved to file {}", SETTINGS_FILENAME);
    }

    /// Checks if the configuration is complete enough to run the application.
    pub fn check_config_is_ok(&self) -> bool {
        // Must have valid SteamID
        if !self.self_steamid64.is_valid() {
            return false;
        }

        // Must have RCON password
        if self.rcon_password.trim().is_empty() {
            return false;
        }

        true
    }
}

//
// Platform specific functions
// - Try to find the SteadID for the current user
// - To get the default log filename
//

#[cfg(target_os = "windows")]
fn get_log_filename() -> String {
    r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf\console.log".to_string()
}

#[cfg(target_os = "linux")]
fn get_log_filename() -> String {
    DEFAULT_LOG_FILENAME.to_string()
}

/// Get the SteamID for the current user, if possible.
/// This is used to show the current user in the scoreboard.
#[cfg(target_os = "windows")]
pub fn get_current_user_steamid() -> Option<SteamID> {
    use winreg::RegKey;
    use winreg::enums::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_steam_api_key_is_optional_for_a_complete_configuration() {
        let mut settings = AppSettings {
            self_steamid64: SteamID::from_u64(crate::models::steamid::MIN_STEAMID64 + 1),
            steam_api_key: String::new(),
            ..AppSettings::default()
        };
        assert!(settings.check_config_is_ok());

        settings.rcon_password.clear();
        assert!(!settings.check_config_is_ok());
    }

    #[test]
    fn legacy_exe_filename_is_ignored_and_removed_on_save_without_changing_settings() {
        let mut settings = AppSettings::default();
        settings.rcon_port = 12345;
        settings.steam_api_key = "test-key".into();
        settings.launch_options = "custom launch options".into();
        let expected = serde_json::to_value(&settings).unwrap();
        assert!(expected.get("exe_filename").is_none());
        let mut legacy = expected.clone();
        legacy["exe_filename"] = serde_json::json!("C:\\Old Steam Folder\\tf_win64.exe");
        let loaded: AppSettings =
            serde_json::from_str(&serde_json::to_string(&legacy).unwrap()).unwrap();
        let saved = serde_json::to_value(&loaded).unwrap();
        assert_eq!(saved, expected);
    }
}
