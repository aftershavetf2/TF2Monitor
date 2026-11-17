pub mod app_settings;
pub mod player_profile;
pub mod steamid;

use self::{app_settings::AppSettings, steamid::SteamID};
use crate::{
    appbus::{AppBus, AppEventMsg},
    db::connection::DbPool,
    gui::{overview_tab::OverviewState, tabs::Tab},
    tf2::lobby::{Lobby, shared_lobby::SharedLobby},
    tf2bd::models::PlayerAttribute,
};
use eframe::egui::Pos2;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

pub struct AppWin {
    pub bus: Arc<Mutex<AppBus>>,

    pub app_settings: AppSettings,
    pub active_tab: Tab,
    pub overview: OverviewState,

    pub profiles: std::cell::RefCell<HashMap<SteamID, player_profile::PlayerProfile>>,
    pub profile_events: bus::BusReader<player_profile::ProfileEvent>,
    pub profile_status: HashMap<SteamID, player_profile::FetchStatus>,
    pub profile_reputations: HashMap<SteamID, crate::reputation::Reputation>,
    pub match_snapshots: HashMap<SteamID, player_profile::MatchSnapshot>,
    pub lobby: Lobby,
    pub shared_lobby: SharedLobby,

    pub self_steamid: SteamID,
    pub selected_player: Option<SteamID>,
    pub open_player_details_windows: Vec<SteamID>,
    pub spectating: bool,

    // When drawing the scoreboard, we remember the center positions of each player's team indicator.
    // This is used to draw friendship indicators between players.
    pub friendship_positions: HashMap<SteamID, Pos2>,

    // Settings dialog state
    pub settings_window_open: bool,
    pub temp_settings: Option<TempSettings>,

    // Database statistics window state
    pub db_statistics_window_open: bool,
    pub player_database_window_open: bool,
    pub player_database_filters: HashSet<PlayerAttribute>,
    pub player_database_search: String,
    pub player_database_page: usize,

    // Database connection pool
    pub db: Arc<DbPool>,
}

/// Temporary settings used while editing in the settings dialog
#[derive(Clone)]
pub struct TempSettings {
    pub self_steamid64: String,
    pub steam_api_key: String,
    pub rcon_password: String,
    pub rcon_ip: String,
    pub rcon_port: String,
    pub log_filename: String,
    pub exe_filename: String,
    pub launch_options: String,
}

impl AppWin {
    pub fn new(settings: &AppSettings, bus: &Arc<Mutex<AppBus>>, db: Arc<DbPool>) -> Self {
        // Open settings window automatically if configuration is incomplete
        let settings_window_open = !settings.config_is_ok;

        let profile_events = bus.lock().unwrap().profile_events.add_rx();
        let shared_lobby = bus.lock().unwrap().shared_lobby.clone();
        Self {
            bus: Arc::clone(bus),

            app_settings: settings.clone(),
            active_tab: Tab::default(),
            overview: OverviewState::default(),

            profiles: Default::default(),
            profile_events,
            profile_status: HashMap::new(),
            profile_reputations: HashMap::new(),
            match_snapshots: HashMap::new(),
            lobby: Lobby::new(settings.self_steamid64),
            shared_lobby,
            self_steamid: settings.self_steamid64,
            selected_player: None,
            open_player_details_windows: Vec::new(),
            spectating: false,

            friendship_positions: HashMap::new(),

            settings_window_open,
            temp_settings: None,

            db_statistics_window_open: false,
            player_database_window_open: false,
            player_database_filters: HashSet::from([PlayerAttribute::Cheater]),
            player_database_search: String::new(),
            player_database_page: 0,
            db,
        }
    }

    pub fn player_profile(
        &self,
        steamid: SteamID,
    ) -> Result<player_profile::PlayerProfile, Box<dyn std::error::Error>> {
        if let Some(profile) = self.profiles.borrow().get(&steamid) {
            return Ok(profile.clone());
        }
        let mut conn = self.db.get()?;
        let profile = player_profile::PlayerProfile::load(&mut conn, steamid)?;
        self.profiles.borrow_mut().insert(steamid, profile.clone());
        Ok(profile)
    }

    pub fn updated_settings(&mut self) {
        log::info!("Saving and broadcasting updated settings");

        // Update config_is_ok flag based on current settings
        self.app_settings.config_is_ok = self.app_settings.check_config_is_ok();

        self.app_settings.save();
        self.bus
            .lock()
            .unwrap()
            .app_event_bus
            .broadcast(AppEventMsg::UpdatedSettings(self.app_settings.clone()));
    }

    pub fn get_latest_lobby(&mut self) {
        // Get a copy of the current lobby state
        let lobby = self.shared_lobby.get();
        if self.lobby.session_id != lobby.session_id {
            self.match_snapshots.clear();
        }
        for player in &lobby.players {
            self.match_snapshots.insert(
                player.steamid,
                player_profile::MatchSnapshot::new(lobby.session_id, player),
            );
        }
        self.lobby = lobby;
        while let Ok(event) = self.profile_events.try_recv() {
            match event {
                player_profile::ProfileEvent::Changed(id) => {
                    self.profiles.borrow_mut().remove(&id);
                }
                player_profile::ProfileEvent::FetchStatus(id, status) => {
                    self.profile_status.insert(id, status);
                }
                player_profile::ProfileEvent::Reputation(value) => {
                    self.profile_reputations.insert(value.steamid, value);
                }
            }
        }
        let mut bus = self.bus.lock().unwrap();
        bus.profile_requests = self
            .open_player_details_windows
            .iter()
            .copied()
            .chain(self.selected_player)
            .collect();
    }

    /// The selected player is who is shown in the player details panel.
    pub fn set_selected_player(&mut self, clicked_on_steamid: SteamID) {
        self.selected_player = Some(clicked_on_steamid);
    }

    pub fn open_player_details_window(&mut self, steamid: SteamID) {
        if !self.open_player_details_windows.contains(&steamid) {
            self.open_player_details_windows.push(steamid);
        }
    }

    pub fn is_me(&self, steamid: Option<SteamID>) -> bool {
        match steamid {
            Some(steamid) => steamid == self.self_steamid,
            None => false,
        }
    }
}
