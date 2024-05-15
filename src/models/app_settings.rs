use super::steamid::SteamID;
use crate::utils::BoxResult;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::prelude::*;

const SETTINGS_FILENAME: &str = "settings.json";

#[derive(Serialize, Deserialize, Debug)]
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
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            log_filename: get_log_filename(),
            exe_filename: get_exe_filename(),

            self_steamid64: SteamID::from_u64(0),

            // TODO: Remove this before publishing
            steam_api_key: "".to_string(),

            rcon_password: "rconpwd".to_string(),
            rcon_ip: "127.0.0.1".to_string(),
            rcon_port: 40434,
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
                println!("Error loading settings file: {}.", error);

                let settings = AppSettings::default();
                let json = serde_json::to_string_pretty(&settings).unwrap();
                println!("Using default values: {}.", json);

                settings.save();

                println!("To create a SteamAPI key, go to https://steamcommunity.com/dev/apikey");
                println!("The SteamAPI key is used to fetch info about players from Steam.");

                println!(
                    "Please edit the {} file and restart the application.",
                    SETTINGS_FILENAME
                );

                settings
            }
        }
    }

    pub fn load() -> BoxResult<AppSettings> {
        let mut f = File::open(SETTINGS_FILENAME)?;
        let mut json = String::new();
        f.read_to_string(&mut json)?;
        let preferences: AppSettings = serde_json::from_str(&json).unwrap();

        log::info!("Settings loaded from file {}", SETTINGS_FILENAME);
        log::info!("\n{}", json);

        Ok(preferences)
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        let mut f = File::create(SETTINGS_FILENAME).unwrap();
        f.write(json.as_bytes()).unwrap();

        println!("Settings saved to file {}", SETTINGS_FILENAME);
    }
}

#[cfg(target_os = "windows")]
fn get_log_filename() -> String {
    r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf\console.log".to_string()
}

#[cfg(target_os = "linux")]
fn get_log_filename() -> String {
    std::fs::canonicalize(r"~/.local/share/Steam/steamapps/common/Team Fortress 2/tf/console.log")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}

#[cfg(target_os = "windows")]
fn get_exe_filename() -> String {
    r"C:\Program Files (x86)\Steam\steamapps\common\Team Fortress 2\tf\console.log".to_string()
}

#[cfg(target_os = "linux")]
fn get_exe_filename() -> String {
    std::fs::canonicalize(r"~/.local/share/Steam/steamapps/common/Team Fortress 2/hl2_linux")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string()
}
